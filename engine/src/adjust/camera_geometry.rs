//! Camera Raw's independently measured Core Image perspective arithmetic.
//! Corners/differences/products are f32, ratios/coefficients/inversion are f64,
//! and the sampling kernel receives f32 rows. See the public fixture notes.
use crate::Point;

const RECIPROCAL_DELTA: &[u8; 1 << 21] = include_bytes!("camera_geometry_reciprocal.bin");

/// Reproduce the measured reciprocal rounding over the verified positive
/// exponent range. Outside it, retain IEEE division rather than extrapolating
/// an unmeasured sign, exponent, subnormal or overflow rule.
fn reciprocal(value: f32) -> f32 {
    let rounded = 1.0 / value;
    if !(0.5..2.0).contains(&value) {
        return rounded;
    }
    let mantissa = value.to_bits() & 0x7f_ffff;
    let code = (RECIPROCAL_DELTA[(mantissa >> 2) as usize] >> ((mantissa & 3) * 2)) & 3;
    let bits = match code {
        1 => rounded.to_bits() + 1,
        2 => rounded.to_bits() - 1,
        _ => rounded.to_bits(),
    };
    f32::from_bits(bits)
}

pub(crate) struct Perspective([[f32; 3]; 3]);

impl Perspective {
    pub(crate) fn from_corners(corners: &[Point; 4], width: u32, height: u32) -> Option<Self> {
        // Input and output are y-up; square order is bottom-left, bottom-right,
        // top-right, top-left. Each f32 operation is intentional.
        let [a, b, c, d] = [3, 2, 1, 0].map(|i| [corners[i].x as f32, corners[i].y as f32]);
        let [sx, sy] = [0, 1].map(|i| ((a[i] - b[i]) + c[i]) - d[i]);
        let [dx1, dy1] = [b[0] - c[0], b[1] - c[1]];
        let [dx2, dy2] = [d[0] - c[0], d[1] - c[1]];
        let denominator = dx1 * dy2 - dx2 * dy1;
        if denominator == 0.0 || !denominator.is_finite() {
            return None;
        }
        let g = (-dx2).mul_add(sy, sx * dy2) as f64 / denominator as f64;
        let h = (-sx).mul_add(dy1, dx1 * sy) as f64 / denominator as f64;
        let coefficient = |v: f32, origin: f32, t: f64| t.mul_add(v as f64, (v - origin) as f64);
        let forward = [
            [
                coefficient(b[0], a[0], g),
                coefficient(d[0], a[0], h),
                a[0] as f64,
            ],
            [
                coefficient(b[1], a[1], g),
                coefficient(d[1], a[1], h),
                a[1] as f64,
            ],
            [g, h, 1.0],
        ];
        let mut cofactor = [[0.0; 3]; 3];
        for (i, row) in cofactor.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                let rows: Vec<_> = (0..3).filter(|v| *v != i).collect();
                let cols: Vec<_> = (0..3).filter(|v| *v != j).collect();
                let [r, s] = [rows[0], rows[1]];
                let [u, v] = [cols[0], cols[1]];
                let [p, q, x, y] = [forward[r][u], forward[s][v], forward[r][v], forward[s][u]];
                *cell = if (i + j) % 2 == 0 {
                    p.mul_add(q, -(x * y))
                } else {
                    x.mul_add(y, -(p * q))
                };
            }
        }
        let determinant = (forward[0][0] * cofactor[0][0] + forward[0][1] * cofactor[0][1])
            + forward[0][2] * cofactor[0][2];
        if determinant == 0.0 || !determinant.is_finite() {
            return None;
        }
        let scale = [width as f64, height as f64, 1.0];
        let rows = std::array::from_fn(|i| {
            std::array::from_fn(|j| (cofactor[j][i] / determinant * scale[i]) as f32)
        });
        if rows.iter().flatten().any(|v| !v.is_finite()) {
            return None;
        }
        Some(Self(rows))
    }

    pub(crate) fn coordinate(&self, x: u32, y: u32, height: u32) -> (f64, f64) {
        self.coordinate_in_extent(x, y, height, height)
    }

    pub(crate) fn coordinate_in_extent(&self, x: u32, y: u32, output_height: u32, source_height: u32) -> (f64, f64) {
        let height = output_height;
        let px = x as f32 + 0.5;
        let py = height as f32 - (y as f32 + 0.5);
        let [qx, qy, qz] = self.0.map(|r| r[1].mul_add(py, r[0] * px) + r[2]);
        let r = reciprocal(qz);
        ((qx * r) as f64, (source_height as f32 - qy * r) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    #[derive(Deserialize)]
    struct Case {
        name: String,
        width: u32,
        height: u32,
        #[serde(rename = "cornersYUp")]
        corners: [[f64; 2]; 4],
        #[serde(rename = "floatBits")]
        bits: [[String; 3]; 3],
    }
    #[test]
    fn matches_actual_public_kernel_rows_for_fifteen_independent_quadrilaterals() {
        let cases: Vec<Case> = serde_json::from_str(include_str!(
            "../../tests/fixtures/core-image-perspective-matrices.json"
        ))
        .unwrap();
        assert_eq!(cases.len(), 15);
        for case in cases {
            let corners = case.corners.map(|[x, y]| Point { x, y });
            let p = Perspective::from_corners(&corners, case.width, case.height).unwrap();
            for i in 0..3 {
                for j in 0..3 {
                    let expected =
                        f32::from_bits(u32::from_str_radix(&case.bits[i][j], 16).unwrap());
                    // Preserve the observed signed-zero bits in the fixture; both
                    // zero signs have the same pixel-coordinate numeric value.
                    assert_eq!(p.0[i][j], expected, "{} row={} column={}", case.name, i, j);
                }
            }
        }
    }
    #[test]
    fn complete_normalized_reciprocal_domain_matches_independent_mac_measurement() {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for mantissa in 0..(1 << 23) {
            let value = f32::from_bits(0x3f80_0000 + mantissa);
            let bits = reciprocal(value).to_bits();
            hash = (hash ^ bits as u64).wrapping_mul(0x100_0000_01b3);
        }
        // Independent complete-domain word hash: all 8,388,608 f32 mantissas.
        assert_eq!(hash, 0x3f9d_ef28_0998_14da);
    }
    #[test]
    fn unmeasured_reciprocal_inputs_keep_ieee_division() {
        for v in [
            -4.0f32,
            -0.5,
            -0.0,
            0.0,
            f32::MIN_POSITIVE,
            0.25,
            2.0,
            8.0,
            f32::INFINITY,
        ] {
            assert_eq!(reciprocal(v).to_bits(), (1.0 / v).to_bits());
        }
        assert!(reciprocal(f32::NAN).is_nan());
    }
}

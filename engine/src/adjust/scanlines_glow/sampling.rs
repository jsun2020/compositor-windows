//! The positive half texture sampler observed in the Mac format-matrix probe.
//! Multiplication exponents are added before mantissa normalization. Aligning
//! to the exponent of the already-normalized product is observably different.
use super::{half_float, half_up};

/// Four nonnegative finite half texels, in TL/TR/BL/BR order; Q8 fractions.
pub(crate) fn quad_half(q: [[u16; 4]; 4], x: u16, y: u16) -> [u16; 4] {
    debug_assert!(x <= 256 && y <= 256 && q.iter().flatten().all(|&v| v < 0x7c00));
    let (x, y) = (u32::from(x), u32::from(y));
    let weights = [(256 - x) * (256 - y), x * (256 - y), (256 - x) * y, x * y];
    std::array::from_fn(|c| {
        let mut mantissas = [0_u32; 4];
        let mut powers = [0_i32; 4];
        let mut exponent = i32::MIN;
        for i in 0..4 {
            if weights[i] == 0 || q[i][c] == 0 {
                continue;
            }
            let he = i32::from(q[i][c] >> 10);
            let mantissa = u32::from(q[i][c] & 1023) + if he == 0 { 0 } else { 1024 };
            let power = if he == 0 { -24 } else { he - 25 };
            mantissas[i] = mantissa;
            powers[i] = power;
            let input_exponent = power + (31 - mantissa.leading_zeros()) as i32;
            let weight_exponent = (31 - weights[i].leading_zeros()) as i32 - 16;
            exponent = exponent.max(input_exponent + weight_exponent);
        }
        if exponent == i32::MIN {
            return 0;
        }
        let mut sum = 0_u64;
        for i in 0..4 {
            if mantissas[i] == 0 {
                continue;
            }
            let product = u64::from(mantissas[i]) * u64::from(weights[i]);
            let shift = exponent - powers[i] - 1;
            sum += if shift >= 0 {
                product >> shift
            } else {
                product << -shift
            };
        }
        // At most four 19-bit aligned products: the sum is exactly binary32.
        // The grid exponent is in -57..=-2 for finite positive half inputs.
        let step = f32::from_bits(((exponent + 110) as u32) << 23);
        half_up(f64::from(sum as f32 * step))
    })
}

pub(super) fn quad_float(q: [[u16; 4]; 4], x: u16, y: u16) -> [f32; 4] {
    quad_half(q, x, y).map(half_float)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_mac_pre_normalization_witness() {
        // Actual independent sampler sample12640, including a blue component
        // for which ideal bilinear then half rounding returns one ULP too high.
        let q = [
            [13089, 13089, 13071, 15359],
            [13098, 13100, 13082, 15359],
            [13283, 13302, 13273, 15359],
            [13290, 13309, 13282, 15359],
        ];
        assert_eq!(quad_half(q, 85, 85), [13156, 13163, 13141, 15359]);
    }
    #[test]
    fn corners_zero_and_subnormal_samples() {
        let q = [
            [1, 2, 3, 4],
            [7, 9, 11, 13],
            [1023, 1024, 1025, 1026],
            [15359, 15360, 15361, 15362],
        ];
        for (x, y, i) in [(0, 0, 0), (256, 0, 1), (0, 256, 2), (256, 256, 3)] {
            assert_eq!(quad_half(q, x, y), q[i]);
        }
        assert_eq!(quad_half([[0; 4]; 4], 127, 31), [0; 4]);
        assert_eq!(quad_half([[1; 4], [0; 4], [0; 4], [0; 4]], 128, 0), [1; 4]);
    }
}

//! Bounded CPU reference for Compositor v1.4.5's Smudge and Metal Liquify.
//!
//! Coordinates are local pixels of one premultiplied RGBA plane. This is an
//! algorithm oracle, not a document command or the large-document GPU path.
//! Selection, transforms, history and preview ownership belong to the caller.

pub const REFERENCE_MAX_PIXELS: usize = 4_194_304;
const MAX_DIAMETER: f64 = 2000.0;
const MAX_STEPS: usize = 4096;
const MAX_DAB_SAMPLES: usize = 16_777_216;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WarpMode {
    Smudge,
    Liquify,
}

#[derive(Clone, Copy, Debug)]
pub struct WarpSettings {
    pub diameter: f64,
    pub hardness: f32,
    pub strength: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarpError {
    InvalidPlane,
    InvalidSettings,
    InvalidPoint,
    MovementTooLong,
}

/// A stroke owns its source snapshot and retains fractional colors/offsets.
/// Reference limits: 4 Mi pixels, diameter 2000, 4096 dabs and 16 Mi brush
/// samples per append. This deliberately refuses oversized synchronous work;
/// callers must not use it as the 24/100 MP interactive preview implementation.
pub struct WarpStroke {
    width: usize,
    height: usize,
    mode: WarpMode,
    settings: WarpSettings,
    radius: i32,
    original: Vec<u8>,
    pixels: Vec<u8>,
    offsets: Vec<[f32; 2]>,
    carried: Vec<[f32; 4]>,
    last: Option<[f64; 2]>,
}

impl WarpStroke {
    pub fn new(
        width: usize,
        height: usize,
        pixels: &[u8],
        mode: WarpMode,
        mut settings: WarpSettings,
    ) -> Result<Self, WarpError> {
        let count = width.checked_mul(height).ok_or(WarpError::InvalidPlane)?;
        if count == 0
            || count > REFERENCE_MAX_PIXELS
            || count.checked_mul(4) != Some(pixels.len())
            || pixels
                .chunks_exact(4)
                .any(|p| p[..3].iter().any(|v| *v > p[3]))
        {
            return Err(WarpError::InvalidPlane);
        }
        if !settings.diameter.is_finite()
            || !settings.hardness.is_finite()
            || !settings.strength.is_finite()
            || settings.diameter > MAX_DIAMETER
        {
            return Err(WarpError::InvalidSettings);
        }
        settings.diameter = settings.diameter.max(2.0);
        settings.hardness = settings.hardness.clamp(0.0, 0.98);
        settings.strength = settings.strength.clamp(0.01, 1.0);
        Ok(Self {
            width,
            height,
            mode,
            settings,
            radius: (settings.diameter * 0.5).ceil() as i32,
            original: if mode == WarpMode::Liquify {
                pixels.to_vec()
            } else {
                Vec::new()
            },
            pixels: pixels.to_vec(),
            offsets: if mode == WarpMode::Liquify {
                vec![[0.0; 2]; count]
            } else {
                Vec::new()
            },
            carried: Vec::new(),
            last: None,
        })
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Returns scheduled dabs, not a claim that the pixel bytes changed.
    /// Rejected movement leaves both pixels and the previous anchor intact.
    pub fn append(&mut self, point: [f64; 2]) -> Result<usize, WarpError> {
        self.append_dabs(point).map(|dabs| dabs.len())
    }

    /// Actual scheduled centers, for the final replacement footprint. Pickup
    /// and sub-spacing moves return no centers; callers must not paint them.
    pub fn append_dabs(&mut self, point: [f64; 2]) -> Result<Vec<[f64; 2]>, WarpError> {
        if point
            .iter()
            .any(|p| !p.is_finite() || p.abs() > 1_000_000.0)
        {
            return Err(WarpError::InvalidPoint);
        }
        let Some(last) = self.last else {
            if self.mode == WarpMode::Smudge {
                self.pickup(point);
            }
            self.last = Some(point);
            return Ok(Vec::new());
        };
        let delta = [point[0] - last[0], point[1] - last[1]];
        let distance = delta[0].hypot(delta[1]);
        let spacing = (self.settings.diameter
            * match self.mode {
                WarpMode::Smudge => 0.005,
                WarpMode::Liquify => 0.025,
            })
        .max(1.0);
        if distance < spacing {
            return Ok(Vec::new());
        }
        let steps = (distance / spacing).ceil() as usize;
        let side = (2 * self.radius + 1) as usize;
        let work = steps.checked_mul(side).and_then(|n| n.checked_mul(side));
        if steps > MAX_STEPS || work.is_none_or(|n| n > MAX_DAB_SAMPLES) {
            return Err(WarpError::MovementTooLong);
        }
        let mut previous = last;
        let mut dabs = Vec::with_capacity(steps);
        for step in 1..=steps {
            let t = step as f64 / steps as f64;
            let current = [last[0] + delta[0] * t, last[1] + delta[1] * t];
            match self.mode {
                WarpMode::Smudge => self.smudge(current),
                WarpMode::Liquify => self.push(previous, current),
            }
            previous = current;
            dabs.push(current);
        }
        self.last = Some(point);
        Ok(dabs)
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height)
            .then(|| y as usize * self.width + x as usize)
    }

    fn center(point: [f64; 2]) -> [i32; 2] {
        [point[0].round() as i32, point[1].round() as i32]
    }

    fn weight(&self, dx: i32, dy: i32) -> f32 {
        let u = ((dx * dx + dy * dy) as f32).sqrt() * (1.0 / (self.settings.diameter * 0.5) as f32);
        if u >= 1.0 {
            return 0.0;
        }
        if u <= self.settings.hardness {
            return 1.0;
        }
        let t = (1.0 - u) / (1.0 - self.settings.hardness);
        t * t * (3.0 - 2.0 * t)
    }

    fn pickup(&mut self, point: [f64; 2]) {
        let [cx, cy] = Self::center(point);
        let side = (2 * self.radius + 1) as usize;
        self.carried = vec![[0.0; 4]; side * side];
        for dy in -self.radius..=self.radius {
            for dx in -self.radius..=self.radius {
                if let Some(i) = self.index(cx + dx, cy + dy) {
                    let c = (dy + self.radius) as usize * side + (dx + self.radius) as usize;
                    self.carried[c] = std::array::from_fn(|k| self.pixels[i * 4 + k] as f32);
                }
            }
        }
    }

    fn smudge(&mut self, point: [f64; 2]) {
        let [cx, cy] = Self::center(point);
        let side = (2 * self.radius + 1) as usize;
        for dy in -self.radius..=self.radius {
            for dx in -self.radius..=self.radius {
                let weight = self.weight(dx, dy);
                if weight <= 0.0 {
                    continue;
                }
                let Some(i) = self.index(cx + dx, cy + dy) else {
                    continue;
                };
                let c = (dy + self.radius) as usize * side + (dx + self.radius) as usize;
                for k in 0..4 {
                    let under = self.pixels[i * 4 + k] as f32;
                    let painted =
                        under + (self.carried[c][k] - under) * weight * self.settings.strength;
                    self.pixels[i * 4 + k] = painted.round().clamp(0.0, 255.0) as u8;
                    self.carried[c][k] = painted;
                }
            }
        }
    }

    fn push(&mut self, from: [f64; 2], to: [f64; 2]) {
        let [cx, cy] = Self::center(to);
        let movement = [
            (to[0] - from[0]) as f32 * self.settings.strength,
            (to[1] - from[1]) as f32 * self.settings.strength,
        ];
        let margin = movement[0].abs().max(movement[1].abs()).ceil() as i32 + 2;
        let left = (cx - self.radius - margin).clamp(0, self.width as i32);
        let top = (cy - self.radius - margin).clamp(0, self.height as i32);
        let right = (cx + self.radius + margin + 1).clamp(0, self.width as i32);
        let bottom = (cy + self.radius + margin + 1).clamp(0, self.height as i32);
        let width = (right - left) as usize;
        let height = (bottom - top) as usize;
        if width == 0 || height == 0 {
            return;
        }
        // Freeze only the dependency rectangle, so a dab cannot read its writes.
        let mut scratch = Vec::with_capacity(width * height);
        for y in top..bottom {
            let start = y as usize * self.width + left as usize;
            scratch.extend_from_slice(&self.offsets[start..start + width]);
        }
        for dy in -self.radius..=self.radius {
            for dx in -self.radius..=self.radius {
                let weight = self.weight(dx, dy);
                if weight <= 0.0 {
                    continue;
                }
                let x = cx + dx;
                let y = cy + dy;
                let Some(i) = self.index(x, y) else {
                    continue;
                };
                let sampled: [f32; 2] = bilinear(
                    width,
                    height,
                    x as f32 - left as f32 - movement[0] * weight,
                    y as f32 - top as f32 - movement[1] * weight,
                    |p, k| scratch[p][k],
                );
                let offset = [
                    sampled[0] - movement[0] * weight,
                    sampled[1] - movement[1] * weight,
                ];
                self.offsets[i] = offset;
                let color: [f32; 4] = bilinear(
                    self.width,
                    self.height,
                    x as f32 + offset[0],
                    y as f32 + offset[1],
                    |p, k| self.original[p * 4 + k] as f32,
                );
                for (k, value) in color.into_iter().enumerate() {
                    self.pixels[i * 4 + k] = value.round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

/// Clamped bilinear reads define one-pixel axes (the Metal dab guard skips them).
fn bilinear<const N: usize>(
    width: usize,
    height: usize,
    x: f32,
    y: f32,
    value: impl Fn(usize, usize) -> f32,
) -> [f32; N] {
    let x = x.clamp(0.0, (width - 1) as f32);
    let y = y.clamp(0.0, (height - 1) as f32);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    std::array::from_fn(|k| {
        let a = value(y0 * width + x0, k);
        let b = value(y0 * width + x1, k);
        let c = value(y1 * width + x0, k);
        let d = value(y1 * width + x1, k);
        let upper = a + (b - a) * fx;
        let lower = c + (d - c) * fx;
        upper + (lower - upper) * fy
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(strength: f32) -> WarpSettings {
        WarpSettings {
            diameter: 2.0,
            hardness: 0.5,
            strength,
        }
    }
    fn opaque_row(values: &[u8]) -> Vec<u8> {
        values.iter().flat_map(|v| [*v, *v, *v, 255]).collect()
    }

    #[test]
    fn first_click_and_sub_spacing_keep_pixels_and_anchor() {
        for mode in [WarpMode::Smudge, WarpMode::Liquify] {
            let pixels = opaque_row(&[255, 0, 0, 0]);
            let mut stroke = WarpStroke::new(4, 1, &pixels, mode, settings(0.5)).unwrap();
            assert_eq!(stroke.append([0.0, 0.0]), Ok(0));
            assert_eq!(stroke.append([0.6, 0.0]), Ok(0));
            assert_eq!(stroke.pixels(), pixels);
            assert_eq!(stroke.last, Some([0.0, 0.0]));
            assert_eq!(stroke.append([1.1, 0.0]), Ok(2));
        }
    }

    #[test]
    fn smudge_is_one_fading_trail_with_fractional_carry() {
        let mut stroke = WarpStroke::new(
            7,
            1,
            &opaque_row(&[255, 0, 0, 0, 0, 0, 0]),
            WarpMode::Smudge,
            settings(0.5),
        )
        .unwrap();
        stroke.append([0.0, 0.0]).unwrap();
        stroke.append([6.0, 0.0]).unwrap();
        let row: Vec<_> = stroke.pixels().chunks_exact(4).map(|p| p[0]).collect();
        assert_eq!(row, [255, 128, 64, 32, 16, 8, 4]);
        assert_eq!(stroke.carried[4][0], 255.0 / 64.0);
    }

    #[test]
    fn low_strength_does_not_retain_initial_pickup() {
        let mut stroke = WarpStroke::new(
            4,
            1,
            &opaque_row(&[255, 0, 0, 0]),
            WarpMode::Smudge,
            settings(0.25),
        )
        .unwrap();
        stroke.append([0.0, 0.0]).unwrap();
        stroke.append([3.0, 0.0]).unwrap();
        assert_eq!(stroke.pixels(), opaque_row(&[255, 64, 16, 4]));
        assert_eq!(stroke.carried[4][0], 255.0 / 64.0);
    }

    #[test]
    fn liquify_advects_offsets_and_reads_immutable_source() {
        let original = opaque_row(&[0, 0, 255, 0, 0]).repeat(3);
        let mut stroke =
            WarpStroke::new(5, 3, &original, WarpMode::Liquify, settings(1.0)).unwrap();
        stroke.push([2.0, 1.0], [2.25, 1.0]);
        assert_eq!(stroke.offsets[7], [-0.25, 0.0]);
        assert_eq!(stroke.pixels()[28], 191);
        stroke.push([2.0, 1.0], [2.25, 1.0]);
        assert_eq!(stroke.offsets[7], [-0.4375, 0.0]);
        assert_eq!(stroke.pixels()[28], 143);
        assert_eq!(stroke.original, original);
    }

    #[test]
    fn constant_planes_and_thin_axes_are_safe() {
        for (width, height) in [(1, 1), (1, 9), (9, 1), (8, 8)] {
            for mode in [WarpMode::Smudge, WarpMode::Liquify] {
                let pixels = [80, 40, 10, 100].repeat(width * height);
                let mut stroke =
                    WarpStroke::new(width, height, &pixels, mode, settings(1.0)).unwrap();
                stroke.append([0.0, 0.0]).unwrap();
                stroke
                    .append([width as f64 - 1.0, height as f64 - 1.0])
                    .unwrap();
                assert_eq!(stroke.pixels(), pixels);
                assert!(stroke.offsets.iter().flatten().all(|v| v.is_finite()));
            }
        }
    }

    #[test]
    fn transparent_pickup_and_negative_half_center() {
        let mut stroke = WarpStroke::new(
            3,
            1,
            &opaque_row(&[255, 255, 255]),
            WarpMode::Smudge,
            settings(1.0),
        )
        .unwrap();
        stroke.append([-0.5, 0.0]).unwrap();
        assert_eq!(stroke.carried[4], [0.0; 4]);
        stroke.append([1.0, 0.0]).unwrap();
        assert_eq!(
            stroke.pixels(),
            &[0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255]
        );
    }

    #[test]
    fn invalid_inputs_and_excessive_movement_are_atomic() {
        for (w, h, pixels) in [
            (0, 1, vec![]),
            (usize::MAX, 2, vec![]),
            (1, 1, vec![255, 0, 0, 0]),
        ] {
            assert!(matches!(
                WarpStroke::new(w, h, &pixels, WarpMode::Liquify, settings(1.0)),
                Err(WarpError::InvalidPlane)
            ));
        }
        let mut bad = settings(1.0);
        bad.hardness = f32::NAN;
        assert!(matches!(
            WarpStroke::new(1, 1, &[0; 4], WarpMode::Smudge, bad),
            Err(WarpError::InvalidSettings)
        ));
        let mut stroke = WarpStroke::new(
            3,
            1,
            &opaque_row(&[255, 0, 0]),
            WarpMode::Smudge,
            settings(0.5),
        )
        .unwrap();
        stroke.append([0.0, 0.0]).unwrap();
        let before = stroke.pixels().to_vec();
        assert_eq!(
            stroke.append([f64::INFINITY, 0.0]),
            Err(WarpError::InvalidPoint)
        );
        assert_eq!(
            stroke.append([5000.0, 0.0]),
            Err(WarpError::MovementTooLong)
        );
        assert_eq!(stroke.pixels(), before);
        assert_eq!(stroke.last, Some([0.0, 0.0]));
        let mut options = settings(1.0);
        options.diameter = 2000.0;
        let mut large_brush = WarpStroke::new(1, 1, &[0; 4], WarpMode::Liquify, options).unwrap();
        large_brush.append([0.0, 0.0]).unwrap();
        assert_eq!(
            large_brush.append([500.0, 0.0]),
            Err(WarpError::MovementTooLong)
        );
        assert_eq!(large_brush.last, Some([0.0, 0.0]));
    }

    #[test]
    fn modes_use_their_distinct_mac_spacing() {
        let pixels = opaque_row(&[0; 20]);
        for (mode, dabs) in [(WarpMode::Smudge, 3), (WarpMode::Liquify, 0)] {
            let mut options = settings(0.5);
            options.diameter = 200.0;
            let mut stroke = WarpStroke::new(20, 1, &pixels, mode, options).unwrap();
            stroke.append([0.0, 0.0]).unwrap();
            assert_eq!(stroke.append([3.0, 0.0]), Ok(dabs));
        }
    }

    #[test]
    fn mixed_alpha_remains_premultiplied_near_edges() {
        let pixels = vec![
            120, 60, 0, 128, 0, 0, 0, 0, 30, 20, 10, 40, 255, 255, 255, 255,
        ];
        for mode in [WarpMode::Smudge, WarpMode::Liquify] {
            let mut options = settings(0.7);
            options.diameter = 6.0;
            let mut stroke = WarpStroke::new(4, 1, &pixels, mode, options).unwrap();
            stroke.append([-1.0, 0.0]).unwrap();
            stroke.append([5.0, 0.0]).unwrap();
            stroke.append([0.0, 0.0]).unwrap();
            assert!(stroke
                .pixels()
                .chunks_exact(4)
                .all(|p| p[..3].iter().all(|v| *v <= p[3])));
        }
    }

    #[test]
    fn fractional_diameter_uses_actual_radius_for_falloff() {
        let mut options = settings(1.0);
        options.diameter = 3.0;
        options.hardness = 0.0;
        let stroke = WarpStroke::new(1, 1, &[0; 4], WarpMode::Smudge, options).unwrap();
        assert_eq!(stroke.radius, 2);
        // At distance 1, u=2/3 and smoothstep(1/3)=7/27, not 1/2.
        assert!((stroke.weight(1, 0) - 7.0 / 27.0).abs() < 0.000001);
        assert_eq!(stroke.weight(2, 0), 0.0);
    }

    #[test]
    fn liquify_dab_reads_one_snapshot_independent_of_pixel_order() {
        let original = opaque_row(&[0, 20, 40, 60, 80]).repeat(5);
        let mut options = settings(1.0);
        options.diameter = 4.0;
        options.hardness = 0.98;
        let mut stroke = WarpStroke::new(5, 5, &original, WarpMode::Liquify, options).unwrap();
        // An analytic linear field: sampling a quarter-pixel to the left gives
        // x-1/4, then subtracting movement gives x-1/2. Reading earlier writes
        // would change the next pixel's result, even on this simple field.
        for y in 0..5 {
            for x in 0..5 {
                stroke.offsets[y * 5 + x] = [x as f32, y as f32];
            }
        }
        stroke.push([2.0, 2.0], [2.25, 2.0]);
        assert_eq!(stroke.offsets[12], [1.5, 2.0]);
        assert_eq!(stroke.offsets[13], [2.5, 2.0]);
        assert_eq!(stroke.pixels()[48], 70);
        assert_eq!(stroke.pixels()[52], 80);
    }

    #[test]
    fn mac_bright_circle_fixture_has_no_repeated_smudge_peaks() {
        let mut pixels = [26, 26, 26, 255].repeat(300 * 100);
        for y in 42i32..58 {
            for x in 52i32..68 {
                if (x - 60).pow(2) + (y - 50).pow(2) <= 64 {
                    let i = (y as usize * 300 + x as usize) * 4;
                    pixels[i..i + 4].copy_from_slice(&[255; 4]);
                }
            }
        }
        let options = WarpSettings {
            diameter: 40.0,
            hardness: 0.5,
            strength: 0.6,
        };
        let mut stroke = WarpStroke::new(300, 100, &pixels, WarpMode::Smudge, options).unwrap();
        for x in (60..=240).step_by(3) {
            stroke.append([x as f64, 50.0]).unwrap();
        }
        let row: Vec<i32> = (70..=240)
            .map(|x| stroke.pixels()[(50 * 300 + x) * 4] as i32)
            .collect();
        let peaks = (2..row.len() - 2)
            .filter(|&i| row[i] > row[i - 2] + 2 && row[i] > row[i + 2] + 2)
            .count();
        assert_eq!(peaks, 0);
        assert!(row[0] > row[row.len() - 1] + 20);
    }
}

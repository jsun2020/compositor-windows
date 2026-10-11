//! Independent whole-plane transcription of fixed Mac 1.4.9 MetalLayerEffects.swift.
//! Source: 0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd; no production renderer calls.
    use compositor_engine::EffectPasses;

    fn alpha(pixels: &[u8], w: usize, h: usize, inset: usize) -> (Vec<f32>, usize, usize) {
        let (pw, ph) = (w + 2 * inset, h + 2 * inset);
        let mut a = vec![0.0f32; pw * ph];
        for y in 0..h { for x in 0..w { a[(y + inset) * pw + x + inset] = pixels[(y * w + x) * 4 + 3] as f32 / 255.0; } }
        (a, pw, ph)
    }
    fn spread(src: &[f32], w: usize, h: usize, reach: usize, smallest: bool) -> Vec<f32> {
        let reach = reach as i64;
        let pass = |src: &[f32], along_rows: bool| {
            let mut out = vec![0.0f32; w * h];
            for y in 0..h { for x in 0..w {
                let mut best = if smallest { 1.0f32 } else { 0.0 };
                for o in -reach..=reach {
                    let (sx, sy) = if along_rows { (x as i64 + o, y as i64) } else { (x as i64, y as i64 + o) };
                    let v = if sx < 0 || sy < 0 || sx >= w as i64 || sy >= h as i64 { 0.0 } else { src[sy as usize * w + sx as usize] };
                    best = if smallest { best.min(v) } else { best.max(v) };
                }
                out[y * w + x] = best;
            }}
            out
        };
        pass(&pass(src, true), false)
    }
    fn shift(src: &[f32], w: usize, h: usize, dx: f32, dy: f32) -> Vec<f32> {
        let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let mut out = vec![0.0f32; w * h];
        for y in 0..h { for x in 0..w {
            let (sx, sy) = (x as f32 - dx, y as f32 - dy);
            if sx >= 0.0 && sy >= 0.0 && sx <= (w - 1) as f32 && sy <= (h - 1) as f32 {
                let (x0, y0) = (sx.floor() as usize, sy.floor() as usize);
                let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
                let (fx, fy) = (sx - x0 as f32, sy - y0 as f32);
                let top = mix(src[y0 * w + x0], src[y0 * w + x1], fx);
                let bottom = mix(src[y1 * w + x0], src[y1 * w + x1], fx);
                out[y * w + x] = mix(top, bottom, fy);
            }
        }}
        out
    }
    pub fn blur(src: &[f32], w: usize, h: usize, sigma: f32) -> Vec<f32> {
        let radius = ((sigma * 3.0).round() as i64).max(1);
        let pass = |src: &[f32], along_rows: bool| {
            let mut out = vec![0.0f32; w * h];
            for y in 0..h { for x in 0..w {
                let (mut total, mut sum) = (0.0f32, 0.0f32);
                for o in -radius..=radius {
                    let weight = (-((o * o) as f32) / (2.0 * sigma * sigma)).exp();
                    let s = if along_rows { y * w + (x as i64 + o).clamp(0, w as i64 - 1) as usize } else { (y as i64 + o).clamp(0, h as i64 - 1) as usize * w + x };
                    total += weight * src[s];
                    sum += weight;
                }
                out[y * w + x] = total / sum;
            }}
            out
        };
        pass(&pass(src, true), false)
    }
    /// `effects_shift` then, above sigma 0.01, the two blur passes (a zero move is the shape itself).
    fn soften(shape: &[f32], w: usize, h: usize, dx: f32, dy: f32, sigma: f32) -> Vec<f32> {
        let moved = if dx == 0.0 && dy == 0.0 { shape.to_vec() } else { shift(shape, w, h, dx, dy) };
        if sigma > 0.01 { blur(&moved, w, h, sigma) } else { moved }
    }
    pub fn render(pixels: &[u8], w: usize, h: usize, inset: usize, p: &EffectPasses) -> Vec<u8> {
        let (shape, pw, ph) = alpha(pixels, w, h, inset);
        let n = pw * ph;
        let inside_of = |m: Vec<f32>| (0..n).map(|i| (1.0 - m[i]).clamp(0.0, 1.0)).collect::<Vec<f32>>();
        let ring = p.stroke.map(|s| { let m = spread(&shape, pw, ph, s.reach, s.inside);
            (0..n).map(|i| if s.inside { shape[i] - m[i] } else { m[i] - shape[i] }.clamp(0.0, 1.0)).collect::<Vec<f32>>() });
        let shadow = p.shadow.map(|s| soften(&shape, pw, ph, s.dx, s.dy, s.sigma));
        let inner = p.inner_shadow.map(|s| inside_of(soften(&shape, pw, ph, s.dx, s.dy, s.sigma)));
        let glow = p.outer_glow.map(|g| soften(&shape, pw, ph, 0.0, 0.0, g.sigma));
        let inner_glow = p.inner_glow.map(|g| inside_of(soften(&shape, pw, ph, 0.0, 0.0, g.sigma)));
        let over = |c: &mut [f32; 3], a: &mut f32, rgb: [f32; 3], k: f32| { for i in 0..3 { c[i] = rgb[i] * k + c[i] * (1.0 - k); } *a = k + *a * (1.0 - k); };
        let mut out = vec![0u8; n * 4];
        for y in 0..ph { for x in 0..pw {
            let i = y * pw + x;
            let (mut c, mut a) = ([0.0f32; 3], 0.0f32);
            if let (Some(s), Some(v)) = (p.shadow, &shadow) { let k = (v[i] * s.opacity).clamp(0.0, 1.0); c = s.color.map(|v| v * k); a = k; }
            if let (Some(g), Some(v)) = (p.outer_glow, &glow) { over(&mut c, &mut a, g.color, (v[i] * (1.0 - shape[i]) * g.opacity).clamp(0.0, 1.0)); }
            let stroke = match (p.stroke, &ring) { (Some(s), Some(r)) => (r[i] * s.opacity).clamp(0.0, 1.0), _ => 0.0 };
            if let Some(s) = p.stroke { if !s.inside { over(&mut c, &mut a, s.color, stroke); } }
            let mut src: [f32; 4] = if x >= inset && y >= inset && x < inset + w && y < inset + h {
                let j = ((y - inset) * w + x - inset) * 4; [0, 1, 2, 3].map(|k| pixels[j + k] as f32 / 255.0)
            } else { [0.0; 4] };
            let mut recolor = |rgb: [f32; 3], amount: f32| {
                for k in 0..3 { src[k] = src[k] + (rgb[k] * src[3] - src[k]) * amount; }
            };
            if let Some(o) = p.overlay { recolor(o.color, o.opacity); }
            if let (Some(g), Some(v)) = (p.inner_glow, &inner_glow) { recolor(g.color, (v[i] * g.opacity).clamp(0.0, 1.0)); }
            if let (Some(s), Some(v)) = (p.inner_shadow, &inner) { recolor(s.color, (v[i] * s.opacity).clamp(0.0, 1.0)); }
            for k in 0..3 { c[k] = src[k] + c[k] * (1.0 - src[3]); }
            a = src[3] + a * (1.0 - src[3]);
            if let Some(s) = p.stroke { if s.inside { over(&mut c, &mut a, s.color, stroke); } }
            for k in 0..3 { out[i * 4 + k] = (c[k].clamp(0.0, 1.0) * 255.0 + 0.5) as u8; }
            out[i * 4 + 3] = (a.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        }}
        out
    }

//! Isolated, bounded HEIF helper. Production resolves only beside the application.
use super::{Decoded, Page, MAX_BYTES, MAX_PIXELS};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};
static QUEUE: Mutex<()> = Mutex::new(());
struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn helper() -> Result<PathBuf, String> {
    #[cfg(test)]
    if let Some(p) = std::env::var_os("COMPOSITOR_HEIF_HELPER") {
        return Ok(p.into());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let file = exe
        .parent()
        .ok_or("Cannot locate the HEIF decoder")?
        .join("CompositorHeif.exe");
    if !file.is_file() {
        return Err("The portable installation is missing CompositorHeif.exe. Extract the entire portable ZIP.".into());
    }
    Ok(file)
}
#[derive(Debug)]
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub source_bits: u32,
    pub icc: Vec<u8>,
    pub rgba: Vec<u8>,
}
fn read_pixels(path: &Path, remaining: u64) -> Result<Pixels, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut header = [0u8; 28];
    file.read_exact(&mut header).map_err(|e| e.to_string())?;
    let word = |at: usize| u32::from_le_bytes(header[at..at + 4].try_into().unwrap());
    let (width, height, depth, profile, source_bits, reserved) =
        (word(4), word(8), word(12), word(16), word(20), word(24));
    let count = u64::from(width) * u64::from(height);
    if &header[..4] != b"HIF1"
        || depth != 16
        || reserved != 0
        || !(8..=16).contains(&source_bits)
        || width == 0
        || height == 0
        || width > 30000
        || height > 30000
        || count > remaining.min(MAX_PIXELS)
        || profile > 16 * 1024 * 1024
        || file.metadata().map_err(|e| e.to_string())?.len() != 28 + u64::from(profile) + count * 8
    {
        return Err("Invalid or oversized HEIF helper output".into());
    }
    let mut icc = vec![0; profile as usize];
    file.read_exact(&mut icc).map_err(|e| e.to_string())?;
    if !icc.is_empty()
        && (icc.len() < 128
            || &icc[16..20] != b"RGB "
            || &icc[36..40] != b"acsp"
            || u32::from_be_bytes(icc[..4].try_into().unwrap()) as usize != icc.len())
    {
        return Err("HEIF has an invalid or unsupported RGB ICC profile".into());
    }
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(count as usize * 8)
        .map_err(|_| "Not enough memory for HEIF pixels")?;
    rgba.resize(count as usize * 8, 0);
    file.read_exact(&mut rgba).map_err(|e| e.to_string())?;
    Ok(Pixels {
        width,
        height,
        source_bits,
        icc,
        rgba,
    })
}
pub fn decode(input: &Path, remaining: u64) -> Result<Decoded, String> {
    let start = Instant::now();
    let _queue = loop {
        if start.elapsed() > Duration::from_secs(180) {
            return Err("HEIF decoding exceeded its time limit".into());
        }
        match QUEUE.try_lock() {
            Ok(lock) => break lock,
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err("HEIF decoder queue is unavailable".into())
            }
            Err(_) => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    let folder = std::env::temp_dir().join(format!("compositor-heif-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&folder).map_err(|e| e.to_string())?;
    let _cleanup = Temporary(folder.clone());
    let output = folder.join("image.hif1");
    let errors = folder.join("error.txt");
    let mut command = Command::new(helper()?);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .arg(input)
        .arg(&output)
        .arg(remaining.min(MAX_PIXELS).to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&errors)
                .map_err(|e| e.to_string())?,
        );
    let mut child=command.spawn().map_err(|e|format!("Cannot start bundled HEIF decoder: {e}. Extract the complete portable ZIP, including its DLLs."))?;
    let status = loop {
        if start.elapsed() > Duration::from_secs(180) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("HEIF decoding exceeded its time limit".into());
        }
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
        }
    };
    if !status.success() {
        let mut message = String::new();
        let _ = fs::File::open(&errors).and_then(|f| f.take(65536).read_to_string(&mut message));
        return Err(format!(
            "Cannot decode HEIF with the bundled decoder ({status}): {message}"
        ));
    }
    let mut pixels = read_pixels(&output, remaining)?;
    if !pixels.icc.is_empty() {
        color_manage(&mut pixels)?;
    }
    for sample in pixels.rgba.chunks_exact_mut(2) {
        sample.swap(0, 1);
    }
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, pixels.width, pixels.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Sixteen);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer
            .write_image_data(&pixels.rgba)
            .map_err(|e| e.to_string())?;
    }
    if png.len() > MAX_BYTES {
        return Err("Decoded HEIF exceeds the transfer budget".into());
    }
    Ok(Decoded {
        resolution: 72.,
        pages: vec![Page {
            name: input
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            png,
        }],
    })
}
fn color_manage(p: &mut Pixels) -> Result<(), String> {
    use moxcms::{ColorProfile, Layout, TransformOptions};
    let error = |e| format!("HEIF ICC color conversion failed: {e}");
    let source = ColorProfile::new_from_slice(&p.icc).map_err(error)?;
    let transform = source
        .create_transform_16bit(
            Layout::Rgba,
            &ColorProfile::new_srgb(),
            Layout::Rgba,
            TransformOptions {
                prefer_fixed_point: false,
                ..Default::default()
            },
        )
        .map_err(error)?;
    // Bounded bands keep the 16-bit transform independent of image height.
    let band_bytes = p.width as usize * 64 * 8;
    let mut input = vec![0u16; band_bytes / 2];
    let mut output = vec![0u16; band_bytes / 2];
    for band in p.rgba.chunks_mut(band_bytes) {
        let samples = band.len() / 2;
        for (value, bytes) in input[..samples].iter_mut().zip(band.chunks_exact(2)) {
            *value = u16::from_le_bytes([bytes[0], bytes[1]]);
        }
        transform
            .transform(&input[..samples], &mut output[..samples])
            .map_err(error)?;
        // ICC transforms color only; preserve straight alpha exactly.
        for (before, after) in input[..samples]
            .chunks_exact(4)
            .zip(output[..samples].chunks_exact_mut(4))
        {
            after[3] = before[3];
        }
        for (bytes, value) in band.chunks_exact_mut(2).zip(&output[..samples]) {
            bytes.copy_from_slice(&value.to_le_bytes());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(w: u32, h: u32) -> (Temporary, PathBuf, Vec<u8>) {
        let dir = std::env::temp_dir().join(format!("heif-wire-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&dir).unwrap();
        let p = dir.join("pixels");
        let mut b = b"HIF1".to_vec();
        for n in [w, h, 16, 0, 10, 0] {
            b.extend(n.to_le_bytes());
        }
        (Temporary(dir), p, b)
    }
    #[test]
    fn icc_conversion_preserves_alpha_and_deep_samples_across_bands() {
        // Eight-code spacing tolerates ICC table rounding, but an 8-bit conversion
        // would collapse these 130 colors to at most six values.
        let samples: Vec<u16> = (0..130)
            .flat_map(|i| [20000 + i * 8, 30000 + i * 8, 40000 + i * 8, i * 499])
            .collect();
        let mut pixels = Pixels {
            width: 1,
            height: 130,
            source_bits: 16,
            icc: moxcms::ColorProfile::new_srgb().encode().unwrap(),
            rgba: samples.iter().flat_map(|v| v.to_le_bytes()).collect(),
        };
        color_manage(&mut pixels).unwrap();
        let output: Vec<u16> = pixels
            .rgba
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        for (a, b) in samples.chunks_exact(4).zip(output.chunks_exact(4)) {
            assert_eq!(a[3], b[3]);
            for c in 0..3 {
                assert!(
                    a[c].abs_diff(b[c]) <= 4,
                    "identity changed sample: {a:?} -> {b:?}"
                );
            }
        }
        assert!(
            output
                .chunks_exact(4)
                .map(|p| p[0])
                .collect::<std::collections::HashSet<_>>()
                .len()
                > 120
        );
    }
    #[test]
    fn adobe_rgb_conversion_changes_color_and_rejects_bad_profiles() {
        let mut pixels = Pixels {
            width: 1,
            height: 1,
            source_bits: 8,
            icc: moxcms::ColorProfile::new_adobe_rgb().encode().unwrap(),
            rgba: [206u16 * 257, 35 * 257, 44 * 257, 12345]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect(),
        };
        color_manage(&mut pixels).unwrap();
        let rgb: Vec<u16> = pixels
            .rgba
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        for (got, expected) in rgb[..3].iter().zip([240u16, 30, 40]) {
            assert!(((*got as u32 + 128) / 257).abs_diff(expected as u32) <= 2);
        }
        assert_eq!(rgb[3], 12345);
        pixels.icc = vec![0; 128];
        assert!(color_manage(&mut pixels).is_err());
    }
    #[test]
    #[ignore = "Requires the frozen private HEIF ICC fixture and bundled helper"]
    fn real_heif_icc_matches_independent_apple_reference() {
        let root = PathBuf::from(std::env::var_os("COMPOSITOR_ICC_FIXTURES").unwrap());
        let decoded = decode(&root.join("icc-adobe.heic"), MAX_PIXELS).unwrap();
        let read = |bytes: &[u8]| compositor_engine::decode_image(bytes).unwrap().raster;
        let actual = read(&decoded.pages[0].png);
        let reference = read(&fs::read(root.join("icc-adobe-reference.png")).unwrap());
        let mut max = 0u8;
        let mut sum = 0u64;
        let mut count = 0;
        for y in 8..24 {
            for x in [3, 11, 19, 27, 35, 43] {
                for c in 0..3 {
                    let delta = actual.pixel(x, y)[c].abs_diff(reference.pixel(x, y)[c]);
                    max = max.max(delta);
                    sum += u64::from(delta);
                    count += 1;
                }
            }
        }
        let mean = sum as f64 / count as f64;
        println!("HEIF ICC max {max}, mean {mean}");
        assert!(max <= 12 && mean <= 3.);
    }
    #[test]
    fn preserves_adjacent_deep_samples() {
        let (_temp, p, mut b) = fixture(2, 1);
        let values = [20000u16, 30000, 40000, 65535, 20001, 30001, 40001, 0];
        for n in values {
            b.extend(n.to_le_bytes());
        }
        fs::write(&p, b).unwrap();
        let r = read_pixels(&p, 2).unwrap();
        assert_eq!(r.source_bits, 10);
        assert_eq!(
            r.rgba
                .chunks_exact(2)
                .map(|x| u16::from_le_bytes([x[0], x[1]]))
                .collect::<Vec<_>>(),
            values
        );
    }
    #[test]
    fn rejects_truncated_oversized_and_overbudget_output() {
        let (_temp, p, mut b) = fixture(2, 1);
        fs::write(&p, &b).unwrap();
        assert!(read_pixels(&p, 2).is_err());
        b.extend([0; 16]);
        fs::write(&p, &b).unwrap();
        assert!(read_pixels(&p, 1).is_err());
        b[4..8].copy_from_slice(&30001u32.to_le_bytes());
        fs::write(&p, &b).unwrap();
        assert!(read_pixels(&p, MAX_PIXELS).is_err());
    }
    #[test]
    fn rejects_invalid_embedded_profile() {
        let (_temp, p, mut b) = fixture(1, 1);
        b[16..20].copy_from_slice(&128u32.to_le_bytes());
        b.extend([0; 136]);
        fs::write(&p, b).unwrap();
        assert!(read_pixels(&p, 1).unwrap_err().contains("ICC"));
    }
}

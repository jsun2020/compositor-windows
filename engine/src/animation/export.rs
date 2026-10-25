use super::*;
use crate::{CommandError, Document, EffectsCache, ExportError, export_as::{self, ExportOptions}};

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduledFrame { pub start: u64, pub sample: u64, pub ticks: u16 }

/// Merge the tracks' clocks without an unbounded union of all frame-start timestamps.
pub fn schedule(tracks: &[Vec<u64>]) -> Result<Vec<ScheduledFrame>, CommandError> {
    if tracks.is_empty() { return Ok(vec![ScheduledFrame { start: 0, sample: 0, ticks: 10 }]); }
    if tracks.iter().any(|t| t.is_empty() || t.len() > MAX_FRAMES || t.iter().any(|d| !(2..=65535).contains(d))) {
        return Err(CommandError::Argument("Invalid animation timing".into()));
    }
    let starts: Vec<Vec<u64>> = tracks.iter().map(|t| {
        let mut n = 0; t.iter().map(|d| { let start = n; n += d; start }).collect()
    }).collect();
    let end = tracks.iter().map(|t| t.iter().sum::<u64>()).max().unwrap();
    let mut positions = vec![1; tracks.len()];
    let (mut start, mut sample) = (0, 0);
    let mut out = vec![];
    fn append(out: &mut Vec<ScheduledFrame>, start: &mut u64, sample: &mut u64, boundary: u64) -> Result<(), CommandError> {
        while *start < boundary {
            if out.len() >= MAX_FRAMES { return Err(ExportError::TooLarge.into()); }
            let mut ticks = (boundary - *start).min(65535);
            let remainder = boundary - *start - ticks;
            if remainder > 0 && remainder < 2 { ticks -= 2 - remainder; }
            out.push(ScheduledFrame { start: *start, sample: (*start).max(*sample), ticks: ticks as u16 });
            *start += ticks; *sample = *start;
        }
        Ok(())
    }
    loop {
        let next = starts.iter().enumerate().filter_map(|(i, v)| v.get(positions[i])).copied().min().unwrap_or(end);
        if next == end { break; }
        for (i, v) in starts.iter().enumerate() { if v.get(positions[i]) == Some(&next) { positions[i] += 1; } }
        if next <= start { continue; }
        if next - start >= 2 { append(&mut out, &mut start, &mut sample, next)?; }
        else if end - start >= 4 { let boundary = start + 2; append(&mut out, &mut start, &mut sample, boundary)?; }
        else { sample = next; }
    }
    append(&mut out, &mut start, &mut sample, end)?;
    Ok(out)
}

struct LimitedBytes(Vec<u8>);
impl std::io::Write for LimitedBytes {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.len() > SOURCE_BYTE_LIMIT - self.0.len() { return Err(std::io::Error::other("GIF exceeds 512 MiB")); }
        self.0.extend_from_slice(buf); Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

pub fn encode(doc: &Document, options: &ExportOptions, preview: u32) -> Result<Vec<u8>, CommandError> {
    options.validate()?;
    let tracks: Vec<_> = crate::compositor::render_layers(doc).into_iter()
        .filter_map(|l| l.extra.gif.as_ref().map(|gif| (l.id, gif))).collect();
    if tracks.iter().any(|(_, g)| !g.valid()) { return Err(CommandError::Argument("Invalid animation timeline".into())); }
    let refs: Vec<_> = tracks.iter().map(|(_, g)| g.references()).collect();
    let delays: Vec<Vec<u64>> = tracks.iter().zip(&refs).map(|((_, g), refs)| refs.iter().map(|f| (g.duration(*f) * 100.0).round() as u64).collect()).collect();
    let schedule = schedule(&delays)?;
    if options.width as u64 * options.height as u64 > MAX_PIXELS / schedule.len() as u64 { return Err(ExportError::TooLarge.into()); }
    let mut resized = options.clone();
    if preview > 0 {
        let scale = (preview as f64 / options.width.max(options.height) as f64).min(1.0);
        resized.width = (options.width as f64 * scale).round().max(1.0) as u32;
        resized.height = (options.height as f64 * scale).round().max(1.0) as u32;
    }
    let starts: Vec<Vec<u64>> = delays.iter().map(|t| { let mut n = 0; t.iter().map(|d| { let start = n; n += d; start }).collect() }).collect();
    let mut capture = doc.clone();
    let mut positions = vec![usize::MAX; tracks.len()];
    let mut bytes = LimitedBytes(vec![]);
    {
        let mut encoder = gif::Encoder::new(&mut bytes, resized.width as u16, resized.height as u16, &[]).map_err(|_| ExportError::Encode)?;
        if schedule.len() > 1 && options.gif_loop.unwrap_or(true) { encoder.set_repeat(gif::Repeat::Infinite).map_err(|_| ExportError::Encode)?; }
        // Each frame owns a local palette and clears its full rectangle before the next frame.
        // This prevents old opaque pixels showing through a later transparent frame.
        for frame in schedule {
            for (i, (id, animation)) in tracks.iter().enumerate() {
                let position = starts[i].partition_point(|start| *start <= frame.sample).saturating_sub(1);
                if positions[i] != position {
                    let pixels = animation.frame(refs[i][position].index)?;
                    capture.layer_mut(*id).unwrap().set_pixels(Some(pixels));
                    positions[i] = position;
                }
            }
            let cache = EffectsCache::default();
            let raster = export_as::render(&capture, &cache, &resized, 0)?;
            let raster = if let Some(bg) = resized.background { export_as::flatten(raster, bg) } else { raster };
            let mut straight = raster.to_straight();
            for px in straight.chunks_exact_mut(4) { if px[3] < 128 { px.fill(0); } else { px[3] = 255; } }
            let mut encoded = gif::Frame::from_rgba_speed(resized.width as u16, resized.height as u16, &mut straight, 10);
            encoded.delay = frame.ticks;
            encoded.dispose = gif::DisposalMethod::Background;
            encoder.write_frame(&encoded).map_err(|_| ExportError::Encode)?;
        }
    }
    Ok(bytes.0)
}

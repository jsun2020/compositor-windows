//! Layered RGB PSD, with an independent merged image and editable layer masks.
use crate::*;
use std::collections::HashMap;
use uuid::Uuid;

fn too_large() -> CommandError {
    CommandError::Export(ExportError::TooLarge)
}

fn length(size: usize) -> Result<[u8; 4], CommandError> {
    Ok(u32::try_from(size).map_err(|_| too_large())?.to_be_bytes())
}

fn block(out: &mut Vec<u8>, key: &[u8; 4], data: &[u8]) -> Result<(), CommandError> {
    out.extend(b"8BIM");
    out.extend(key);
    out.extend(length(data.len())?);
    out.extend(data);
    if data.len() % 2 != 0 {
        out.push(0);
    }
    Ok(())
}

#[derive(Clone, Copy, Default)]
struct Bounds {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl Bounds {
    fn write(self, out: &mut Vec<u8>) {
        for n in [self.y, self.x, self.y + self.h, self.x + self.w] {
            out.extend(n.to_be_bytes());
        }
    }
    fn canvas(doc: &Document) -> Self {
        Self {
            x: 0,
            y: 0,
            w: doc.width,
            h: doc.height,
        }
    }
    fn region(self) -> Rect {
        Rect {
            x: self.x.into(),
            y: self.y.into(),
            width: self.w.into(),
            height: self.h.into(),
        }
    }
}

fn blend(mode: BlendMode) -> &'static [u8; 4] {
    use BlendMode::*;
    match mode {
        Normal => b"norm",
        Multiply => b"mul ",
        Screen => b"scrn",
        Overlay => b"over",
        Darken => b"dark",
        Lighten => b"lite",
        Difference => b"diff",
        ColorDodge => b"div ",
        ColorBurn => b"idiv",
        Hue => b"hue ",
        Saturation => b"sat ",
        Color => b"colr",
        Luminosity => b"lum ",
        LinearBurn => b"lbrn",
        LinearDodge => b"lddg",
        SoftLight => b"sLit",
        HardLight => b"hLit",
        VividLight => b"vLit",
        LinearLight => b"lLit",
        PinLight => b"pLit",
        HardMix => b"hMix",
        Exclusion => b"smud",
        Subtract => b"fsub",
        Divide => b"fdiv",
    }
}

fn reserve(out: &mut Vec<u8>, count: usize) -> Result<(), CommandError> {
    if out
        .len()
        .checked_add(count)
        .filter(|n| *n <= i32::MAX as usize)
        .is_none()
    {
        return Err(too_large());
    }
    out.try_reserve(count).map_err(|_| too_large())
}

/// PSD stores straight colour in planes, with all 16-bit words in network byte order.
fn planes(
    out: &mut Vec<u8>,
    raster: &Raster,
    depth: usize,
    layer: bool,
) -> Result<(), CommandError> {
    let plane = raster.width as usize * raster.height as usize * depth;
    reserve(
        out,
        plane
            .checked_mul(4)
            .and_then(|n| n.checked_add(8))
            .ok_or_else(too_large)?,
    )?;
    for c in 0..4 {
        if layer {
            out.extend([0, 0]);
        } // raw channel
        for y in 0..raster.height {
            for x in 0..raster.width {
                let p = raster.pixel_f32(x, y);
                let v = if c == 3 {
                    p[3]
                } else if p[3] > 0. {
                    p[c] / p[3]
                } else {
                    0.
                };
                let v = v.clamp(0., 1.);
                if depth == 2 {
                    out.extend(((v * 65535.).round() as u16).to_be_bytes());
                } else {
                    out.push((v * 255.).round() as u8);
                }
            }
        }
    }
    Ok(())
}

struct Writer<'a> {
    doc: &'a Document,
    cache: &'a EffectsCache,
    siblings: HashMap<Option<Uuid>, Vec<&'a Layer>>,
    depth: usize,
    records: Vec<u8>,
    channels: Vec<u8>,
    count: i16,
}

impl Writer<'_> {
    fn tree(&mut self, parent: Option<Uuid>, depth: usize) -> Result<(), CommandError> {
        if depth > MAX_NESTING {
            return Err(too_large());
        }
        for layer in self.siblings.remove(&parent).unwrap_or_default() {
            if layer.is_group {
                self.record(layer, 3)?;
                self.tree(Some(layer.id), depth + 1)?;
                self.record(layer, 1)?;
            } else if !layer.is_adjustment() {
                self.record(layer, 0)?;
            }
        }
        Ok(())
    }

    fn pixels(
        &self,
        layer: &Layer,
        bake_effects: bool,
    ) -> Result<Option<(Bounds, Raster)>, CommandError> {
        let Some(image) = layer.pixels.as_ref() else {
            return Ok(None);
        };
        let mut isolated = layer.clone();
        isolated.parent_id = None;
        isolated.mask_source_id = None;
        isolated.visible = true;
        isolated.opacity = 1.;
        isolated.blend_mode = BlendMode::Normal;
        if !bake_effects {
            isolated.mask = None;
        }
        let placed = effects_draw(layer, None).map_or(layer.transform, |fx| {
            grown_transform(&layer.transform, image.width, image.height, fx.inset)
        });
        let b = placed.bounds();
        let x = b.x.floor().clamp(0., self.doc.width.into()) as u32;
        let y = b.y.floor().clamp(0., self.doc.height.into()) as u32;
        let right = (b.x + b.width)
            .ceil()
            .clamp(x.into(), self.doc.width.into()) as u32;
        let bottom = (b.y + b.height)
            .ceil()
            .clamp(y.into(), self.doc.height.into()) as u32;
        if x == right || y == bottom {
            return Ok(None);
        }
        let area = Bounds {
            x,
            y,
            w: right - x,
            h: bottom - y,
        };
        let mut doc = Document::new(self.doc.width, self.doc.height);
        doc.layers.push(isolated);
        let image =
            checked_composite_edit_with(&doc, None, area.region(), area.w, area.h, self.cache)?;
        let mut bounds = (area.w, area.h, 0, 0);
        for y in 0..area.h {
            for x in 0..area.w {
                if image.pixel_f32(x, y)[3] > 0. {
                    bounds = (
                        bounds.0.min(x),
                        bounds.1.min(y),
                        bounds.2.max(x + 1),
                        bounds.3.max(y + 1),
                    );
                }
            }
        }
        let (l, t, r, b) = bounds;
        if r == 0 {
            return Ok(None);
        }
        let rect = Bounds {
            x: x + l,
            y: y + t,
            w: r - l,
            h: b - t,
        };
        let image = if (rect.w, rect.h) == (area.w, area.h) {
            image
        } else {
            image.cropped(l, t, rect.w, rect.h)
        };
        Ok(Some((rect, image)))
    }

    fn record(&mut self, layer: &Layer, section: u32) -> Result<(), CommandError> {
        let effects = effects_draw(layer, None).is_some();
        let pixels = if section == 0 {
            self.pixels(layer, effects)?
        } else {
            None
        };
        let rect = pixels.as_ref().map_or(Bounds::default(), |p| p.0);
        let mask = layer
            .mask
            .as_ref()
            .filter(|_| section == 1 || (pixels.is_some() && !effects && section == 0));
        let mask_rect = if section == 1 {
            Bounds::canvas(self.doc)
        } else {
            rect
        };
        self.count = self.count.checked_add(1).ok_or_else(too_large)?;
        rect.write(&mut self.records);
        self.records
            .extend((4u16 + u16::from(mask.is_some())).to_be_bytes());
        for id in [0i16, 1, 2, -1, -2]
            .into_iter()
            .take(4 + usize::from(mask.is_some()))
        {
            let b = if id == -2 { mask_rect } else { rect };
            self.records.extend(id.to_be_bytes());
            self.records
                .extend(length(2 + b.w as usize * b.h as usize * self.depth)?);
        }
        let key = if section == 1 {
            b"pass"
        } else {
            blend(layer.blend_mode)
        };
        self.records.extend(b"8BIM");
        self.records.extend(key);
        self.records.extend([
            (layer.opacity.clamp(0., 1.) * 255.).round() as u8,
            u8::from(section == 0 && layer.mask_source_id.is_some()),
            if section == 0 { 0 } else { 24 } | if !layer.visible { 2 } else { 0 },
            0,
        ]);
        let mut extra = Vec::new();
        if let Some(m) = mask {
            extra.extend(20u32.to_be_bytes());
            mask_rect.write(&mut extra);
            extra.extend([
                m.background(),
                u8::from(!m.is_linked()) | (u8::from(!m.enabled) << 1),
                0,
                0,
            ]);
        } else {
            extra.extend([0; 4]);
        }
        extra.extend([0; 4]); // blending ranges
        let name = if section == 3 {
            "</Layer group>"
        } else {
            &layer.name
        };
        let legacy: Vec<_> = name
            .chars()
            .take(255)
            .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
            .collect();
        extra.push(legacy.len() as u8);
        extra.extend(&legacy);
        extra.resize(extra.len() + (4 - (1 + legacy.len()) % 4) % 4, 0);
        let mut unicode = length(name.encode_utf16().count())?.to_vec();
        for unit in name.encode_utf16() {
            unicode.extend(unit.to_be_bytes());
        }
        block(&mut extra, b"luni", &unicode)?;
        if section != 0 {
            let mut divider = section.to_be_bytes().to_vec();
            if section == 1 {
                divider.extend(b"8BIM");
                divider.extend(key);
            }
            block(&mut extra, b"lsct", &divider)?;
        }
        self.records.extend(length(extra.len())?);
        self.records.extend(extra);
        if let Some((_, image)) = pixels {
            planes(&mut self.channels, &image, self.depth, true)?;
        } else {
            self.channels.extend([0; 8]);
        }
        if let Some(m) = mask {
            reserve(
                &mut self.channels,
                2 + mask_rect.w as usize * mask_rect.h as usize * self.depth,
            )?;
            self.channels.extend([0, 0]);
            let mut enabled = layer.clone();
            enabled.mask.as_mut().unwrap().enabled = true;
            let cov = crate::plan::own_coverage(&enabled, None).unwrap();
            for y in mask_rect.y..mask_rect.y + mask_rect.h {
                for x in mask_rect.x..mask_rect.x + mask_rect.w {
                    let v = coverage_at(
                        self.doc,
                        &cov,
                        Point {
                            x: x as f64 + 0.5,
                            y: y as f64 + 0.5,
                        },
                    );
                    if self.depth == 2 {
                        self.channels
                            .extend(((v * 65535.).round() as u16).to_be_bytes());
                    } else {
                        self.channels.push((v * 255.).round() as u8);
                    }
                }
            }
            debug_assert_eq!(m.background(), cov.background);
        }
        Ok(())
    }
}

pub fn encode(
    doc: &Document,
    cache: &EffectsCache,
    merged: &Raster,
) -> Result<Vec<u8>, CommandError> {
    if (merged.width, merged.height) != (doc.width, doc.height) {
        return Err(CommandError::Argument(
            "PSD requires the document's original dimensions".into(),
        ));
    }
    let depth = if doc.layers.iter().any(|l| {
        l.pixels
            .as_ref()
            .is_some_and(|p| p.depth() != PixelDepth::Eight)
    }) {
        2
    } else {
        1
    };
    let mut writer = Writer {
        doc,
        cache,
        siblings: HashMap::new(),
        depth,
        records: Vec::new(),
        channels: Vec::new(),
        count: 0,
    };
    for layer in &doc.layers {
        writer
            .siblings
            .entry(layer.parent_id)
            .or_default()
            .push(layer);
    }
    writer.tree(None, 0)?;
    let mut out = b"8BPS\0\x01\0\0\0\0\0\0\0\x04".to_vec();
    out.extend(doc.height.to_be_bytes());
    out.extend(doc.width.to_be_bytes());
    out.extend(((depth * 8) as u16).to_be_bytes());
    out.extend([0, 3, 0, 0, 0, 0]);
    out.extend(28u32.to_be_bytes());
    out.extend(b"8BIM");
    out.extend(1005u16.to_be_bytes());
    out.extend([0, 0]);
    out.extend(16u32.to_be_bytes());
    for _ in 0..2 {
        out.extend(((doc.resolution.clamp(1., 9600.) * 65536.).round() as u32).to_be_bytes());
        out.extend([0, 1, 0, 1]);
    }
    let size = 2 + writer.records.len() + writer.channels.len();
    let padded = size + size % 2;
    reserve(&mut out, padded + 12)?;
    out.extend(length(padded + 8)?);
    out.extend(length(padded)?);
    out.extend((-writer.count).to_be_bytes());
    out.append(&mut writer.records);
    drop(writer.records);
    out.append(&mut writer.channels);
    drop(writer.channels);
    if size % 2 != 0 {
        out.push(0);
    }
    out.extend([0; 4]);
    out.extend([0, 0]);
    planes(&mut out, merged, depth, false)?;
    Ok(out)
}

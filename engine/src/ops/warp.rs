//! Document-space warp and replacement writeback, following Mac 1.4.5
//! SmudgeLiquify.swift. CPU reference execution stays bounded; the writeback
//! also accepts a completed GPU plane without resampling the edited pixels.
use crate::warp::{WarpMode, WarpSettings, WarpStroke, REFERENCE_MAX_PIXELS};
use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MASK_REFUSAL: &str = "Smudge and Liquify work on a layer's pixels, not its mask.";
pub const REFERENCE_LIMIT: &str =
    "This warp needs the GPU path; the CPU reference is limited to 4 Mi pixels.";
const MAX_INPUT_POINTS: usize = 4096;
const MAX_STROKE_SAMPLES: usize = 67_108_864;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WarpSpec {
    pub mode: WarpMode,
    pub diameter: f64,
    pub hardness: f64,
    pub strength: f64,
    pub points: Vec<Point>,
}

impl WarpSpec {
    pub fn check(&self) -> Result<(), CommandError> {
        if !self.diameter.is_finite()
            || !(2.0..=2000.0).contains(&self.diameter)
            || !self.hardness.is_finite()
            || !(0.0..=0.98).contains(&self.hardness)
            || !self.strength.is_finite()
            || !(0.01..=1.0).contains(&self.strength)
            || self.points.is_empty()
            || self.points.len() > MAX_INPUT_POINTS
            || self.points.iter().any(|p| !valid_point(*p))
        {
            return Err(CommandError::Argument("warp settings out of range".into()));
        }
        Ok(())
    }
}

fn valid_point(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.x.abs() <= 1_000_000.0 && p.y.abs() <= 1_000_000.0
}

/// The actual scheduled centers retained for the final diameter+4 hard tip.
/// GPU clients send input points, not an arbitrary replacement footprint.
pub fn footprint(spec: &WarpSpec) -> Result<Vec<Point>, CommandError> {
    spec.check()?;
    let side = (2.0 * (spec.diameter / 2.0).ceil() + 1.0) as usize;
    let spacing = (spec.diameter * 0.05).max(1.0);
    let mut anchor = None;
    let mut kept: Vec<Point> = Vec::new();
    let mut final_dab = None;
    let mut work = 0usize;
    for p in &spec.points {
        let dabs = crate::warp::schedule_dabs(spec.mode, spec.diameter, anchor, [p.x, p.y])
            .map_err(|_| CommandError::Argument("the warp stroke is too long".into()))?;
        work = dabs
            .len()
            .checked_mul(side)
            .and_then(|n| n.checked_mul(side))
            .and_then(|n| work.checked_add(n))
            .filter(|n| *n <= MAX_STROKE_SAMPLES)
            .ok_or_else(|| CommandError::Argument("the warp stroke is too long".into()))?;
        if anchor.is_none() || !dabs.is_empty() {
            anchor = Some([p.x, p.y]);
        }
        for dab in dabs {
            let p = Point {
                x: dab[0],
                y: dab[1],
            };
            if kept
                .last()
                .is_none_or(|last| (p.x - last.x).hypot(p.y - last.y) >= spacing)
            {
                kept.push(p);
            }
            final_dab = Some(p);
            if kept.len() > 100_000 {
                return Err(CommandError::Argument("the warp stroke is too long".into()));
            }
        }
    }
    if let Some(last) = final_dab {
        if kept.last() != Some(&last) {
            kept.push(last);
        }
    }
    Ok(kept)
}

pub(crate) fn target(doc: &Document, id: Uuid, mask: bool) -> Result<&Layer, CommandError> {
    if mask {
        return Err(CommandError::Refused(MASK_REFUSAL.into()));
    }
    ops::raster_edit::check_target(doc, id, false)?;
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
    if layer.pixels.is_none() {
        return Err(CommandError::Refused("The layer has no pixels".into()));
    }
    let (w, h) = ops::raster_edit::layer_grid(layer);
    if layer.transform.pixel_to_document(w, h).invert().is_none() {
        return Err(CommandError::Argument("the layer cannot be painted".into()));
    }
    Ok(layer)
}

/// The unmasked, full-opacity asset in document coordinates. The warp must not
/// bake opacity, blend mode, effects, masks or neighbouring layers into pixels.
pub fn source_plane(doc: &Document, id: Uuid) -> Result<Raster, CommandError> {
    let mut layer = target(doc, id, false)?.clone();
    layer.parent_id = None;
    layer.mask_source_id = None;
    layer.mask = None;
    layer.extra.effects = None;
    layer.opacity = 1.0;
    layer.blend_mode = BlendMode::Normal;
    layer.visible = true;
    let mut single = Document::new(doc.width, doc.height);
    single.layers = vec![layer];
    Ok(compositor::composite(
        &single,
        Rect {
            x: 0.0,
            y: 0.0,
            width: doc.width as f64,
            height: doc.height as f64,
        },
        doc.width,
        doc.height,
    ))
}

/// Existing edit-job execution makes a detached document and installs it only
/// through LayerStamp. No document mutation occurs until the entire CPU stroke
/// and replacement have succeeded. This is not the large-canvas preview path.
pub fn apply_reference(
    doc: &mut Document,
    clips: &SelectionClips,
    id: Uuid,
    mask: bool,
    spec: &WarpSpec,
) -> Result<Dirty, CommandError> {
    spec.check()?;
    let own = target(doc, id, mask)?.pixels.as_ref().unwrap();
    if doc.width as u64 * doc.height as u64 > REFERENCE_MAX_PIXELS as u64
        || own.width as u64 * own.height as u64 > REFERENCE_MAX_PIXELS as u64
    {
        return Err(CommandError::Refused(REFERENCE_LIMIT.into()));
    }
    let source = source_plane(doc, id)?;
    let mut stroke = WarpStroke::new(
        doc.width as usize,
        doc.height as usize,
        source.bytes(),
        spec.mode,
        WarpSettings {
            diameter: spec.diameter,
            hardness: spec.hardness as f32,
            strength: spec.strength as f32,
        },
    )
    .map_err(|_| CommandError::Argument("invalid warp source or settings".into()))?;
    let mut kept: Vec<Point> = Vec::new();
    let spacing = (spec.diameter * 0.05).max(1.0);
    let side = (2.0 * (spec.diameter / 2.0).ceil() + 1.0) as usize;
    let mut work = 0usize;
    let mut final_dab = None;
    for p in &spec.points {
        let dabs = stroke
            .append_dabs([p.x, p.y])
            .map_err(|_| CommandError::Argument("the warp stroke is too long".into()))?;
        work = dabs
            .len()
            .checked_mul(side)
            .and_then(|n| n.checked_mul(side))
            .and_then(|n| work.checked_add(n))
            .filter(|n| *n <= MAX_STROKE_SAMPLES)
            .ok_or_else(|| CommandError::Argument("the warp stroke is too long".into()))?;
        for dab in dabs {
            let p = Point {
                x: dab[0],
                y: dab[1],
            };
            if kept
                .last()
                .is_none_or(|last| (p.x - last.x).hypot(p.y - last.y) >= spacing)
            {
                kept.push(p);
            }
            final_dab = Some(p);
        }
    }
    let Some(last) = final_dab else {
        return Ok(Dirty::pixels(vec![]));
    };
    if kept.last() != Some(&last) {
        kept.push(last);
    }
    let result = Raster::from_premultiplied(doc.width, doc.height, stroke.pixels().to_vec());
    writeback(doc, clips, id, spec, &kept, &result, true)
}

/// One replacement edit through the final hard brush footprint (diameter+4),
/// canvas bounds and selection coverage. `bounded_reference` limits CPU grids;
/// GPU integration still uses the existing project budget checks.
pub fn writeback(
    doc: &mut Document,
    clips: &SelectionClips,
    id: Uuid,
    spec: &WarpSpec,
    dabs: &[Point],
    result: &Raster,
    bounded_reference: bool,
) -> Result<Dirty, CommandError> {
    spec.check()?;
    let original = target(doc, id, false)?.clone();
    if (result.width, result.height) != (doc.width, doc.height)
        || dabs.len() > 100_000
        || dabs.iter().any(|p| !valid_point(*p))
        || dabs
            .windows(2)
            .map(|p| (p[1].x - p[0].x).hypot(p[1].y - p[0].y))
            .sum::<f64>()
            > 2_000_000.0
    {
        return Err(CommandError::Argument("invalid warp result".into()));
    }
    if dabs.is_empty() {
        return Ok(Dirty::pixels(vec![]));
    }
    let brush = BrushSpec {
        diameter: spec.diameter + 4.0,
        hardness: 1.0,
        opacity: 1.0,
        points: dabs.to_vec(),
        color: [0.0; 4],
        erasing: false,
        operation: BrushOperation::Paint,
    };
    let radius = brush.diameter * 0.5 + 1.0;
    let left = dabs
        .iter()
        .map(|p| p.x)
        .fold(f64::INFINITY, f64::min)
        .floor()
        - radius;
    let top = dabs
        .iter()
        .map(|p| p.y)
        .fold(f64::INFINITY, f64::min)
        .floor()
        - radius;
    let right = (dabs
        .iter()
        .map(|p| p.x)
        .fold(f64::NEG_INFINITY, f64::max)
        .ceil()
        + radius)
        .min(doc.width as f64);
    let bottom = (dabs
        .iter()
        .map(|p| p.y)
        .fold(f64::NEG_INFINITY, f64::max)
        .ceil()
        + radius)
        .min(doc.height as f64);
    let (left, top) = (left.max(0.0), top.max(0.0));
    if right <= left || bottom <= top {
        return Ok(Dirty::pixels(vec![]));
    }
    let grid = ops::brush::grid_for(
        doc,
        &original,
        Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        },
    )?;
    if bounded_reference && grid.width as u64 * grid.height as u64 > REFERENCE_MAX_PIXELS as u64 {
        return Err(CommandError::Refused(REFERENCE_LIMIT.into()));
    }
    let (rect, coverage) =
        ops::brush::coverage(&brush, doc, clips, grid.transform, grid.width, grid.height);
    if coverage.iter().all(|v| *v == 0.0) {
        return Ok(Dirty::pixels(vec![]));
    }
    let mut bytes = ops::raster_edit::pixels_on(&grid, original.pixels.as_ref());
    let to_doc = grid.transform.pixel_to_document(grid.width, grid.height);
    let mut changed = false;
    for y in 0..rect.height {
        for x in 0..rect.width {
            let amount = coverage[(y * rect.width + x) as usize] as f64;
            if amount <= 0.0 {
                continue;
            }
            let p = to_doc.apply(Point {
                x: (x + rect.x) as f64 + 0.5,
                y: (y + rect.y) as f64 + 0.5,
            });
            let color = compositor::sample(result, p.x, p.y, false);
            let at = (((y + rect.y) * grid.width + x + rect.x) * 4) as usize;
            for k in 0..4 {
                // Replacement includes alpha: transparent dragged pixels can clear
                // the destination, unlike source-over clone/ordinary painting.
                let v = (color[k] as f64 * 255.0 * amount + bytes[at + k] as f64 * (1.0 - amount))
                    .round()
                    .clamp(0.0, 255.0) as u8;
                changed |= v != bytes[at + k];
                bytes[at + k] = v;
            }
        }
    }
    if !changed {
        return Ok(Dirty::pixels(vec![]));
    }
    let target = doc.layer_mut(id).unwrap();
    target.set_pixels(Some(Raster::from_premultiplied(
        grid.width,
        grid.height,
        bytes,
    )));
    target.transform = grid.transform;
    if let Some(mask) = &original.mask {
        if mask.placement.is_none() {
            target.mask_mut().unwrap().pixels = ops::raster_edit::followed(
                &mask.pixels,
                ops::raster_edit::layer_grid(&original),
                &grid,
                (0, 0, grid.width, grid.height),
            );
        }
    }
    let dirty = Dirty::pixels(vec![id]);
    Ok(
        if original
            .pixels
            .as_ref()
            .is_some_and(|p| p.width == grid.width && p.height == grid.height)
        {
            dirty.within(vec![Region {
                layer: id,
                plane: Plane::Pixels,
                rect,
            }])
        } else {
            dirty
        },
    )
}

//! The selection (Phase 4a): Compositor for Mac's `DocumentSelection` (Document/Selection.swift:7-49).
//! An outline in document pixels, filled by the winding rule, with an anti-alias flag and a feather.
//! It is part of the `Document`, so undo covers it, and it is never saved (EditorSession.swift:70-71).
//!
//! The outline is a set of closed contours in fixed point, `SUBPIXEL` units per document pixel, so
//! that whole-pixel shapes stay exact through every boolean (`geometry`), a move is exact, and two
//! equal selections compare equal. `coverage` rasterises it as Core Graphics fills it; `trace` turns
//! pixels back into outlines (WandPixels.c `wand_trace`); `wand` is the Magic Wand's matcher.
pub mod coverage;
pub mod feather;
pub mod geometry;
pub mod outline;
pub mod trace;
pub mod wand;

use crate::{Point, Rect};
use serde::{Deserialize, Serialize};

/// Fixed-point units per document pixel in which an outline is stored and combined.
pub const SUBPIXEL: f64 = 256.0;
/// No outline point lies further than this from the canvas origin, in document pixels (the i32
/// fixed-point range, with room for a 500 px Expand). A move that would pass it is refused.
pub const SELECTION_COORDINATE_LIMIT: f64 = 1_000_000.0;
/// Feather amounts reach 250 px (Selection.swift:307, :330).
pub const MAX_FEATHER: f64 = 250.0;
/// Expand and Contract take 1 to 500 px (Selection.swift:307, :334).
pub const MAX_RESIZE: u32 = 500;

/// One closed contour, in `SUBPIXEL` units.
pub type Contour = Vec<[i32; 2]>;

#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
    /// Closed contours in `SUBPIXEL` units of document space, top-left origin, filled by the nonzero
    /// winding rule. Empty contours or a zero-area outline make an explicit empty selection.
    pub contours: Vec<Contour>,
    /// Hard pixel edges when false (and no feather), as the Mac's Anti-alias toggle.
    pub antialiased: bool,
    /// How far the edge fades, in document pixels, 0 to `MAX_FEATHER`; sigma is half of it.
    pub feather: f64,
}

impl Selection {
    pub fn new(contours: Vec<Contour>, antialiased: bool, feather: f64) -> Selection { Selection { contours, antialiased, feather } }
    /// The outline's bounding box in document pixels; None when it has no points.
    pub fn bounds(&self) -> Option<Rect> {
        let mut points = self.contours.iter().flatten();
        let first = points.next()?;
        let (mut x0, mut y0, mut x1, mut y1) = (first[0], first[1], first[0], first[1]);
        for p in points { x0 = x0.min(p[0]); y0 = y0.min(p[1]); x1 = x1.max(p[0]); y1 = y1.max(p[1]); }
        Some(Rect { x: x0 as f64 / SUBPIXEL, y: y0 as f64 / SUBPIXEL, width: (x1 - x0) as f64 / SUBPIXEL, height: (y1 - y0) as f64 / SUBPIXEL })
    }
    /// An explicit empty selection: no outline, or one whose bounds have no area (`isEmpty`).
    /// Every edit refuses it; it is not the same as no selection, which edits the whole layer.
    pub fn is_empty(&self) -> bool { self.bounds().map_or(true, |b| !(b.width > 0.0) || !(b.height > 0.0)) }
    /// The bounds grown by ceil(2 x feather): four sigmas of the fade (`coverageBounds`).
    pub fn coverage_bounds(&self) -> Option<Rect> {
        let b = self.bounds()?;
        let g = (self.feather * 2.0).ceil();
        Some(Rect { x: b.x - g, y: b.y - g, width: b.width + 2.0 * g, height: b.height + 2.0 * g })
    }
    /// The outline in document pixels.
    pub fn points(&self) -> Vec<Vec<Point>> {
        self.contours.iter().map(|c| c.iter().map(|p| Point { x: p[0] as f64 / SUBPIXEL, y: p[1] as f64 / SUBPIXEL }).collect()).collect()
    }
    /// How many points the outline has, all contours together.
    pub fn point_count(&self) -> usize { self.contours.iter().map(Vec::len).sum() }
    /// Whether `p` lies inside, by the nonzero winding rule (`path.contains(_, using: .winding)`).
    pub fn contains(&self, p: Point) -> bool {
        let (px, py) = (p.x * SUBPIXEL, p.y * SUBPIXEL);
        let mut winding = 0i32;
        for c in &self.contours {
            for i in 0..c.len() {
                let (a, b) = (c[i], c[(i + 1) % c.len()]);
                let (ay, by) = (a[1] as f64, b[1] as f64);
                if (ay <= py) != (by <= py) {
                    let x = a[0] as f64 + (py - ay) * (b[0] - a[0]) as f64 / (by - ay);
                    if x > px { winding += if by > ay { 1 } else { -1 }; }
                }
            }
        }
        winding != 0
    }
    /// The same outline moved by whole `SUBPIXEL` units, flags kept.
    pub fn translated(&self, dx: i32, dy: i32) -> Selection {
        Selection { contours: self.contours.iter().map(|c| c.iter().map(|p| [p[0] + dx, p[1] + dy]).collect()).collect(), ..self.clone() }
    }
}

/// How a new outline meets the current selection (`SelectionMode`, Selection.swift:85-89).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionMode { Replace, Add, Subtract }

/// The drawn outline's kind (`LassoKind`, Selection.swift:75-83): the Lasso's two and the Marquee's two.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionShape { Freehand, Polygonal, Rectangle, Ellipse }

impl SelectionShape {
    /// The undo name each makes (Selection.swift:228-230).
    pub fn action_name(self) -> &'static str {
        match self {
            SelectionShape::Freehand => "Lasso", SelectionShape::Polygonal => "Polygonal Lasso",
            SelectionShape::Rectangle => "Rectangular Marquee", SelectionShape::Ellipse => "Elliptical Marquee",
        }
    }
}

/// What the app needs to know about the selection without its outline (`DocumentState`): the
/// outline itself travels only when `revision` changes (`Engine::selection_outline`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionState {
    /// Issued by the engine's revision counter whenever the outline or its flags change; never reused.
    pub revision: u64,
    pub empty: bool,
    /// The outline's bounds in document pixels; None for an outline with no points.
    pub bounds: Option<Rect>,
    pub antialiased: bool,
    pub feather: f64,
    pub points: usize,
}

use crate::{Document, GuideAxis};
use std::sync::Arc;

/// v1.4.9 CanvasRotation.swift: change placement, never resample a pixel buffer.
pub fn rotate_canvas(doc: &mut Document, clockwise: bool) {
    let canvas = doc.size();
    for layer in &mut doc.layers {
        layer.transform = layer.transform.quarter_turned(clockwise, canvas);
        if let Some(mask) = &mut layer.mask {
            if let Some(placement) = mask.placement {
                mask.placement = Some(placement.quarter_turned(clockwise, canvas));
            }
        }
    }
    for guide in &mut doc.guides {
        guide.position = match (guide.axis, clockwise) {
            (GuideAxis::Vertical, true) | (GuideAxis::Horizontal, false) => guide.position,
            (GuideAxis::Vertical, false) => canvas.width - guide.position,
            (GuideAxis::Horizontal, true) => canvas.height - guide.position,
        };
        guide.axis = match guide.axis {
            GuideAxis::Vertical => GuideAxis::Horizontal,
            GuideAxis::Horizontal => GuideAxis::Vertical,
        };
    }
    if let Some(selection) = &mut doc.selection {
        let w = doc.width as i32 * 256;
        let h = doc.height as i32 * 256;
        selection.contours = Arc::new(
            selection
                .contours
                .iter()
                .map(|contour| {
                    contour
                        .iter()
                        .map(|&[x, y]| if clockwise { [h - y, x] } else { [y, w - x] })
                        .collect()
                })
                .collect(),
        );
    }
    std::mem::swap(&mut doc.width, &mut doc.height);
}

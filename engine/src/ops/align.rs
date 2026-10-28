//! Mac 1.5.8 layer alignment. A selected folder moves as one item; selected
//! descendants never move twice. Coordinates and offsets are document pixels.
use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LayerAlignment {
    #[serde(rename="Left Edges")] Left,
    #[serde(rename="Horizontal Centers")] HorizontalCenter,
    #[serde(rename="Right Edges")] Right,
    #[serde(rename="Top Edges")] Top,
    #[serde(rename="Vertical Centers")] VerticalCenter,
    #[serde(rename="Bottom Edges")] Bottom,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LayerDistribution {
    #[serde(rename="Horizontal Centers")] HorizontalCenters,
    #[serde(rename="Vertical Centers")] VerticalCenters,
    #[serde(rename="Horizontal Spacing")] HorizontalSpacing,
    #[serde(rename="Vertical Spacing")] VerticalSpacing,
}
struct Item { members: Vec<Uuid>, bounds: Rect }
fn items(doc: &Document, ids: &[Uuid]) -> Vec<Item> {
    let mut groups: Vec<(Uuid, Vec<Uuid>)> = Vec::new();
    for id in super::transform::members(doc, ids) {
        let mut owner = id;
        let mut parent = doc.layer(id).and_then(|l| l.parent_id);
        for _ in 0..MAX_NESTING {
            let Some(p) = parent else { break; };
            if ids.contains(&p) { owner = p; }
            parent = doc.layer(p).and_then(|l| l.parent_id);
        }
        if let Some((_, members)) = groups.iter_mut().find(|(g, _)| *g == owner) {
            members.push(id);
        } else { groups.push((owner, vec![id])); }
    }
    groups.into_iter().map(|(_, members)| {
        let points: Vec<_> = members.iter().flat_map(|id| Homography::corners_of(&doc.layer(*id).unwrap().transform)).collect();
        let x = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let y = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let right = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let bottom = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        Item { members, bounds: Rect { x, y, width: right-x, height: bottom-y } }
    }).collect()
}
fn move_items(doc: &mut Document, items: &[Item], offsets: Vec<Point>) -> Result<(), CommandError> {
    // Validate every destination before mutating any layer, including direct callers.
    let mut moves = Vec::new();
    for (item, offset) in items.iter().zip(offsets) {
        let (dx, dy) = (offset.x.round(), offset.y.round());
        if dx == 0. && dy == 0. { continue; }
        for id in &item.members {
            let mut t = doc.layer(*id).unwrap().transform;
            t.origin.x += dx; t.origin.y += dy;
            if !t.is_valid() { return Err(CommandError::Argument("alignment out of range".into())); }
            moves.push((*id, t));
        }
    }
    for (id, t) in moves { super::transform::set_transform(doc, id, t)?; }
    Ok(())
}
pub fn align(doc: &mut Document, ids: &[Uuid], alignment: LayerAlignment) -> Result<(), CommandError> {
    let items = items(doc, ids);
    if items.is_empty() { return Ok(()); }
    let target = doc.selection.as_ref().filter(|s| !s.is_empty()).and_then(|s| s.bounds()).unwrap_or_else(|| {
        if items.len() == 1 { return Rect { x:0., y:0., width:doc.width as f64, height:doc.height as f64 }; }
        let x = items.iter().map(|i| i.bounds.x).fold(f64::INFINITY, f64::min);
        let y = items.iter().map(|i| i.bounds.y).fold(f64::INFINITY, f64::min);
        let right = items.iter().map(|i| i.bounds.x+i.bounds.width).fold(f64::NEG_INFINITY, f64::max);
        let bottom = items.iter().map(|i| i.bounds.y+i.bounds.height).fold(f64::NEG_INFINITY, f64::max);
        Rect { x, y, width:right-x, height:bottom-y }
    });
    let offsets = items.iter().map(|i| {
        let b = i.bounds;
        let (x,y) = match alignment {
            LayerAlignment::Left => (target.x-b.x,0.),
            LayerAlignment::HorizontalCenter => (target.x+target.width/2.-b.x-b.width/2.,0.),
            LayerAlignment::Right => (target.x+target.width-b.x-b.width,0.),
            LayerAlignment::Top => (0.,target.y-b.y),
            LayerAlignment::VerticalCenter => (0.,target.y+target.height/2.-b.y-b.height/2.),
            LayerAlignment::Bottom => (0.,target.y+target.height-b.y-b.height),
        }; Point { x, y }
    }).collect();
    move_items(doc, &items, offsets)
}
pub fn distribute(doc: &mut Document, ids: &[Uuid], distribution: LayerDistribution) -> Result<(), CommandError> {
    let items = items(doc, ids);
    if items.len() < 3 { return Ok(()); }
    let horizontal = matches!(distribution, LayerDistribution::HorizontalCenters | LayerDistribution::HorizontalSpacing);
    let start = |b: Rect| if horizontal { b.x } else { b.y };
    let length = |b: Rect| if horizontal { b.width } else { b.height };
    let center = |b: Rect| start(b)+length(b)/2.;
    let mut order: Vec<_> = (0..items.len()).collect();
    order.sort_by(|a,b| center(items[*a].bounds).total_cmp(&center(items[*b].bounds)));
    let first = items[order[0]].bounds;
    let last = items[*order.last().unwrap()].bounds;
    let steps = (items.len()-1) as f64;
    let gap = (start(last)+length(last)-start(first)-items.iter().map(|i| length(i.bounds)).sum::<f64>())/steps;
    let mut position = start(first);
    let mut offsets = vec![Point { x:0., y:0. }; items.len()];
    for (rank,index) in order.into_iter().enumerate() {
        let b = items[index].bounds;
        let delta = if matches!(distribution, LayerDistribution::HorizontalCenters | LayerDistribution::VerticalCenters) {
            center(first)+(center(last)-center(first))*rank as f64/steps-center(b)
        } else { position-start(b) };
        offsets[index] = if horizontal { Point { x:delta, y:0. } } else { Point { x:0., y:delta } };
        position += length(b)+gap;
    }
    move_items(doc, &items, offsets)
}

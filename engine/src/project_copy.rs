//! Layer drags between project tabs, including a fitted new canvas.
use crate::*;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(serde::Serialize)]
pub struct ProjectCopy { pub document: Uuid, pub selected: Vec<Uuid> }

fn union(rects: impl Iterator<Item=Rect>) -> Option<Rect> {
    rects.reduce(|a,b| {let x=a.x.min(b.x);let y=a.y.min(b.y);Rect{x,y,width:a.max_x().max(b.max_x())-x,height:a.max_y().max(b.max_y())-y}})
}

impl Engine {
    pub fn copy_layers_to_project(&mut self,source:Uuid,ids:&[Uuid],destination:Option<Uuid>,at:Option<Point>)->Result<ProjectCopy,CommandError>{
        if ids.is_empty()||destination==Some(source){return Err(CommandError::Argument("Choose layers and a different project".into()));}
        if at.is_some_and(|p| !p.x.is_finite()||!p.y.is_finite()){return Err(CommandError::Argument("Invalid drop position".into()));}
        let session=self.session(source)?;
        if session.preview.is_some()||session.floating.is_some(){return Err(CommandError::Refused("Apply or cancel the current edit before copying layers".into()));}
        let original=&session.document;
        let mut included=HashSet::new();
        for id in ids {original.layer(*id).ok_or(CommandError::NoLayer)?;included.insert(*id);included.extend(original.descendants(*id));}
        let mut copies:Vec<_>=original.layers.iter().filter(|l|included.contains(&l.id)).cloned().collect();
        for layer in &mut copies {
            if layer.parent_id.is_some_and(|id|!included.contains(&id)){layer.parent_id=None;}
            if layer.mask_source_id.is_some_and(|id|!included.contains(&id)){
                if !layer.is_adjustment(){
                    let baked=ops::hierarchy::bake_clip(original,layer.id).ok_or_else(||CommandError::Refused("The omitted clipping source could not be baked".into()))?;
                    layer.set_pixels(Some(baked));layer.extra.gif=None;
                }
                layer.mask_source_id=None;
            }
        }
        let fitted=if destination.is_none(){union(copies.iter().filter(|l|!l.is_group&&!l.is_adjustment()&&(l.pixels.is_some()||l.extra.text.is_some())).map(|l|l.transform.bounds()))
            .map(|r|{let x=r.x.floor();let y=r.y.floor();Rect{x,y,width:r.max_x().ceil()-x,height:r.max_y().ceil()-y}})}else{None};
        let (width,height)=if let Some(id)=destination {
            let target=self.session(id)?;
            if target.preview.is_some()||target.floating.is_some(){return Err(CommandError::Refused("Apply or cancel the destination edit first".into()));}
            (target.document.width,target.document.height)
        }else{fitted.map_or((original.width,original.height),|r|(r.width.clamp(1.,30000.)as u32,r.height.clamp(1.,30000.)as u32))};
        let pictured=union(copies.iter().filter(|l|!l.is_group).map(|l|Rect{x:l.transform.origin.x,y:l.transform.origin.y,width:l.transform.size.width,height:l.transform.size.height}));
        let anchor=fitted.map(|r|Point{x:r.x+r.width/2.,y:r.y+r.height/2.}).unwrap_or_else(||{
            if ids.len()>1 {if let Some(r)=pictured{return Point{x:r.x+r.width/2.,y:r.y+r.height/2.};}}
            let t=original.layer(ids[0]).unwrap().transform;Point{x:t.origin.x+t.size.width/2.,y:t.origin.y+t.size.height/2.}
        });
        let center=if fitted.is_some(){Point{x:width as f64/2.,y:height as f64/2.}}else{at.unwrap_or(Point{x:width as f64/2.,y:height as f64/2.})};
        let offset=Point{x:center.x-anchor.x,y:center.y-anchor.y};
        let mapping:HashMap<_,_>=copies.iter().map(|l|(l.id,Uuid::new_v4())).collect();
        for layer in &mut copies {
            layer.id=mapping[&layer.id];layer.parent_id=layer.parent_id.and_then(|id|mapping.get(&id).copied());layer.mask_source_id=layer.mask_source_id.and_then(|id|mapping.get(&id).copied());
            layer.transform.origin.x+=offset.x;layer.transform.origin.y+=offset.y;
            if let Some(placement)=layer.mask.as_mut().and_then(|m|m.placement.as_mut()){placement.origin.x+=offset.x;placement.origin.y+=offset.y;}
        }
        let selected:Vec<_>=ids.iter().map(|id|mapping[id]).collect();
        let mut next=destination.map(|id|self.document(id).unwrap().clone()).unwrap_or_else(||Document::new(width,height));
        next.layers.extend(copies);next.active_layer_id=Some(selected[0]);next.selection=None;
        ops::hierarchy::validate(&next)?;animation::validate_document(&next)?;
        let target=match destination{Some(id)=>id,None=>self.new_document(width,height,false)?};
        if let Err(error)=self.edit(target,move|doc,_|{*doc=next;Ok(Dirty::everything())}){
            if destination.is_none(){self.close_document(target);}return Err(error);
        }
        Ok(ProjectCopy{document:target,selected})
    }
}

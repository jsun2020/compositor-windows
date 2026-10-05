//! BrushStroke.swift v1.4.5: hard dabs accumulate by lighten, soft dabs
//! by screen, then the whole stroke is blended once at the chosen opacity.
use crate::*;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrushSpec {
    pub diameter: f64, pub hardness: f64, pub opacity: f64,
    pub points: Vec<Point>, pub color: [f64; 4], pub erasing: bool,
    #[serde(default)] pub operation: BrushOperation,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag="kind")]
pub enum BrushOperation {
    #[default] Paint,
    Blur { radius: f64 },
    Clone { offset: Point, #[serde(rename="allLayers")] all_layers: bool },
    Heal { mode: HealingMode, seed: u32 },
}
impl BrushSpec {
    fn check(&self) -> Result<(), CommandError> {
        if !self.diameter.is_finite() || !(1.0..=2100.0).contains(&self.diameter) || !self.hardness.is_finite() || !(0.0..=1.0).contains(&self.hardness)
            || !self.opacity.is_finite() || !(0.01..=1.0).contains(&self.opacity) || self.color.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || self.points.is_empty() || self.points.len() > 100_000 || self.points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > SELECTION_COORDINATE_LIMIT || p.y.abs() > SELECTION_COORDINATE_LIMIT) {
            return Err(CommandError::Argument("brush settings out of range".into()));
        }
        match &self.operation {
            BrushOperation::Blur { radius } if !radius.is_finite() || !(0.5..=50.0).contains(radius) => return Err(CommandError::Argument("blur radius out of range".into())),
            BrushOperation::Clone { offset, .. } if !offset.x.is_finite() || !offset.y.is_finite() || offset.x.abs()>SELECTION_COORDINATE_LIMIT || offset.y.abs()>SELECTION_COORDINATE_LIMIT => return Err(CommandError::Argument("clone offset out of range".into())),
            _ => {},
        }
        if self.points.windows(2).map(|p|(p[1].x-p[0].x).hypot(p[1].y-p[0].y)).sum::<f64>()>2_000_000.0 {
            return Err(CommandError::Argument("the stroke is too long".into()));
        }
        Ok(())
    }
    fn bounds(&self, doc: &Document) -> Option<Rect> {
        let r = self.diameter / 2.0;
        let x = self.points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min).floor() - r - 1.0;
        let y = self.points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min).floor() - r - 1.0;
        let right = (self.points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max).ceil() + r + 1.0).min(doc.width as f64);
        let bottom = (self.points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max).ceil() + r + 1.0).min(doc.height as f64);
        let (x,y)=(x.max(0.0),y.max(0.0));
        (right > x && bottom > y).then_some(Rect { x,y,width:right-x,height:bottom-y })
    }
}

pub fn grid_for(doc: &Document, layer: &Layer, area: Rect) -> Result<ops::raster_edit::EditGrid, CommandError> {
    let (ow,oh)=ops::raster_edit::layer_grid(layer);
    let inverse=layer.transform.pixel_to_document(ow,oh).invert().ok_or(ProjectError::Invalid)?;
    let corners=[Point{x:area.x,y:area.y},Point{x:area.max_x(),y:area.y},Point{x:area.max_x(),y:area.max_y()},Point{x:area.x,y:area.max_y()}].map(|p|inverse.apply(p));
    let base = layer.pixels.is_some();
    let snap=ops::raster_edit::snap_near_int;
    let x0=snap(corners.iter().map(|p|p.x).fold(if base {0.0}else{f64::INFINITY},f64::min)).floor();
    let y0=snap(corners.iter().map(|p|p.y).fold(if base {0.0}else{f64::INFINITY},f64::min)).floor();
    let x1=snap(corners.iter().map(|p|p.x).fold(if base {ow as f64}else{f64::NEG_INFINITY},f64::max)).ceil();
    let y1=snap(corners.iter().map(|p|p.y).fold(if base {oh as f64}else{f64::NEG_INFINITY},f64::max)).ceil();
    let (w,h)=(x1-x0,y1-y0);
    if w>MAX_SIDE as f64 || h>MAX_SIDE as f64 || w*h>MAX_PIXELS.saturating_sub(doc.used_pixels().saturating_sub(layer.pixels.as_ref().map_or(0,|r|r.width as u64*r.height as u64))) as f64 {return Err(ProjectError::TooLarge.into());}
    let transform=ops::adjust::placed_like(&layer.transform,ow,oh,w as u32,h as u32,-x0,-y0);
    Ok(ops::raster_edit::EditGrid{width:w as u32,height:h as u32,x:if base{(-x0) as u32}else{0},y:if base{(-y0) as u32}else{0},transform})
}

/// BrushStroke.swift's settled centripetal Catmull-Rom path, including its final tail.
/// Two-pixel subdivisions feed the same arc-length dab spacing as a straight stroke.
fn curved_path(points: &[Point]) -> Vec<Point> {
    let mut out=vec![points[0]];
    let mix=|a:Point,b:Point,ta:f64,tb:f64,t:f64|Point{x:a.x*(tb-t)/(tb-ta)+b.x*(t-ta)/(tb-ta),y:a.y*(tb-t)/(tb-ta)+b.y*(t-ta)/(tb-ta)};
    for i in 0..points.len().saturating_sub(1) {
        let (p0,p1,p2,p3)=(points[i.saturating_sub(1)],points[i],points[i+1],points[(i+2).min(points.len()-1)]);
        let knot=|a:Point,b:Point|(b.x-a.x).hypot(b.y-a.y).sqrt().max(0.0001);
        let t0=0.0;let t1=knot(p0,p1);let t2=t1+knot(p1,p2);let t3=t2+knot(p2,p3);
        let pieces=((p2.x-p1.x).hypot(p2.y-p1.y)/2.0).ceil().max(1.0) as usize;
        for j in 1..=pieces {
            let t=t1+(t2-t1)*j as f64/pieces as f64;
            let a1=mix(p0,p1,t0,t1,t);let a2=mix(p1,p2,t1,t2,t);let a3=mix(p2,p3,t2,t3,t);
            let b1=mix(a1,a2,t0,t2,t);let b2=mix(a2,a3,t1,t3,t);
            out.push(if j==pieces {p2}else{mix(b1,b2,t1,t2,t)});
        }
    }
    out
}

/// Coverage is allocated only over the stroke footprint on the target grid.
/// Blank margins of a 100 MP canvas cost no brush-tip storage.
pub fn coverage(spec: &BrushSpec, doc: &Document, clips: &SelectionClips, transform: LayerTransform, w: u32,h: u32) -> (PixelRect, Vec<f32>) {
    let to_doc=transform.pixel_to_document(w,h); let inverse=to_doc.invert().unwrap();
    let area=spec.bounds(doc).unwrap();
    let c=[Point{x:area.x,y:area.y},Point{x:area.max_x(),y:area.y},Point{x:area.max_x(),y:area.max_y()},Point{x:area.x,y:area.max_y()}].map(|p|inverse.apply(p));
    let x=c.iter().map(|p|p.x).fold(f64::INFINITY,f64::min).floor().max(0.0).min(w as f64) as u32;
    let y=c.iter().map(|p|p.y).fold(f64::INFINITY,f64::min).floor().max(0.0).min(h as f64) as u32;
    let right=c.iter().map(|p|p.x).fold(f64::NEG_INFINITY,f64::max).ceil().max(0.0).min(w as f64) as u32;
    let bottom=c.iter().map(|p|p.y).fold(f64::NEG_INFINITY,f64::max).ceil().max(0.0).min(h as f64) as u32;
    let rect=PixelRect{x,y,width:right.saturating_sub(x),height:bottom.saturating_sub(y)};
    let mut out=vec![0.0f32;rect.width as usize*rect.height as usize];
    let radius=spec.diameter/2.0;
    let spacing=(spec.diameter*if spec.hardness>=1.0{0.015}else{0.025}).max(0.25);
    let path=curved_path(&spec.points);
    let mut dabs=vec![spec.points[0]]; let mut next=spacing;
    for pair in path.windows(2) {
        let (a,b)=(pair[0],pair[1]); let (dx,dy)=(b.x-a.x,b.y-a.y);let len=dx.hypot(dy);
        if len==0.0 {continue;}
        while next<=len {dabs.push(Point{x:a.x+dx*next/len,y:a.y+dy*next/len});next+=spacing;}
        next-=len;
    }
    for dab in dabs {
        let corners=[Point{x:dab.x-radius-1.0,y:dab.y-radius-1.0},Point{x:dab.x+radius+1.0,y:dab.y-radius-1.0},Point{x:dab.x+radius+1.0,y:dab.y+radius+1.0},Point{x:dab.x-radius-1.0,y:dab.y+radius+1.0}].map(|p|inverse.apply(p));
        let x0=corners.iter().map(|p|p.x).fold(f64::INFINITY,f64::min).floor().max(x as f64) as u32;
        let y0=corners.iter().map(|p|p.y).fold(f64::INFINITY,f64::min).floor().max(y as f64) as u32;
        let x1=corners.iter().map(|p|p.x).fold(f64::NEG_INFINITY,f64::max).ceil().min(right as f64).max(0.0) as u32;
        let y1=corners.iter().map(|p|p.y).fold(f64::NEG_INFINITY,f64::max).ceil().min(bottom as f64).max(0.0) as u32;
        for yy in y0..y1 {for xx in x0..x1 {
            let value=if spec.hardness>=1.0 {
                let mut hit=0u32;
                for sy in 0..4 {for sx in 0..4 {let p=to_doc.apply(Point{x:xx as f64+(sx as f64+0.5)/4.0,y:yy as f64+(sy as f64+0.5)/4.0});if (p.x-dab.x).hypot(p.y-dab.y)<=radius {hit+=1;}}}
                hit as f32/16.0
            }else{
                let p=to_doc.apply(Point{x:xx as f64+0.5,y:yy as f64+0.5});let d=(p.x-dab.x).hypot(p.y-dab.y)/radius;
                let u=((d-spec.hardness)/(1.0-spec.hardness)).clamp(0.0,1.0);
                ((-2.5*u*u).exp()-(-2.5f64).exp()).max(0.0) as f32/(1.0-(-2.5f64).exp()) as f32
            };
            let at=((yy-y)*rect.width+xx-x) as usize;
            out[at]=if spec.hardness>=1.0 {out[at].max(value)}else{out[at]+value-out[at]*value};
        }}
    }
    let clip=clips.clip(doc);
    for yy in 0..rect.height {for xx in 0..rect.width {
        let p=to_doc.apply(Point{x:(rect.x+xx) as f64+0.5,y:(rect.y+yy) as f64+0.5});
        let at=(yy*rect.width+xx) as usize;
        out[at]*=if p.x<0.0 || p.y<0.0 || p.x>=doc.width as f64 || p.y>=doc.height as f64 {0.0}else{clip.as_ref().map_or(1.0,|c|c.at(p))};
    }}
    (rect,out)
}

pub fn paint(doc: &mut Document, clips: &SelectionClips, id: Uuid, mask: bool, spec: &BrushSpec) -> Result<Dirty,CommandError> {
    spec.check()?; ops::raster_edit::check_target(doc,id,mask)?;
    let Some(area)=spec.bounds(doc) else {return Ok(Dirty::pixels(vec![]));};
    let original=doc.layer(id).ok_or(CommandError::NoLayer)?.clone();
    if matches!(spec.operation,BrushOperation::Heal { .. }|BrushOperation::Clone { .. }) && mask {return Err(CommandError::Refused("Select a pixel layer for this tool".into()));}
    if !matches!(spec.operation,BrushOperation::Paint|BrushOperation::Clone{all_layers:true,..}) && original.pixels.is_none() && !mask {return Err(CommandError::Refused("The layer has no pixels".into()));}
    if mask {
        let mut grid_doc=doc.clone();
        grid_doc.selection=Some(Selection::new(vec![selection::geometry::rectangle(area)],false,0.0));
        let grid=ops::raster_edit::mask_grid(&grid_doc,id,matches!(spec.operation,BrushOperation::Paint))?;
        let mut bytes=ops::raster_edit::mask_on_grid(original.mask.as_ref().unwrap(),&grid);
        let (rect,cov)=coverage(spec,doc,clips,grid.transform,grid.width,grid.height);
        let target=if spec.erasing {255.0}else{spec.color[0]*255.0};
        let blurred=if let BrushOperation::Blur { radius }=spec.operation {
            let mapping=grid.transform.pixel_to_document(grid.width,grid.height);
            let per=(mapping.a*mapping.d-mapping.b*mapping.c).abs().sqrt().max(1e-6);
            Some(ops::masks::blur_gray(&GrayRaster::from_bytes(grid.width,grid.height,bytes.clone()),(radius/per).min(grid.width.max(grid.height) as f64/2.0)))
        } else {None};
        if cov.iter().all(|&v|v==0.0){return Ok(Dirty::pixels(vec![]));}
        for y in 0..rect.height {for x in 0..rect.width {
            let a=cov[(y*rect.width+x) as usize] as f64*spec.opacity*spec.color[3];let at=((y+rect.y)*grid.width+x+rect.x) as usize;
            let target=blurred.as_ref().map_or(target,|m|m.bytes()[at] as f64);
            bytes[at]=(bytes[at] as f64*(1.0-a)+target*a).round().clamp(0.0,255.0) as u8;
        }}
        let m=doc.layer_mut(id).unwrap().mask_mut().unwrap();m.pixels=GrayRaster::from_bytes(grid.width,grid.height,bytes);m.placement=grid.placement;
        let dirty=Dirty::pixels(vec![id]);
        return Ok(if original.mask.as_ref().is_some_and(|m|m.pixels.width==grid.width&&m.pixels.height==grid.height){dirty.within(vec![Region{layer:id,plane:Plane::Mask,rect}])}else{dirty});
    }
    let grid=grid_for(doc,&original,area)?;let mut bytes=ops::raster_edit::pixels_on(&grid,original.pixels.as_ref());
    let (rect,cov)=coverage(spec,doc,clips,grid.transform,grid.width,grid.height);
    if cov.iter().all(|&v|v==0.0){return Ok(Dirty::pixels(vec![]));}
    if let BrushOperation::Heal { mode, seed }=spec.operation {
        let mut cover=vec![0;grid.width as usize*grid.height as usize];
        for y in 0..rect.height {for x in 0..rect.width {cover[((y+rect.y)*grid.width+x+rect.x) as usize]=(cov[(y*rect.width+x) as usize]*255.0).round() as u8;}}
        let healed=crate::retouch::heal(&Raster::from_premultiplied(grid.width,grid.height,bytes),&GrayRaster::from_bytes(grid.width,grid.height,cover),spec.opacity as f32,mode,seed)?;
        let target=doc.layer_mut(id).unwrap();target.set_pixels(Some(healed));target.transform=grid.transform;
        if let Some(mask)=&original.mask {if mask.placement.is_none() {target.mask_mut().unwrap().pixels=ops::raster_edit::followed(&mask.pixels,ops::raster_edit::layer_grid(&original),&grid,(0,0,grid.width,grid.height));}}
        let dirty=Dirty::pixels(vec![id]);
        return Ok(if original.pixels.as_ref().is_some_and(|p|p.width==grid.width&&p.height==grid.height){dirty.within(vec![Region{layer:id,plane:Plane::Pixels,rect}])}else{dirty});
    }
    let sample=match &spec.operation {
        BrushOperation::Blur { radius }=>{
            let own=original.pixels.as_ref().unwrap();let map=original.transform.pixel_to_document(own.width,own.height);
            let sigma=(radius/(map.a*map.d-map.b*map.c).abs().sqrt().max(1e-6)).min(own.width.max(own.height) as f64/2.0);
            // Blur only the source piece reached by this stroke, with a full kernel halo.
            // This has the same transparent boundary as the complete padded layer.
            let inv=map.invert().unwrap();
            let c=[Point{x:area.x,y:area.y},Point{x:area.max_x(),y:area.y},Point{x:area.max_x(),y:area.max_y()},Point{x:area.x,y:area.max_y()}].map(|p|inv.apply(p));
            let pad=(3.0*sigma).ceil() as i64+2;
            let sx0=c.iter().map(|p|p.x).fold(f64::INFINITY,f64::min).floor() as i64-pad;
            let sy0=c.iter().map(|p|p.y).fold(f64::INFINITY,f64::min).floor() as i64-pad;
            let sx1=c.iter().map(|p|p.x).fold(f64::NEG_INFINITY,f64::max).ceil() as i64+pad;
            let sy1=c.iter().map(|p|p.y).fold(f64::NEG_INFINITY,f64::max).ceil() as i64+pad;
            let(w,h)=((sx1-sx0) as u32,(sy1-sy0) as u32);
            if w as i64>MAX_SIDE||h as i64>MAX_SIDE||w as u64*h as u64>MAX_PIXELS{return Err(ProjectError::TooLarge.into());}
            let mut patch=vec![0;w as usize*h as usize*4];
            let x0=sx0.max(0);let x1=sx1.min(own.width as i64);
            if x1>x0 {for y in sy0.max(0)..sy1.min(own.height as i64){let from=(y*own.width as i64+x0) as usize*4;let to=((y-sy0)*w as i64+x0-sx0) as usize*4;let n=(x1-x0) as usize*4;patch[to..to+n].copy_from_slice(&own.bytes()[from..from+n]);}}
            let placed=ops::adjust::placed_like(&original.transform,own.width,own.height,w,h,-(sx0 as f64),-(sy0 as f64));
            Some((adjust::filters::gaussian_blur(&Raster::from_premultiplied(w,h,patch),sigma),placed,Point{x:0.0,y:0.0}))
        },
        BrushOperation::Clone { offset, all_layers }=>{
            if *all_layers {Some((compositor::composite(doc,Rect{x:0.0,y:0.0,width:doc.width as f64,height:doc.height as f64},doc.width,doc.height),LayerTransform::axis_aligned(Point{x:0.0,y:0.0},doc.size()),*offset))}
            else {Some((original.pixels.as_ref().unwrap().clone(),original.transform,*offset))}
        },
        _=>None,
    };
    let sample_map=sample.as_ref().map(|(p,t,_)|t.pixel_to_document(p.width,p.height).invert().unwrap());
    let to_doc=grid.transform.pixel_to_document(grid.width,grid.height);
    for y in 0..rect.height {for x in 0..rect.width {
        let a=cov[(y*rect.width+x) as usize] as f64*spec.opacity*spec.color[3];let at=(((y+rect.y)*grid.width+x+rect.x)*4) as usize;
        if let Some((sample,_,offset))=&sample {
            let p=to_doc.apply(Point{x:(x+rect.x) as f64+0.5,y:(y+rect.y) as f64+0.5});let p=sample_map.unwrap().apply(Point{x:p.x+offset.x,y:p.y+offset.y});
            if p.x<0.0||p.y<0.0||p.x>=sample.width as f64||p.y>=sample.height as f64 {continue;}
            let px=compositor::sample(sample,p.x,p.y,original.transform.sampling==Sampling::Nearest);
            for k in 0..4 {bytes[at+k]=(px[k] as f64*a*255.0+bytes[at+k] as f64*(1.0-px[3] as f64*a)).round().clamp(0.0,255.0) as u8;}
        }
        else if spec.erasing {for v in &mut bytes[at..at+4] {*v=(*v as f64*(1.0-a)).round() as u8;}}
        else {for k in 0..4 {let v=if k==3 {1.0}else{spec.color[k]};bytes[at+k]=(v*a*255.0+bytes[at+k] as f64*(1.0-a)).round().clamp(0.0,255.0) as u8;}}
    }}
    let target=doc.layer_mut(id).unwrap();target.set_pixels(Some(Raster::from_premultiplied(grid.width,grid.height,bytes)));target.transform=grid.transform;
    if let Some(mask)=&original.mask {if mask.placement.is_none() {let old=ops::raster_edit::layer_grid(&original);target.mask_mut().unwrap().pixels=ops::raster_edit::followed(&mask.pixels,old,&grid,(0,0,grid.width,grid.height));}}
    let dirty=Dirty::pixels(vec![id]);
    Ok(if original.pixels.as_ref().is_some_and(|p|p.width==grid.width&&p.height==grid.height){dirty.within(vec![Region{layer:id,plane:Plane::Pixels,rect}])}else{dirty})
}

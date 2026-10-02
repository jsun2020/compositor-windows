use crate::*;
use uuid::Uuid;
pub fn apply(doc:&mut Document,clips:&SelectionClips,id:Uuid)->Result<Dirty,CommandError>{
    ops::raster_edit::check_target(doc,id,false)?;
    if doc.selection.is_none(){return Err(CommandError::Refused(ops::selection::NO_SELECTION.into()));}
    let layer=doc.layer(id).ok_or(CommandError::NoLayer)?.clone();
    let bounds=doc.selection.as_ref().and_then(|s|s.bounds()).ok_or_else(||CommandError::Refused(ops::selection::NO_SELECTION.into()))?;
    let (x,y)=(bounds.x.max(0.0),bounds.y.max(0.0));
    let area=Rect{x,y,width:bounds.max_x().min(doc.width as f64)-x,height:bounds.max_y().min(doc.height as f64)-y};
    if area.width<=0.0 || area.height<=0.0{return Err(CommandError::Refused(ops::selection::NO_SELECTION.into()));}
    let grid=ops::brush::grid_for(doc,&layer,area)?;
    let original=Raster::from_premultiplied(grid.width,grid.height,ops::raster_edit::pixels_on(&grid,layer.pixels.as_ref()));
    let coverage=selection_coverage_with(doc,clips,&grid.transform.pixel_to_document(grid.width,grid.height),grid.width,grid.height).ok_or_else(||CommandError::Refused(ops::selection::NO_SELECTION.into()))?;
    let filled=crate::retouch::fill(&original,&coverage)?;
    let filled=adjust::apply::blend_by_coverage(&filled,&original,&coverage);
    let target=doc.layer_mut(id).unwrap();target.set_pixels(Some(filled));target.transform=grid.transform;
    if let Some(mask)=&layer.mask {if mask.placement.is_none(){target.mask_mut().unwrap().pixels=ops::raster_edit::followed(&mask.pixels,ops::raster_edit::layer_grid(&layer),&grid,(0,0,grid.width,grid.height));}}
    let dirty=Dirty::pixels(vec![id]);
    if let Some(pixels)=&layer.pixels {if (pixels.width,pixels.height)==(grid.width,grid.height)&&layer.transform==grid.transform {
        if let Some(clip)=clips.clip(doc){if let Some(rect)=clip.rect_on_grid(&grid.transform.pixel_to_document(grid.width,grid.height),grid.width,grid.height){return Ok(dirty.within(vec![Region{layer:id,plane:Plane::Pixels,rect}]));}}
    }}
    Ok(dirty)
}

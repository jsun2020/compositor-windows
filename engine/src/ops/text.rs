//! LayerTextStyle v1.4.5. Windows supplies the platform-font raster alongside
//! this editable source; the package carries both so other platforms can redraw.
use crate::*;
use serde::{Deserialize,Serialize};
use uuid::Uuid;
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct TextColorRun {pub location:usize,pub length:usize,pub red:f64,pub green:f64,pub blue:f64,#[serde(flatten)]pub unknown:serde_json::Map<String,serde_json::Value>}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct TextFontRun {pub location:usize,pub length:usize,pub font_name:String,#[serde(flatten)]pub unknown:serde_json::Map<String,serde_json::Value>}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(default,rename_all="camelCase")]
pub struct TextStyle {
    pub content:String,pub font_name:String,pub font_size:f64,pub red:f64,pub green:f64,pub blue:f64,
    pub alignment:String,pub tracking:f64,pub leading:f64,
    #[serde(skip_serializing_if="Option::is_none")]pub box_size:Option<Size>,
    #[serde(skip_serializing_if="Option::is_none")]pub color_runs:Option<Vec<TextColorRun>>,
    #[serde(skip_serializing_if="Option::is_none")]pub font_runs:Option<Vec<TextFontRun>>,
    #[serde(flatten)]pub unknown:serde_json::Map<String,serde_json::Value>,
}
impl Default for TextStyle {fn default()->Self{Self{content:"Text".into(),font_name:"Arial".into(),font_size:72.0,red:0.0,green:0.0,blue:0.0,alignment:"Left".into(),tracking:0.0,leading:0.0,box_size:None,color_runs:None,font_runs:None,unknown:Default::default()}}}
impl TextStyle {
    pub fn is_valid(&self)->bool{
        let n=self.content.encode_utf16().count();let unit=|v:f64|v.is_finite()&&(0.0..=1.0).contains(&v);
        let font=|s:&str|!s.is_empty()&&s.chars().count()<=200&&!s.contains(['\r','\n']);
        if n>100_000||!font(&self.font_name)||!self.font_size.is_finite()||!(1.0..=2000.0).contains(&self.font_size)||![self.red,self.green,self.blue].into_iter().all(unit)||!self.tracking.is_finite()||!(-100.0..=1000.0).contains(&self.tracking)||!self.leading.is_finite()||!(0.0..=5000.0).contains(&self.leading)||!["Left","Center","Right"].contains(&self.alignment.as_str()){return false;}
        if self.box_size.is_some_and(|s|!s.width.is_finite()||!s.height.is_finite()||!(16.0..=MAX_SIDE as f64).contains(&s.width)||!(16.0..=MAX_SIDE as f64).contains(&s.height)||s.width*s.height>MAX_PIXELS as f64){return false;}
        if let Some(runs)=&self.color_runs {let mut end=0;if runs.is_empty(){return false;}for r in runs{let Some(next)=r.location.checked_add(r.length)else{return false;};if r.length==0||r.location<end||next>n||![r.red,r.green,r.blue].into_iter().all(unit){return false;}end=next;}}
        if let Some(runs)=&self.font_runs {let mut end=0;if runs.is_empty(){return false;}for r in runs{let Some(next)=r.location.checked_add(r.length)else{return false;};if r.length==0||r.location<end||next>n||!font(&r.font_name){return false;}end=next;}}
        true
    }
}
pub fn placement(layer:&Layer,image:&Raster)->LayerTransform{
    let mut t=layer.transform;let old=layer.pixels.as_ref().unwrap();let anchor=t.unit_to_document().apply(Point{x:0.0,y:0.0});
    t.size=Size{width:image.width as f64*t.size.width/old.width as f64,height:image.height as f64*t.size.height/old.height as f64};
    let moved=t.unit_to_document().apply(Point{x:0.0,y:0.0});t.origin.x+=anchor.x-moved.x;t.origin.y+=anchor.y-moved.y;t
}
pub fn install(doc:&mut Document,target:Option<Uuid>,style:serde_json::Value,image:Raster,origin:Point)->Result<Dirty,CommandError>{
    let parsed:TextStyle=serde_json::from_value(style.clone()).map_err(|_|CommandError::Argument("invalid text style".into()))?;
    if !parsed.is_valid()||image.width==0||image.height==0||image.width as i64>MAX_SIDE||image.height as i64>MAX_SIDE||!origin.x.is_finite()||!origin.y.is_finite()||origin.x.abs()>SELECTION_COORDINATE_LIMIT||origin.y.abs()>SELECTION_COORDINATE_LIMIT{return Err(CommandError::Argument("invalid text edit".into()));}
    if target.is_none()&&parsed.content.trim().is_empty(){return Ok(Dirty::default());}
    let id=if let Some(id)=target {
        let old=doc.layer(id).ok_or(CommandError::NoLayer)?.clone();
        if old.extra.text.is_none()||old.pixels.is_none()||old.is_group||old.is_adjustment(){return Err(CommandError::Refused("That layer is no longer editable text".into()));}
        if old.extra.text.as_ref()==Some(&style){return Ok(Dirty::default());}
        let transform=placement(&old,&image);if !transform.is_valid(){return Err(ProjectError::TooLarge.into());}
        let layer=doc.layer_mut(id).unwrap();if layer.mask.as_ref().is_some_and(|m|m.placement.is_none()){layer.mask_mut().unwrap().placement=Some(old.transform);}
        layer.set_pixels(Some(image));layer.transform=transform;layer.extra.text=Some(style);id
    }else{
        let name=parsed.content.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(40).collect::<String>();
        let mut layer=Layer::with_pixels(if name.is_empty(){"Text"}else{&name},image,origin);layer.extra.text=Some(style);ops::layers::insert_above_active(doc,layer)?
    };
    if doc.used_pixels()>MAX_PIXELS||doc.used_mask_pixels()>MAX_PIXELS{return Err(ProjectError::TooLarge.into());}
    Ok(Dirty::pixels(vec![id]))
}
impl Engine {
    pub fn install_text(&mut self,doc:Uuid,target:Option<Uuid>,style:serde_json::Value,image:Raster,origin:Point,stamp:Option<LayerStamp>)->Result<Dirty,CommandError>{
        self.clear_preview(doc);
        self.edit(doc,move |d,_|{
            if let (Some(target),Some(stamp))=(target,stamp){if LayerStamp::of(d,d.layer(target).ok_or(CommandError::NoLayer)?)!=stamp{return Err(CommandError::Refused(jobs::LAYER_CHANGED.into()));}}
            install(d,target,style,image,origin)
        })
    }
}

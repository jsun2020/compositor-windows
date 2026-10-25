//! Floating point compositing surface for 16-bit and HDR documents. The established RGBA8
//! compositor remains the byte-exact path for documents containing only 8-bit layers.
use super::*;
struct Surface { data:Vec<f32>, w:u32,h:u32,region:Rect,depth:PixelDepth }
impl Surface {
    fn point(&self,x:u32,y:u32)->Point {Point{x:self.region.x+(x as f64+0.5)*self.region.width/self.w as f64,y:self.region.y+(y as f64+0.5)*self.region.height/self.h as f64}}
    fn pixel(&self,i:usize)->[f32;4] {self.data[i..i+4].try_into().unwrap()}
}
fn over(dst:[f32;4],src:[f32;4],mode:BlendMode)->[f32;4] {
    let (ad,as_)=(dst[3],src[3]);if as_<=0. {return dst;}
    if mode==BlendMode::Normal || ad<=0. {return [src[0]+dst[0]*(1.-as_),src[1]+dst[1]*(1.-as_),src[2]+dst[2]*(1.-as_),as_+ad*(1.-as_)];}
    let b=blend_rgb(mode,[dst[0]/ad,dst[1]/ad,dst[2]/ad],[src[0]/as_,src[1]/as_,src[2]/as_]);
    [src[0]*(1.-ad)+dst[0]*(1.-as_)+as_*ad*b[0],src[1]*(1.-ad)+dst[1]*(1.-as_)+as_*ad*b[1],src[2]*(1.-ad)+dst[2]*(1.-as_)+as_*ad*b[2],as_+ad*(1.-as_)]
}
fn draw(doc:&Document,plan:&RenderPlan,t:&mut Surface,d:&LayerDraw,mode:BlendMode,use_clip:bool,cache:&EffectsCache) {
    let sx=t.w as f64/t.region.width;let sy=t.h as f64/t.region.height;
    let sources=if use_clip {d.clip.map(|c|clip_source_rasters_xy(doc,plan,c,sx,sy,cache)).unwrap_or_default()}else{SourceRasters::new()};
    if let Some(a)=&d.adjustment {
        let mut input=t.data.clone();
        if a.kind.is_spatial() {for y in 0..t.h{for x in 0..t.w{let p=t.point(x,y);if p.x<0.||p.y<0.||p.x>=doc.width as f64||p.y>=doc.height as f64{let i=((y*t.w+x)*4)as usize;input[i..i+4].fill(0.);}}}}
        let original=Raster::from_premultiplied_f32(t.w,t.h,input,t.depth);
        let adjusted=if a.kind.is_spatial() {
            let b=spatial_blur(a,sx);
            match a.kind {AdjustmentKind::GaussianBlur=>gaussian_blur(&original,b.sigma),AdjustmentKind::MotionBlur=>motion_blur(&original,b.angle,b.distance),_=>a.apply_spatial_filter(&original,t.region)}
        } else {apply_adjustment(&original,a,Point{x:t.region.x,y:t.region.y},1./sx,None)};
        for y in 0..t.h {for x in 0..t.w {
            let p=t.point(x,y);if p.x<0.||p.y<0.||p.x>=doc.width as f64||p.y>=doc.height as f64 {continue;}
            let mut k=d.opacity as f32*coverages_at(doc,&d.coverages,p);
            if use_clip {if let Some(c)=d.clip {k*=source_coverage_at(doc,plan,c,p,&sources);}}
            if k<=0. {continue;}
            let i=((y*t.w+x)*4) as usize;let before=t.pixel(i);let mut after=adjusted.pixel_f32(x,y);
            if mode!=BlendMode::Normal || d.keeps_alpha {
                let straight=|p:[f32;4]|[0,1,2].map(|c|if p[3]>0. {p[c]/p[3]}else{0.});
                let b=blend_rgb(mode,straight(before),straight(after));after=[b[0]*before[3],b[1]*before[3],b[2]*before[3],before[3]];
            }
            for c in 0..4 {t.data[i+c]=before[c]+(after[c]-before[c])*k;}
        }}
        return;
    }
    let Some(raster)=draw_raster(doc,d,cache) else {return;};
    let level=if prefilters(d.transform.sampling,d.corners.is_some()) {prefilter_level(raster.width,raster.height,raster.width as f64/(d.transform.size.width*sx))}else{0};
    let source=raster.reduced_deep(level);let scale=0.5f64.powi(level as i32);let nearest=nearest_for_draw(d,sx,scale);
    for y in 0..t.h {for x in 0..t.w {
        let p=t.point(x,y);let Some(px)=to_pixels(&d.transform,d.corners.as_ref(),raster.width,raster.height,p) else {continue;};
        let mut color=sample_placed(d,&source,px,p,scale,nearest,sx,sy,true);
        if color[3]<=0. {continue;}
        let mut k=d.opacity as f32*coverages_at_scaled(doc,&d.coverages,p,sx,sy);
        if use_clip {if let Some(c)=d.clip {k*=source_coverage_at(doc,plan,c,p,&sources);}}
        if k<=0. {continue;}for v in &mut color {*v*=k;}
        let i=((y*t.w+x)*4) as usize;let result=over(t.pixel(i),color,mode);t.data[i..i+4].copy_from_slice(&result);
    }}
}
fn composite_one(doc:&Document,plan:&RenderPlan,region:Rect,w:u32,h:u32,cache:&EffectsCache)->Raster {
    let depth=if doc.layers.iter().any(|l|l.pixels.as_ref().is_some_and(|p|p.depth()==PixelDepth::Hdr)){PixelDepth::Hdr}else{PixelDepth::Sixteen};
    let mut t=Surface{data:vec![0.;w as usize*h as usize*4],w,h,region,depth};
    for node in &plan.nodes {match node {
        PlanNode::Layer{draw:d}=>draw(doc,plan,&mut t,d,d.blend,true,cache),
        PlanNode::Stack{base,children,folder_coverages}=>{
            let mut temp=Surface{data:vec![0.;t.data.len()],w,h,region,depth};
            draw(doc,plan,&mut temp,base,BlendMode::Normal,true,cache);
            let alpha:Vec<_>=temp.data.chunks_exact(4).map(|p|p[3]).collect();
            for p in temp.data.chunks_exact_mut(4) {let a=p[3];if a>0. {for v in &mut p[..3] {*v/=a;}}p[3]=1.;}
            for child in children {draw(doc,plan,&mut temp,child,child.blend,false,cache);}
            for y in 0..h {for x in 0..w {
                let at=(y*w+x) as usize;let i=at*4;let a=alpha[at];if a<=0. {continue;}
                let k=coverages_at(doc,folder_coverages,t.point(x,y));let p=temp.pixel(i);
                let result=over(t.pixel(i),[p[0]*a*k,p[1]*a*k,p[2]*a*k,a*k],base.blend);t.data[i..i+4].copy_from_slice(&result);
            }}
        }
    }}
    Raster::from_premultiplied_f32(w,h,t.data,depth)
}

/// Large deep composites keep the float working surface in row bands. The final
/// 16-bit buffer is retained; only spatial dependencies enlarge each band's halo.
pub(super) fn composite(doc:&Document,plan:&RenderPlan,region:Rect,w:u32,h:u32,cache:&EffectsCache)->Raster {
 if w as u64*h as u64<=1_000_000 {return composite_one(doc,plan,region,w,h,cache);}
 composite_banded(doc,plan,region,w,h,cache,128)
}
fn composite_banded(doc:&Document,plan:&RenderPlan,region:Rect,w:u32,h:u32,cache:&EffectsCache,rows:u32)->Raster {
 let sx=w as f64/region.width;
 let halo_of=|d:&LayerDraw|->u32 {d.adjustment.as_ref().filter(|a|a.kind.is_spatial()).map(|a|{
  (a.sampling_margin()*sx).ceil()as u32
 }).unwrap_or(0)};
 // Consecutive spatial adjustments can read one another's halos.
 let halo=plan.nodes.iter().fold(0u32,|sum,n|{let reach=match n{
  PlanNode::Layer{draw}=>halo_of(draw),PlanNode::Stack{base,children,..}=>children.iter().fold(halo_of(base),|v,c|v.saturating_add(halo_of(c))),
 };sum.saturating_add(reach).min(h)});
 let depth=if doc.layers.iter().any(|l|l.pixels.as_ref().is_some_and(|p|p.depth()==PixelDepth::Hdr)){PixelDepth::Hdr}else{PixelDepth::Sixteen};
 let mut samples=vec![0;w as usize*h as usize*4];
 for y in (0..h).step_by(rows as usize){let end=(y+rows).min(h);let top=y.saturating_sub(halo);let bottom=end.saturating_add(halo).min(h);
  let tile_region=Rect{x:region.x,y:region.y+top as f64*region.height/h as f64,width:region.width,height:(bottom-top)as f64*region.height/h as f64};
  let tile=composite_one(doc,plan,tile_region,w,bottom-top,cache);let source=&tile.deep_pixels().unwrap().samples;
  let stride=w as usize*4;let from=(y-top)as usize*stride;let count=(end-y)as usize*stride;
  samples[y as usize*stride..y as usize*stride+count].copy_from_slice(&source[from..from+count]);
 }
 Raster::from_deep(w,h,DeepPixels{depth,samples})
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn row_bands_match_whole_deep_composite_across_spatial_adjustments(){
  let mut doc=Document::new(40,31);let values:Vec<_>=(0..40*31).flat_map(|i|[(i*941%65536)as u16,(i*1783%65536)as u16,(i*311%65536)as u16,65535]).collect();
  doc.layers.push(Layer::with_pixels("Deep",Raster::from_rgba16(40,31,values),Point{x:0.,y:0.}));
  let mut a=Layer::blank("Blur",doc.size());a.extra.adjustment=Some(LayerAdjustment::new(AdjustmentKind::GaussianBlur));doc.layers.push(a);
  let plan=crate::plan::render_plan(&doc,None);let region=Rect{x:0.,y:0.,width:40.,height:31.};let cache=EffectsCache::default();
  let whole=composite_one(&doc,&plan,region,40,31,&cache);let banded=composite_banded(&doc,&plan,region,40,31,&cache,5);
  assert_eq!(whole.deep_pixels(),banded.deep_pixels());
 }
}

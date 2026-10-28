//! Mac 1.5.11 lens optics. All light operations use straight, linear RGB; the input
//! alpha is restored unchanged after the optical effect.
use crate::Raster;
use serde::{Deserialize, Serialize};
mod light;
use light::Light;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LensEffect {
    #[default] #[serde(rename="Anamorphic Flare")] AnamorphicFlare,
    Starburst, Mist, Prism, Bokeh,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BokehShape { #[default] Round, Hexagon, Oval, #[serde(rename="Cat's Eye")] CatsEye }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase", default)]
pub struct LensEffectSettings {
    pub effect: LensEffect, pub threshold: f64,
    pub flare_amount: f64, pub flare_length: f64, pub flare_blue: f64,
    pub star_amount: f64, pub star_length: f64, pub star_points: u32, pub star_angle: f64,
    pub mist_strength: f64, pub mist_size: f64,
    pub prism_amount: f64, pub prism_copies: f64, pub prism_offset: f64, pub prism_angle: f64, pub prism_spread: f64,
    pub prism_both_ways: bool, pub prism_blur: f64,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub prism_fringe: Option<f64>,
    pub bokeh_radius: f64, pub bokeh_brightness: f64, pub bokeh_ring: f64, pub bokeh_shape: BokehShape,
}
impl Default for LensEffectSettings {
    fn default()->Self { Self { effect:LensEffect::AnamorphicFlare,threshold:90.,flare_amount:50.,flare_length:60.,flare_blue:80.,
        star_amount:50.,star_length:12.,star_points:6,star_angle:15.,mist_strength:50.,mist_size:30.,
        prism_amount:80.,prism_copies:5.,prism_offset:6.,prism_angle:180.,prism_spread:70.,
        prism_both_ways:false,prism_blur:0.,prism_fringe:Some(30.),
        bokeh_radius:2.,bokeh_brightness:50.,bokeh_ring:100.,bokeh_shape:BokehShape::Round } }
}
impl LensEffectSettings {
    pub fn normalized(&self)->Self {
        let mut s=self.clone();let d=Self::default();
        let clamp=|v:f64,lo:f64,hi:f64,f:f64|if v.is_finite(){v.clamp(lo,hi)}else{f};
        macro_rules! bound {($field:ident,$lo:expr,$hi:expr)=>{s.$field=clamp(s.$field,$lo,$hi,d.$field);}}
        bound!(threshold,50.,100.);bound!(flare_amount,0.,100.);bound!(flare_length,1.,100.);bound!(flare_blue,0.,100.);
        bound!(star_amount,0.,100.);bound!(star_length,2.,40.);bound!(star_angle,-180.,180.);
        if ![4,6,8,12,14].contains(&s.star_points){s.star_points=6;}
        bound!(mist_strength,0.,100.);bound!(mist_size,0.,100.);bound!(prism_amount,0.,100.);bound!(prism_copies,1.,8.);
        s.prism_copies=s.prism_copies.round();bound!(prism_offset,0.,50.);bound!(prism_angle,-180.,180.);bound!(prism_spread,10.,100.);
        bound!(prism_blur,0.,100.);s.prism_fringe=s.prism_fringe.map(|v|clamp(v,0.,100.,30.));
        bound!(bokeh_radius,0.5,6.);bound!(bokeh_brightness,0.,100.);bound!(bokeh_ring,0.,100.);s
    }
    pub fn is_identity(&self)->bool { let s=self.normalized();match s.effect {
        LensEffect::AnamorphicFlare=>s.flare_amount==0.,LensEffect::Starburst=>s.star_amount==0.,LensEffect::Mist=>s.mist_strength==0.,
        LensEffect::Prism=>s.prism_amount==0.,LensEffect::Bokeh=>false,
    } }
}

const CURVES:&[u8]=include_bytes!("../../native/LensToneCurves.bin");
const DENSITY_CURVES:&[u8]=include_bytes!("../../native/LensDensityCurves.bin");
fn table_curve(table:&[u8],row:usize,value:f32)->f32 {
    let x=value.clamp(0.,1.)*1024.;let i=(x.floor()as usize).min(1023);let f=x-i as f32;
    let at=(row*1025+i)*4;
    let a=f32::from_le_bytes(table[at..at+4].try_into().unwrap());let b=f32::from_le_bytes(table[at+4..at+8].try_into().unwrap());a+(b-a)*f
}
fn curve(row:usize,value:f32)->f32 {table_curve(CURVES,row,value)}
pub fn highlight_curve(threshold:f64,value:f32)->f32 {
    let row=(threshold.clamp(0.5,0.95)-0.5)*400.;let low=(row.floor()as usize).min(180);let high=(low+1).min(180);
    let a=curve(low,value);a+(curve(high,value)-a)*(row-low as f64)as f32
}
fn linear(v:f32)->f32 { if v<=0.04045{v/12.92}else{((v+0.055)/1.055).powf(2.4)} }
fn gamma(v:f32)->f32 { if v<=0.0031308{v*12.92}else{1.055*v.powf(1./2.4)-0.055} }
fn highlights(image:&Light,threshold:f64)->Light { image.map(|p|{
    let mask=highlight_curve(threshold,0.2126*p[0]+0.7152*p[1]+0.0722*p[2]);p.map(|v|linear(v)*mask)
}) }
fn sources(image:&Light,threshold:f64,side:f64)->Light {
    if threshold<=0.95 {return highlights(image,threshold);}
    let density=((threshold-0.95)/0.05).min(1.);
    let lit=image.map(|p|[highlight_curve(0.95,0.2126*p[0]+0.7152*p[1]+0.0722*p[2]);3]);
    let radius=side*0.03*density;
    let around=lit.blur(radius,radius,true);
    let peak=around.pixels.iter().map(|p|p[0]).fold(0f32,f32::max);
    if peak<=0.001 {return highlights(image,0.95);}
    let row=density*100.;let low=(row.floor()as usize).min(100);let high=(low+1).min(100);
    let solid=around.map(|p| {
        let a=table_curve(DENSITY_CURVES,low,p[0]/peak);let b=table_curve(DENSITY_CURVES,high,p[0]/peak);
        [a+(b-a)*(row-low as f64)as f32;3]
    });
    let whole=solid.maximum_disc(radius);
    let mut result=image.map(|p|p.map(linear));
    for ((p,l),w) in result.pixels.iter_mut().zip(&lit.pixels).zip(&whole.pixels) {
        for c in 0..3 {p[c]*=l[0]*w[0];}
    }
    result
}
fn soft_clip(image:&Light,limit:f32)->Light {image.map(|p|p.map(|v|curve(181,v/(4.*limit))*limit))}

/// Refuse before allocating intermediates, leaving the source and undo history intact.
/// Rotated streak grids can be much larger than the source, especially for thin panoramas.
pub fn apply_checked(raster:&Raster,settings:&LensEffectSettings)->Result<Raster,crate::CommandError>{
    let s=settings.normalized();
    if s.is_identity(){return Ok(raster.clone());}
    if !working_size_allowed(raster.width,raster.height,&s) {
        return Err(crate::CommandError::Refused("Lens Effects working memory limit exceeded; reduce the layer dimensions before applying this filter".into()));
    }
    Ok(apply(raster,&s))
}

/// The same existing optical working-set ceiling applies to adjustment renders.
/// Identity settings require no optical intermediates.
pub fn working_size_allowed(width:u32,height:u32,settings:&LensEffectSettings)->bool {
    let s=settings.normalized();if s.is_identity(){return true;}
    let pixels=u64::from(width)*u64::from(height);
    let rotated=if s.effect==LensEffect::Starburst {(u64::from(width)+u64::from(height)).pow(2)/2}else{pixels};
    pixels.saturating_mul(8).saturating_add(rotated.saturating_mul(4))<=40_000_000
}

pub fn apply(raster:&Raster,settings:&LensEffectSettings)->Raster {
    apply_with_side(raster,settings,raster.width.max(raster.height)as f64)
}
pub fn apply_with_side(raster:&Raster,settings:&LensEffectSettings,side:f64)->Raster {
    let s=settings.normalized();if s.is_identity(){return raster.clone();}
    let image=Light::from_raster(raster);
    let result=match s.effect {
        LensEffect::Mist=>mist(&image,&s,side),LensEffect::Prism=>prism(&image,&s,side),
        LensEffect::AnamorphicFlare=>flare(&image,&s,side),LensEffect::Starburst=>starburst(&image,&s,side),
        LensEffect::Bokeh=>bokeh(&image,&s,side),
    };
    drop(image);
    if raster.depth()==crate::PixelDepth::Eight {
        let mut bytes=Vec::with_capacity(raster.width as usize*raster.height as usize*4);
        for (i,p) in result.pixels.iter().enumerate() {
            let alpha=raster.bytes()[i*4+3];let a=alpha as f32/255.;
            for &v in p {bytes.push((gamma(v.max(0.))*a*255.).round().clamp(0.,255.)as u8);}
            bytes.push(alpha);
        }
        return Raster::from_premultiplied(raster.width,raster.height,bytes);
    }
    let mut values=Vec::with_capacity(raster.width as usize*raster.height as usize*4);
    for (i,p) in result.pixels.iter().enumerate(){let alpha=raster.pixel_f32(i as u32%raster.width,i as u32/raster.width)[3];
        values.extend([gamma(p[0].max(0.))*alpha,gamma(p[1].max(0.))*alpha,gamma(p[2].max(0.))*alpha,alpha]);}
    Raster::from_premultiplied_f32(raster.width,raster.height,values,raster.depth())
}
fn mist(image:&Light,s:&LensEffectSettings,side:f64)->Light {
    let light=image.map(|p|p.map(linear));let scale=side*(0.2+s.mist_size/100.*1.8)/100.;
    let near=light.blur(scale*0.6,scale*0.6,true);let far=light.blur(scale*4.,scale*4.,true);
    let glow=highlights(image,0.75).blur(scale*1.5,scale*1.5,true);
    let strength=s.mist_strength as f32/100.;let scatter=strength*0.4;
    let mut result=near.scaled([scatter*0.6;3]);result.add_scaled(&far,[scatter*0.4;3]);result.add_scaled(&glow,[strength*0.35;3]);
    result.add_scaled(&light,[1.-scatter;3]);result
}
fn prism(image:&Light,s:&LensEffectSettings,side:f64)->Light {
    let theta=s.prism_angle.to_radians();let dir=[theta.cos(),-theta.sin()];
    let shift=side*s.prism_offset/100.;let fringe=side*s.prism_fringe.unwrap_or(30.)/10_000.;
    let ways:&[f64]=if s.prism_both_ways {&[1.,-1.]}else{&[1.]};
    let mut result=image.map(|p|p.map(linear));
    for copy in (1..=s.prism_copies as u32).rev() {
        let step=shift*copy as f64;
        let fade=0.78f32.powi(copy as i32-1)*s.prism_amount as f32/100.;
        if s.prism_blur==0. {
            // With no blur, translated pixels outside the canvas cannot affect
            // the output. Sample only the destination instead of materializing
            // an expanded ghost (up to many canvas widths with Both Ways).
            for y in 0..image.height {for x in 0..image.width {for c in 0..3 {
                let mut ghost=0f32;
                for way in ways {
                    let along=step+match c {0=>fringe,2=>-fringe,_=>0.};
                    let sx=x as f64-dir[0]*along*way;let sy=y as f64-dir[1]*along*way;
                    if sx>=-1. && sy>=-1. && sx<image.width as f64 && sy<image.height as f64 {
                        ghost=ghost.max(linear(image.sample(sx,sy,true)[c]));
                    }
                }
                let v=&mut result.pixels[y*image.width+x][c];*v=v.max(ghost*fade).min(1.);
            }}}
            continue;
        }
        // Blur the union of translated image extents before cropping back to the
        // canvas. Cropping first changes halos at the edge of the document.
        let shifts:Vec<_>=ways.iter().flat_map(|way|[step+fringe,step,step-fringe].map(|d|[dir[0]*d*way,dir[1]*d*way])).collect();
        let ox=shifts.iter().map(|p|p[0]).fold(f64::INFINITY,f64::min).floor();
        let oy=shifts.iter().map(|p|p[1]).fold(f64::INFINITY,f64::min).floor();
        let right=shifts.iter().map(|p|p[0]).fold(f64::NEG_INFINITY,f64::max).ceil()+image.width as f64;
        let bottom=shifts.iter().map(|p|p[1]).fold(f64::NEG_INFINITY,f64::max).ceil()+image.height as f64;
        let gw=(right-ox)as usize;let gh=(bottom-oy)as usize;
        let left=(-ox).max(0.)as usize;let top=(-oy).max(0.)as usize;
        let end_x=(image.width as f64-ox).min(gw as f64).max(0.)as usize;
        let end_y=(image.height as f64-oy).min(gh as f64).max(0.)as usize;
        if left>=end_x || top>=end_y {continue;}
        let pixel=|x:usize,y:usize| {
            let mut out=[0f32;3];
            for way in ways {for c in 0..3 {
                let along=step+match c{0=>fringe,2=>-fringe,_=>0.};
                let sx=x as f64+ox-dir[0]*along*way;let sy=y as f64+oy-dir[1]*along*way;
                if sx>=-1. && sy>=-1. && sx<image.width as f64 && sy<image.height as f64 {
                    out[c]=out[c].max(linear(image.sample(sx,sy,true)[c]));
                }
            }}out
        };
        let sigma=side*s.prism_blur/10_000.*copy as f64;
        let ghost=Light::blur_window(gw,gh,sigma,sigma,left,top,end_x-left,end_y-top,&pixel);
        for y in 0..ghost.height {for x in 0..ghost.width {
            let dx=(left+x)as i64+ox as i64;let dy=(top+y)as i64+oy as i64;
            let p=ghost.pixels[y*ghost.width+x];
            for c in 0..3 {let v=&mut result.pixels[dy as usize*image.width+dx as usize][c];*v=v.max(p[c]*fade).min(1.);}
        }}
    }
    result
}

fn flare(image:&Light,s:&LensEffectSettings,side:f64)->Light {
    let reach=side*s.flare_length/100.;let thin=(side*0.0008).max(0.5);let amount=s.flare_amount as f32/50.;let blue=s.flare_blue/100.;
    let spread=sources(image,s.threshold/100.,side).blur(side*0.004,side*0.004,true);
    let sources=spread.map(|p|p.map(|v|v*(v+0.12).min(1.)*1.6));
    let line_sources=sources.compact((side*0.012)as usize|1);
    let lengths=[1.-0.7*blue,1.-0.4*blue,1.];let coat=[1.-0.9*blue,1.-0.62*blue,1.].map(|v|v as f32);
    let mut flare=Light::black(image.width,image.height);
    for c in 0..3 {
        let line=soft_clip(&line_sources.streak(0.,reach*lengths[c],thin,0.).scaled([4.*amount;3]),0.6);
        let mut gain=[0.;3];gain[c]=coat[c];flare.add_scaled(&line,gain);
        let mut band=Light::black(image.width,image.height);
        for (fraction,weight)in [(0.08,0.8),(0.3,0.35),(0.9,0.15)]{
            let along=reach*fraction*lengths[c];let gain=weight*(along/(side*0.08)).sqrt()as f32*amount;
            band.add_scaled(&sources.streak(0.,along,side*0.006,0.),[gain;3]);
        }
        flare.add_scaled(&soft_clip(&band,0.7),gain);
    }
    flare.add_scaled(&sources.streak(0.,side*0.015,thin*3.,0.),[1.2*amount;3]);
    flare.add_scaled(&sources.blur(side*0.04,side*0.04,true),coat.map(|v|v*0.5*amount));
    flare.add_scaled(&image.map(|p|p.map(linear)),[1.;3]);flare
}
fn starburst(image:&Light,s:&LensEffectSettings,side:f64)->Light {
    let light=sources(image,s.threshold/100.,side);let reach=side*s.star_length/100.;let thin=(side*0.0006).max(0.5);
    let spokes=s.star_points/2;let mut rays=Light::black(image.width,image.height);
    for spoke in 0..spokes {let theta=s.star_angle.to_radians()+spoke as f64*std::f64::consts::PI/spokes as f64;
        rays.add_scaled(&light.star_spikes(-theta,reach,thin,s.star_amount/50.),[1.;3]);
    }
    rays.add_scaled(&image.map(|p|p.map(linear)),[1.;3]);rays
}
fn bokeh(image:&Light,s:&LensEffectSettings,side:f64)->Light {
    let radius=side*s.bokeh_radius/100.;let ring=s.bokeh_ring as f32/100.;
    let lights=highlights(image,0.8).scaled([12.+s.bokeh_brightness as f32*0.7;3]);
    let mut result=soft_clip(&lights.disc(radius,s.bokeh_shape,ring),0.8);
    result.add_scaled(&image.map(|p|p.map(linear)).disc(radius,s.bokeh_shape,ring),[1.;3]);result
}

#[cfg(test)]
mod density_tests {
    use super::*;
    #[test]
    fn density_selection_matches_original_core_image_before_optics() {
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mac1511");
        let input=Raster::from_premultiplied(96,64,std::fs::read(root.join("lens-input.rgba")).unwrap());
        let image=Light::from_raster(&input);
        for threshold in [96,98,100] {
            let actual=sources(&image,threshold as f64/100.,96.);
            if let Ok(dir)=std::env::var("LENS1511_DIAGNOSTICS") {let bytes:Vec<_>=actual.pixels.iter().flat_map(|p|p.iter().flat_map(|v|v.to_le_bytes())).collect();std::fs::write(std::path::Path::new(&dir).join(format!("sources-{threshold}.f32")),bytes).unwrap();}
            let bytes=std::fs::read(root.join(format!("sources-{threshold}.f32"))).unwrap();
            let expected:Vec<f32>=bytes.chunks_exact(4).map(|b|f32::from_le_bytes(b.try_into().unwrap())).collect();
            let mut max=0f32;let mut sum=0f64;
            for(p,q)in actual.pixels.iter().zip(expected.chunks_exact(4)) {for c in 0..3 {let e=(p[c]-q[c]).abs();max=max.max(e);sum+=e as f64;}}
            let mean=sum/(96.*64.*3.);eprintln!("source density {threshold}: max={max} mean={mean}");
            assert!(max<=0.05 && mean<=0.005,"{threshold}: max={max} mean={mean}");
        }
    }
}

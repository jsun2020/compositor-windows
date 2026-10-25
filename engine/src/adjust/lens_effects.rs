//! Mac 1.5.7 lens optics. All light operations use straight, linear RGB; the input
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
    pub bokeh_radius: f64, pub bokeh_brightness: f64, pub bokeh_ring: f64, pub bokeh_shape: BokehShape,
}
impl Default for LensEffectSettings {
    fn default()->Self { Self { effect:LensEffect::AnamorphicFlare,threshold:90.,flare_amount:50.,flare_length:60.,flare_blue:80.,
        star_amount:50.,star_length:12.,star_points:6,star_angle:15.,mist_strength:50.,mist_size:30.,
        prism_amount:70.,prism_copies:3.,prism_offset:11.,prism_angle:180.,prism_spread:70.,
        bokeh_radius:2.,bokeh_brightness:50.,bokeh_ring:100.,bokeh_shape:BokehShape::Round } }
}
impl LensEffectSettings {
    pub fn normalized(&self)->Self {
        let mut s=self.clone();let d=Self::default();
        let clamp=|v:f64,lo:f64,hi:f64,f:f64|if v.is_finite(){v.clamp(lo,hi)}else{f};
        macro_rules! bound {($field:ident,$lo:expr,$hi:expr)=>{s.$field=clamp(s.$field,$lo,$hi,d.$field);}}
        bound!(threshold,50.,99.);bound!(flare_amount,0.,100.);bound!(flare_length,1.,100.);bound!(flare_blue,0.,100.);
        bound!(star_amount,0.,100.);bound!(star_length,2.,40.);bound!(star_angle,-180.,180.);
        if ![4,6,8,12,14].contains(&s.star_points){s.star_points=6;}
        bound!(mist_strength,0.,100.);bound!(mist_size,0.,100.);bound!(prism_amount,0.,100.);bound!(prism_copies,1.,8.);
        s.prism_copies=s.prism_copies.round();bound!(prism_offset,0.,50.);bound!(prism_angle,-180.,180.);bound!(prism_spread,10.,100.);
        bound!(bokeh_radius,0.5,6.);bound!(bokeh_brightness,0.,100.);bound!(bokeh_ring,0.,100.);s
    }
    pub fn is_identity(&self)->bool { let s=self.normalized();match s.effect {
        LensEffect::AnamorphicFlare=>s.flare_amount==0.,LensEffect::Starburst=>s.star_amount==0.,LensEffect::Mist=>s.mist_strength==0.,
        LensEffect::Prism=>s.prism_amount==0.||s.prism_offset==0.,LensEffect::Bokeh=>false,
    } }
}

const CURVES:&[u8]=include_bytes!("../../native/LensToneCurves.bin");
fn curve(row:usize,value:f32)->f32 {
    let x=value.clamp(0.,1.)*1024.;let i=(x.floor()as usize).min(1023);let f=x-i as f32;
    let at=(row*1025+i)*4;
    let a=f32::from_le_bytes(CURVES[at..at+4].try_into().unwrap());let b=f32::from_le_bytes(CURVES[at+4..at+8].try_into().unwrap());a+(b-a)*f
}
pub fn highlight_curve(threshold:f64,value:f32)->f32 {
    let row=(threshold.clamp(0.5,0.95)-0.5)*400.;let low=(row.floor()as usize).min(180);let high=(low+1).min(180);
    let a=curve(low,value);a+(curve(high,value)-a)*(row-low as f64)as f32
}
fn linear(v:f32)->f32 { if v<=0.04045{v/12.92}else{((v+0.055)/1.055).powf(2.4)} }
fn gamma(v:f32)->f32 { if v<=0.0031308{v*12.92}else{1.055*v.powf(1./2.4)-0.055} }
fn highlights(image:&Light,threshold:f64)->Light { image.map(|p|{
    let mask=highlight_curve(threshold,0.2126*p[0]+0.7152*p[1]+0.0722*p[2]);p.map(|v|linear(v)*mask)
}) }
fn soft_clip(image:&Light,limit:f32)->Light {image.map(|p|p.map(|v|curve(181,v/(4.*limit))*limit))}

/// Refuse before allocating intermediates, leaving the source and undo history intact.
/// Rotated streak grids can be much larger than the source, especially for thin panoramas.
pub fn apply_checked(raster:&Raster,settings:&LensEffectSettings)->Result<Raster,crate::CommandError>{
    let s=settings.normalized();
    if s.is_identity(){return Ok(raster.clone());}
    let pixels=u64::from(raster.width)*u64::from(raster.height);
    let rotated=if s.effect==LensEffect::Starburst {
        (u64::from(raster.width)+u64::from(raster.height)).pow(2)/2
    }else{pixels};
    if pixels*8+rotated*4>40_000_000 {
        return Err(crate::CommandError::Refused("Lens Effects working memory limit exceeded; reduce the layer dimensions before applying this filter".into()));
    }
    Ok(apply(raster,&s))
}

pub fn apply(raster:&Raster,settings:&LensEffectSettings)->Raster {
    let s=settings.normalized();if s.is_identity(){return raster.clone();}
    let image=Light::from_raster(raster);let side=raster.width.max(raster.height)as f64;
    let result=match s.effect {
        LensEffect::Mist=>mist(&image,&s,side),LensEffect::Prism=>prism(&image,&s,side),
        LensEffect::AnamorphicFlare=>flare(&image,&s,side),LensEffect::Starburst=>starburst(&image,&s,side),
        LensEffect::Bokeh=>bokeh(&image,&s,side),
    };
    let mut values=Vec::with_capacity(raster.width as usize*raster.height as usize*4);
    for (i,p) in result.pixels.iter().enumerate(){let alpha=raster.pixel_f32(i as u32%raster.width,i as u32/raster.width)[3];
        values.extend([gamma(p[0].max(0.))*alpha,gamma(p[1].max(0.))*alpha,gamma(p[2].max(0.))*alpha,alpha]);}
    if raster.depth()==crate::PixelDepth::Eight {
        Raster::from_premultiplied(raster.width,raster.height,values.into_iter().map(|v|(v*255.).round().clamp(0.,255.)as u8).collect())
    } else {Raster::from_premultiplied_f32(raster.width,raster.height,values,raster.depth())}
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
    let light=image.map(|p|p.map(linear));let theta=s.prism_angle.to_radians();let dir=[theta.cos(),-theta.sin()];
    let shift=side*s.prism_offset/100.;let mut echoes=Light::black(image.width,image.height);
    for copy in 1..=s.prism_copies as u32 {let step=copy as f64;
        let mut ghost=Light::black(image.width,image.height);
        for (channel,stretch)in [1.+0.012*step,1.,1.-0.012*step].into_iter().enumerate(){
            for y in 0..image.height{for x in 0..image.width{
                ghost.pixels[y*image.width+x][channel]=light.sample(x as f64-dir[0]*shift*step*stretch,y as f64-dir[1]*shift*step*stretch,false)[channel];
            }}
        }
        ghost=ghost.blur(side*0.004*step,side*0.004*step,true).scaled([0.72f32.powi(copy as i32-1)*s.prism_amount as f32/100.;3]);
        for (out,p) in echoes.pixels.iter_mut().zip(ghost.pixels){for c in 0..3{out[c]=out[c].max(p[c]).min(1.);}}
    }
    let reach=dir[0].abs()*image.width as f64/2.+dir[1].abs()*image.height as f64/2.;
    let edge=[image.width as f64/2.+dir[0]*reach,image.height as f64/2.+dir[1]*reach];let depth=side*s.prism_spread/100.;
    for y in 0..image.height{for x in 0..image.width{let i=y*image.width+x;
        let mask=(1.-((edge[0]-x as f64-0.5)*dir[0]+(edge[1]-y as f64-0.5)*dir[1])/depth).clamp(0.,1.)as f32;
        for c in 0..3{echoes.pixels[i][c]=(echoes.pixels[i][c]*mask+light.pixels[i][c]*(1.-mask)).max(light.pixels[i][c]).min(1.);}
    }}echoes
}
fn flare(image:&Light,s:&LensEffectSettings,side:f64)->Light {
    let reach=side*s.flare_length/100.;let thin=(side*0.0008).max(0.5);let amount=s.flare_amount as f32/50.;let blue=s.flare_blue/100.;
    let spread=highlights(image,s.threshold/100.).blur(side*0.004,side*0.004,true);
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
    let light=highlights(image,s.threshold/100.);let reach=side*s.star_length/100.;let thin=(side*0.0006).max(0.5);
    let spokes=s.star_points/2;let mut rays=Light::black(image.width,image.height);
    for spoke in 0..spokes {let theta=s.star_angle.to_radians()+spoke as f64*std::f64::consts::PI/spokes as f64;
        for (fraction,weight)in [(0.04,1.8),(0.12,0.9),(0.35,0.35),(1.,0.1)]{for (c,stretch)in [1.,0.9,0.8].into_iter().enumerate(){
            let along=(reach*fraction*stretch).max(thin*2.);let mut gain=[0.;3];gain[c]=(weight*s.star_amount/50.)as f32;
            rays.add_scaled(&light.streak(-theta,along,thin,along*0.8),gain);
        }}
    }
    rays.add_scaled(&image.map(|p|p.map(linear)),[1.;3]);rays
}
fn bokeh(image:&Light,s:&LensEffectSettings,side:f64)->Light {
    let radius=side*s.bokeh_radius/100.;let ring=s.bokeh_ring as f32/100.;
    let lights=highlights(image,0.8).scaled([12.+s.bokeh_brightness as f32*0.7;3]);
    let mut result=soft_clip(&lights.disc(radius,s.bokeh_shape,ring),0.8);
    result.add_scaled(&image.map(|p|p.map(linear)).disc(radius,s.bokeh_shape,ring),[1.;3]);result
}

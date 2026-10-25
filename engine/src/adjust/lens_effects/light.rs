use super::BokehShape;
use crate::Raster;

#[derive(Clone)]
pub(super) struct Light { pub width:usize, pub height:usize, pub pixels:Vec<[f32;3]> }
impl Light {
    pub fn black(width:usize,height:usize)->Self {Self{width,height,pixels:vec![[0.;3];width*height]}}
    pub fn from_raster(r:&Raster)->Self {
        let mut image=Self::black(r.width as usize,r.height as usize);
        for y in 0..r.height{for x in 0..r.width{let p=r.pixel_f32(x,y);if p[3]>0.{image.pixels[(y*r.width+x)as usize]=[p[0]/p[3],p[1]/p[3],p[2]/p[3]];}}}image
    }
    pub fn map(&self, f:impl Fn([f32;3])->[f32;3])->Self {Self{width:self.width,height:self.height,pixels:self.pixels.iter().copied().map(f).collect()}}
    pub fn scaled(&self,gain:[f32;3])->Self{self.map(|p|[p[0]*gain[0],p[1]*gain[1],p[2]*gain[2]])}
    pub fn add_scaled(&mut self,other:&Self,gain:[f32;3]){for(a,b)in self.pixels.iter_mut().zip(&other.pixels){for c in 0..3{a[c]=(a[c]+b[c]*gain[c]).clamp(0.,1.);}}}
    fn get(&self,x:i64,y:i64,clamp:bool)->[f32;3]{
        if !clamp&&(x<0||y<0||x>=self.width as i64||y>=self.height as i64){return[0.;3];}
        self.pixels[y.clamp(0,self.height as i64-1)as usize*self.width+x.clamp(0,self.width as i64-1)as usize]
    }
    /// Pixel-center coordinates, with the first pixel centered at (0,0).
    pub fn sample(&self,x:f64,y:f64,clamp:bool)->[f32;3]{
        let ix=x.floor()as i64;let iy=y.floor()as i64;let tx=(x-ix as f64)as f32;let ty=(y-iy as f64)as f32;
        let mut out=[0.;3];for(xx,yy,w)in[(ix,iy,(1.-tx)*(1.-ty)),(ix+1,iy,tx*(1.-ty)),(ix,iy+1,(1.-tx)*ty),(ix+1,iy+1,tx*ty)]{
            let p=self.get(xx,yy,clamp);for c in 0..3{out[c]+=p[c]*w;}
        }out
    }
    pub fn cubic_sample(&self,x:f64,y:f64)->[f32;3]{
        let weight=|x:f64|{let x=x.abs();if x<1.{1.5*x*x*x-2.5*x*x+1.}else if x<2.{-0.5*x*x*x+2.5*x*x-4.*x+2.}else{0.}};
        let ix=x.floor()as i64;let iy=y.floor()as i64;let mut value=[0.;3];
        for yy in iy-1..=iy+2{for xx in ix-1..=ix+2{let w=(weight(xx as f64-x)*weight(yy as f64-y))as f32;let p=self.get(xx,yy,false);for c in 0..3{value[c]+=p[c]*w;}}}value
    }
    fn resized(&self,width:usize,height:usize,clamp:bool)->Self{
        if width==self.width&&height==self.height{return self.clone();}
        let mut out=Self::black(width,height);
        // Area averaging when reducing avoids dropping small light sources between samples.
        for y in 0..height{for x in 0..width{
            let sx=self.width as f64/width as f64;let sy=self.height as f64/height as f64;
            if sx>1.01||sy>1.01{
                let left=x as f64*sx;let right=left+sx;let top=y as f64*sy;let bottom=top+sy;let mut sum=[0.;3];
                for yy in top.floor()as i64..bottom.ceil()as i64{for xx in left.floor()as i64..right.ceil()as i64{
                    let weight=((right.min(xx as f64+1.)-left.max(xx as f64)).max(0.)*(bottom.min(yy as f64+1.)-top.max(yy as f64)).max(0.)/(sx*sy))as f32;
                    let p=self.get(xx,yy,clamp);for c in 0..3{sum[c]+=p[c]*weight;}
                }}out.pixels[y*width+x]=sum;
            }else{out.pixels[y*width+x]=self.sample((x as f64+0.5)*sx-0.5,(y as f64+0.5)*sy-0.5,clamp);}
        }}out
    }
    pub fn blur(&self,sigma_x:f64,sigma_y:f64,clamp:bool)->Self{
        // Keep the Gaussian work bounded at wide radii. Core Image likewise downsamples wide blurs.
        let reduction=(sigma_x.max(sigma_y)/6.).floor().max(1.)as usize;
        if reduction>1 && (self.width>1 || self.height>1) {
            let small=self.resized((self.width/reduction).max(1),(self.height/reduction).max(1),clamp);
            let result=small.blur(sigma_x*small.width as f64/self.width as f64,sigma_y*small.height as f64/self.height as f64,clamp);
            return result.resized(self.width,self.height,clamp);
        }
        let mut current=self.clone();
        for(horizontal,sigma)in[(true,sigma_x),(false,sigma_y)]{
            if sigma<0.05{continue;}
            let radius=(sigma*3.).ceil()as i64;let mut weights:Vec<f32>=(-radius..=radius).map(|x|(-(x*x)as f64/(2.*sigma*sigma)).exp()as f32).collect();
            let sum: f32=weights.iter().sum();for v in &mut weights{*v/=sum;}
            let mut out=Self::black(self.width,self.height);
            for y in 0..self.height{for x in 0..self.width{let mut value=[0.;3];
                for (j,w)in weights.iter().enumerate(){let d=j as i64-radius;let p=current.get(x as i64+if horizontal{d}else{0},y as i64+if horizontal{0}else{d},clamp);for c in 0..3{value[c]+=p[c]*w;}}
                out.pixels[y*self.width+x]=value;
            }}current=out;
        }current
    }
    fn morphology(&self,horizontal:bool,size:usize,maximum:bool)->Self{
        use std::collections::VecDeque;
        let size=size.max(3)|1;let radius=(size/2)as i64;
        let (length,lines)=if horizontal{(self.width,self.height)}else{(self.height,self.width)};
        let mut out=Self::black(self.width,self.height);
        for line in 0..lines{for c in 0..3{let mut queue:VecDeque<(i64,f32)>=VecDeque::new();
            for at in -radius..length as i64+radius {
                let p=if horizontal{self.get(at,line as i64,false)}else{self.get(line as i64,at,false)};
                while queue.back().is_some_and(|(_,v)|if maximum{*v<=p[c]}else{*v>=p[c]}){queue.pop_back();}
                queue.push_back((at,p[c]));let center=at-radius;
                while queue.front().is_some_and(|(i,_)|*i<center-radius){queue.pop_front();}
                if center>=0&&center<length as i64{
                    let index=if horizontal{line*self.width+center as usize}else{center as usize*self.width+line};out.pixels[index][c]=queue.front().unwrap().1;
                }
            }
        }}out
    }
    pub fn compact(&self,size:usize)->Self{
        let opened=self.morphology(false,size,false).morphology(false,size,true);let mut out=self.clone();
        for (p,o)in out.pixels.iter_mut().zip(opened.pixels){for c in 0..3{p[c]=(p[c]-o[c]).clamp(0.,1.);}}out
    }
    pub fn streak(&self,angle:f64,along:f64,across:f64,spread:f64)->Self{
        let across=across.max(0.5);let squeeze=(along/across).max(1.);let(s,c)=angle.sin_cos();
        let extent_w=c.abs()*self.width as f64+s.abs()*self.height as f64;
        let extent_h=s.abs()*self.width as f64+c.abs()*self.height as f64;
        let w=extent_w.ceil()as usize;let h=extent_h.ceil()as usize;
        let mut turned=Self::black(w,h);
        for y in 0..h{for x in 0..w{let u=x as f64+0.5-extent_w/2.;let v=y as f64+0.5-extent_h/2.;
            turned.pixels[y*w+x]=self.cubic_sample(self.width as f64/2.+u*c-v*s-0.5,self.height as f64/2.+u*s+v*c-0.5);
        }}
        if spread>=3.{turned=turned.morphology(true,(spread as usize|1).min(301),true);}
        // Preserve the continuous scale and the blur's extra extent: rounding a compressed
        // width to an integer shifts the optical axis and changes long streaks' brightness.
        let pad=(across*3.).ceil()as usize+3;
        let sw=(extent_w/squeeze).ceil().max(1.)as usize;
        let mut small=Self::black(sw+pad*2,h+pad*2);
        let sinc=|x:f64|if x.abs()<1e-8{1.}else{let p=x*std::f64::consts::PI;p.sin()/p};
        for x in 0..small.width {
            let center=(x as f64-pad as f64+0.5)*squeeze-0.5;
            let left=(center-3.*squeeze).ceil()as i64;let right=(center+3.*squeeze).floor()as i64;
            let mut weights=Vec::new();let mut total=0.;
            for i in left..=right{let t=(i as f64-center)/squeeze;let weight=sinc(t)*sinc(t/3.);total+=weight;weights.push((i,weight as f32));}
            for y in 0..h{let mut value=[0.;3];for &(i,weight)in &weights{let p=turned.get(i,y as i64,false);for c in 0..3{value[c]+=p[c]*weight;}}
                small.pixels[(y+pad)*small.width+x]=value.map(|v|v/total as f32);
            }
        }
        let back=small.blur(across,across,false);
        let mut out=Self::black(self.width,self.height);
        for y in 0..self.height{for x in 0..self.width{let u=x as f64+0.5-self.width as f64/2.;let v=y as f64+0.5-self.height as f64/2.;
            let at=((extent_w/2.+u*c+v*s)/squeeze-0.5+pad as f64,extent_h/2.-u*s+v*c-0.5+pad as f64);
            out.pixels[y*self.width+x]=if angle==0. {back.sample(at.0,at.1,false)}else{back.cubic_sample(at.0,at.1)}.map(|v|v.max(0.));
        }}out
    }
    pub fn disc(&self,radius:f64,shape:BokehShape,ring:f32)->Self{
        let shrink=(20./radius.max(0.5)).min(1.);let radius=(radius*shrink).max(0.5)as f32;
        let small=self.resized((self.width as f64*shrink).ceil().max(1.)as usize,(self.height as f64*shrink).ceil().max(1.)as usize,true)
            .map(|p|p.map(|v|half::f16::from_f32(v).to_f32()));
        let margin=(radius*1.7).ceil()+2.;let dimensions=[small.width as f32+2.*margin,small.height as f32+2.*margin];
        let rx=radius.ceil()as i32;let ry=(radius*if shape==BokehShape::Oval{1.62}else{1.}).ceil()as i32;
        let smooth=|a:f32,b:f32,x:f32|{let t=((x-a)/(b-a)).clamp(0.,1.);t*t*(3.-2.*t)};
        let mut out=Self::black(small.width,small.height);
        for y in 0..small.height{for x in 0..small.width{
            let m=[(x as f32+margin+0.5-dimensions[0]*0.5)/(dimensions[0].max(dimensions[1])*0.5),(y as f32+margin+0.5-dimensions[1]*0.5)/(dimensions[0].max(dimensions[1])*0.5)];
            let factor=m[0].hypot(m[1]).min(1.);let outward=[m[0]*factor,m[1]*factor];let mut sum=[0.;3];let mut total=0.;
            for dy in -ry..=ry{for dx in -rx..=rx{let o=[dx as f32/radius,dy as f32/radius];let d=match shape{
                BokehShape::Round=>o[0].hypot(o[1]),BokehShape::Hexagon=>(o[0].abs()*0.866025+o[1].abs()*0.5).max(o[1].abs())/0.866025*0.93,
                BokehShape::Oval=>o[0].hypot(o[1]*0.62),BokehShape::CatsEye=>o[0].hypot(o[1]).max((o[0]+outward[0]*1.25).hypot(o[1]+outward[1]*1.25)),
            };let inside=1.-smooth(1.-1./radius,1.,d);if inside<=0.{continue;}
                let weight=inside*(1.+ring*2.5*smooth(0.6,0.97,d));let p=small.get(x as i64+dx as i64,y as i64+dy as i64,true);
                for c in 0..3{sum[c]+=p[c]*weight;}total+=weight;
            }}out.pixels[y*small.width+x]=sum.map(|v|half::f16::from_f32(v/total.max(1e-5)).to_f32());
        }}out.resized(self.width,self.height,true)
    }
}

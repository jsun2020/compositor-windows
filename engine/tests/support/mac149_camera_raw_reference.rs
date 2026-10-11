// Independent trilinear transcription over the fixed CRT3 asset, not engine orchestration.
const MEASURED:&[u8]=include_bytes!(concat!(env!("OUT_DIR"),"/camera-raw-tables.rgb"));
pub fn lookup(index:usize,rgb:[f64;3])->[f64;3]{
 let pos=rgb.map(|v|v.clamp(0.,1.)*16.);let lo=pos.map(|v|(v as usize).min(15));let f=std::array::from_fn::<_,3,_>(|c|pos[c]-lo[c]as f64);let mut out=[0.;3];
 for dr in 0..2{for dg in 0..2{for db in 0..2{let weight=(if dr==0{1.-f[0]}else{f[0]})*(if dg==0{1.-f[1]}else{f[1]})*(if db==0{1.-f[2]}else{f[2]});let at=index*17*17*17*3+((lo[0]+dr)*17*17+(lo[1]+dg)*17+lo[2]+db)*3;for c in 0..3{out[c]+=weight*MEASURED[at+c]as f64/255.;}}}}out
}
pub fn lookup_blend(parts:&[(usize,f64)],rgb:[f64;3])->[f64;3]{let mut out=[0.;3];for &(i,w) in parts{let v=lookup(i,rgb);for c in 0..3{out[c]+=v[c]*(w as f32)as f64;}}out}
pub fn measured(parts:&[(usize,f64)],rgb:[f64;3])->[u8;3]{lookup_blend(parts,rgb.map(|v|v/255.)).map(|v|(v*255.).round()as u8)}
pub fn decode_srgb(v:f64)->f64{if v<=0.04045{v/12.92}else{((v+0.055)/1.055).powf(2.4)}}
pub fn encode_srgb(v:f64)->f64{if v<=0.0031308{12.92*v}else{1.055*v.powf(1./2.4)-0.055}}
pub fn adjacent(v:f64,p:&[f64])->(usize,f64){let v=v.clamp(p[0],*p.last().unwrap());let mut i=0;while i<p.len()-2&&v>p[i+1]{i+=1;}(i,(v-p[i])/(p[i+1]-p[i]))}
pub fn adaptive_parts(start:usize,v:f64,b:f64)->Vec<(usize,f64)>{let(r,d)=adjacent(b,&[53.,80.,104.,130.,160.,190.,224.]);let(c,a)=adjacent(v,&[-100.,-50.,-25.,0.,25.,50.,100.]);let i=start+r*7+c;vec![(i,(1.-d)*(1.-a)),(i+1,(1.-d)*a),(i+7,d*(1.-a)),(i+8,d*a)]}
pub fn white_parts(t:f64,tint:f64)->Vec<(usize,f64)>{let(r,d)=adjacent(t,&[-100.,-75.,-50.,-25.,0.,25.,50.,75.,100.]);let(c,a)=adjacent(tint,&[-100.,-75.,-50.,-25.,0.,25.,50.,75.,100.]);let i=r*9+c;vec![(i,(1.-d)*(1.-a)),(i+1,(1.-d)*a),(i+9,d*(1.-a)),(i+10,d*a)]}

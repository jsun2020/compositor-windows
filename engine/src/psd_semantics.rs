//! Photoshop descriptors and the text-engine dictionary. Derived from the
//! bounded MIT Compositor 1.4.5 parsers; unsupported structures retain pixels.
use super::*;
use crate::ops::text::TextStyle;
use serde_json::{Map,Value as Json};
#[derive(Debug)]enum Value<'a>{Number(f64),Text(String),Data(&'a [u8]),Dict(BTreeMap<String,Value<'a>>),List(Vec<Value<'a>>)}
impl Value<'_>{
    fn number(&self)->Option<f64>{if let Self::Number(n)=self {Some(*n)}else{None}}
    fn text(&self)->Option<&str>{if let Self::Text(s)=self {Some(s)}else{None}}
    fn dict(&self)->Option<&BTreeMap<String,Value<'_>>>{if let Self::Dict(d)=self {Some(d)}else{None}}
    fn find(&self,key:&str)->Option<&Value<'_>>{match self {Self::Dict(d)=>d.get(key).or_else(||d.values().find_map(|v|v.find(key))),Self::List(v)=>v.iter().find_map(|v|v.find(key)),_=>None}}
}
fn unicode(c:&mut Cursor)->Option<String>{let n=c.u32().ok()? as usize;if n>100_000{return None;}let bytes=c.take(n.checked_mul(2)?).ok()?;let units:Vec<_>=bytes.chunks_exact(2).map(|p|u16::from_be_bytes([p[0],p[1]])).collect();Some(String::from_utf16_lossy(&units))}
fn identifier(c:&mut Cursor)->Option<String>{let n=c.u32().ok()? as usize;let n=if n==0{4}else{n};if n>1024{return None;}String::from_utf8(c.take(n).ok()?.to_vec()).ok()}
fn double(c:&mut Cursor)->Option<f64>{let n=f64::from_be_bytes(c.take(8).ok()?.try_into().ok()?);n.is_finite().then_some(n)}
fn descriptor<'a>(c:&mut Cursor<'a>,depth:usize,budget:&mut usize)->Option<Value<'a>>{
    if depth>32{return None;}unicode(c)?;identifier(c)?;let n=c.u32().ok()? as usize;if n>*budget{return None;}*budget-=n;
    let mut dict=BTreeMap::new();for _ in 0..n{let key=identifier(c)?;let kind=c.key().ok()?;let value=value(c,kind,depth,budget)?;dict.insert(key,value);}Some(Value::Dict(dict))
}
fn value<'a>(c:&mut Cursor<'a>,kind:&str,depth:usize,budget:&mut usize)->Option<Value<'a>>{
    if depth>32{return None;}Some(match kind {
        "doub"=>Value::Number(double(c)?),"UntF"=>{c.skip(4).ok()?;Value::Number(double(c)?)},"long"=>Value::Number(c.i32().ok()? as f64),"comp"=>Value::Number(c.u64().ok()? as i64 as f64),"bool"=>Value::Number(if c.u8().ok()?!=0{1.}else{0.}),"TEXT"=>Value::Text(unicode(c)?),
        "enum"=>{identifier(c)?;Value::Text(identifier(c)?)},"tdta"|"alis"=>{let n=c.u32().ok()? as usize;if n>8_000_000{return None;}Value::Data(c.take(n).ok()?)},
        "Objc"|"GlbO"=>descriptor(c,depth+1,budget)?,"VlLs"=>{let n=c.u32().ok()? as usize;if n>*budget{return None;}*budget-=n;let mut list=Vec::with_capacity(n);for _ in 0..n{let kind=c.key().ok()?;list.push(value(c,kind,depth+1,budget)?);}Value::List(list)},
        "type"|"GlbC"=>{unicode(c)?;identifier(c)?;Value::Number(0.)},_=>return None,
    })
}
fn parse_descriptor(bytes:&[u8],prefix:usize)->Option<Value<'_>>{if bytes.len()>8_000_000{return None;}let mut c=Cursor::new(bytes);c.skip(prefix).ok()?;if c.u32().ok()?!=16{return None;}descriptor(&mut c,0,&mut 10_000)}
fn json_string(raw:&[u8])->String{if raw.starts_with(&[254,255]) {let units:Vec<_>=raw[2..].chunks_exact(2).map(|p|u16::from_be_bytes([p[0],p[1]])).collect();String::from_utf16_lossy(&units)}else{raw.iter().map(|&c|c as char).collect()}}
struct EngineData<'a>{bytes:&'a [u8],at:usize,left:usize}
impl EngineData<'_>{
    fn peek(&self)->Option<u8>{self.bytes.get(self.at).copied()}
    fn take(&mut self,token:&[u8])->bool{if self.bytes.get(self.at..self.at+token.len())==Some(token){self.at+=token.len();true}else{false}}
    fn whitespace(&mut self){while self.peek().is_some_and(|c|c<=32||c==b'%'){if self.peek()==Some(b'%'){while self.peek().is_some_and(|c|c!=10&&c!=13){self.at+=1;}}else{self.at+=1;}}}
    fn delimiter(c:u8)->bool{c<=32||b"/<>[]()".contains(&c)}
    fn token(&mut self)->Option<String>{let start=self.at;while self.peek().is_some_and(|c|!Self::delimiter(c)){self.at+=1;}if start==self.at{return None;}String::from_utf8(self.bytes[start..self.at].to_vec()).ok()}
    fn parse(&mut self,depth:usize)->Option<Json>{
        if depth>32||self.left==0{return None;}self.left-=1;self.whitespace();let first=self.peek()?;
        if self.take(b"<<"){let mut dict=Map::new();loop{self.whitespace();if self.take(b">>"){break;}if !self.take(b"/"){return None;}let key=self.token()?;dict.insert(key,self.parse(depth+1)?);}return Some(Json::Object(dict));}
        if self.take(b"["){let mut list=Vec::new();loop{self.whitespace();if self.take(b"]"){break;}list.push(self.parse(depth+1)?);}return Some(Json::Array(list));}
        if self.take(b"("){let mut raw=Vec::new();let mut nesting=1usize;while let Some(c)=self.peek(){self.at+=1;if c==b'\\'{let escaped=self.peek()?;self.at+=1;match escaped {b'n'=>raw.push(10),b'r'=>raw.push(13),b't'=>raw.push(9),b'0'..=b'7'=>{let mut n=(escaped-b'0') as u16;for _ in 0..2{if let Some(d @ b'0'..=b'7')=self.peek(){self.at+=1;n=n*8+(d-b'0') as u16;}else{break;}}raw.push(n as u8)},10|13=>{},_=>raw.push(escaped)}}else if c==b')'{nesting-=1;if nesting==0{return Some(Json::String(json_string(&raw)));}raw.push(c);}else{if c==b'(' {nesting+=1;}raw.push(c);}}return None;}
        if self.take(b"<"){let mut hex=Vec::new();loop{if self.take(b">"){break;}let c=self.peek()?;self.at+=1;if c.is_ascii_whitespace(){continue;}hex.push((c as char).to_digit(16)? as u8);}if hex.len()%2!=0{hex.push(0);}let raw:Vec<_>=hex.chunks_exact(2).map(|p|p[0]*16+p[1]).collect();return Some(Json::String(json_string(&raw)));}
        if self.take(b"/"){return Some(Json::String(self.token()?));}
        let token=self.token()?;match token.as_str(){"true"=>Some(Json::Bool(true)),"false"=>Some(Json::Bool(false)),"null"=>Some(Json::Null),_=>{if first!=b'+'&&first!=b'-'&&first!=b'.'&&!first.is_ascii_digit(){return None;}let n=token.parse::<f64>().ok()?;serde_json::Number::from_f64(n).map(Json::Number)}}
    }
}
fn path<'a>(value:&'a Json,keys:&[&str])->Option<&'a Json>{let mut value=value;for k in keys{value=value.get(k)?;}Some(value)}
fn num(value:&Json,keys:&[&str],default:f64)->f64{path(value,keys).and_then(Json::as_f64).unwrap_or(default)}
fn unit(v:f64)->f64{if v>1. {(v/255.).clamp(0.,1.)}else{v.clamp(0.,1.)}}
pub(super) fn text(extra:&BTreeMap<String,&[u8]>)->Option<(Json,Vec<String>)>{
    let bytes=extra.get("TySh").or_else(||extra.get("tySh"))?;if bytes.len()>8_000_000{return None;}let mut c=Cursor::new(bytes);if c.u16().ok()?!=1{return None;}
    let [xx,xy,yx,yy,_tx,_ty]=[double(&mut c)?,double(&mut c)?,double(&mut c)?,double(&mut c)?,double(&mut c)?,double(&mut c)?];
    // The cached bitmap already contains any rotation. Only an axis-aligned,
    // uniformly scaled record can retain editable text without rotating it twice.
    if xx<=0.||yy<=0.||xy.abs()>1e-6||yx.abs()>1e-6||(xx-yy).abs()>0.02*xx.max(yy){return None;}
    if c.u16().ok()?!=50||c.u32().ok()?!=16{return None;}let descriptor_value=descriptor(&mut c,0,&mut 10_000)?;let dict=descriptor_value.dict()?;
    if dict.get("Ornt").and_then(Value::text)==Some("Vrtc"){return None;}
    let mut engine=None;if let Some(Value::Data(data))=dict.get("EngineData"){engine=EngineData{bytes:data,at:0,left:100_000}.parse(0);}
    let content=dict.get("Txt ").or_else(||dict.get("Txt")).and_then(Value::text).or_else(||engine.as_ref().and_then(|e|path(e,&["EngineDict","Editor","Text"])).and_then(Json::as_str))?.trim_end_matches(['\r','\0']).replace('\r',"\n");
    if content.is_empty()||content.encode_utf16().count()>100_000{return None;}
    let mut style=TextStyle{content,font_size:(12.*xx).clamp(1.,2000.),..TextStyle::default()};let mut notes=Vec::new();
    if let Some(engine)=engine {
        let runs=path(&engine,&["EngineDict","StyleRun","RunArray"]).and_then(Json::as_array);let first=runs.and_then(|v|v.first()).unwrap_or(&engine);let data=path(first,&["StyleSheet","StyleSheetData"]).unwrap_or(first);
        style.font_size=(num(data,&["FontSize"],12.)*xx).clamp(1.,2000.);let index=num(data,&["Font"],0.).round().max(0.) as usize;
        if let Some(font)=path(&engine,&["ResourceDict","FontSet"]).and_then(Json::as_array).and_then(|v|v.get(index)).and_then(|v|v.get("Name")).and_then(Json::as_str){style.font_name=font.into();}
        if let Some(values)=path(data,&["FillColor","Values"]).and_then(Json::as_array){let values:Vec<_>=values.iter().filter_map(Json::as_f64).collect();let rgb=if values.len()>=4{[values[1],values[2],values[3]]}else if values.len()==3{[values[0],values[1],values[2]]}else{[values.first().copied().unwrap_or(0.);3]};[style.red,style.green,style.blue]=rgb.map(unit);}
        style.tracking=(num(data,&["Tracking"],0.)*style.font_size/1000.).clamp(-100.,1000.);
        if path(data,&["AutoLeading"]).and_then(Json::as_bool)==Some(false){style.leading=(num(data,&["Leading"],0.)*xx).clamp(0.,5000.);}
        if ["FauxBold","FauxItalic"].iter().any(|k|data.get(k).and_then(Json::as_bool)==Some(true)){notes.push("Faux bold or faux italic was omitted.".into());}
        if runs.is_some_and(|r|r.iter().skip(1).any(|v|v!=first)){notes.push("Only the first Photoshop text style was retained.".into());}
        let paragraph=path(&engine,&["EngineDict","ParagraphRun","RunArray"]).and_then(Json::as_array).and_then(|v|v.first());let justification=paragraph.map_or(0.,|v|num(v,&["ParagraphSheet","Properties","Justification"],0.));style.alignment=match justification as i32{1=>"Right",2=>"Center",_=>"Left"}.into();if justification>2.{notes.push("Full justification was converted to left alignment.".into());}
    }
    let rect=|key:&str|->Option<[f64;4]>{let d=dict.get(key)?.dict()?;Some([d.get("Left")?.number()?,d.get("Top ").or_else(||d.get("Top"))?.number()?,d.get("Rght")?.number()?,d.get("Btom")?.number()?])};
    if let (Some([l,t,r,b]),Some([gl,gt,gr,gb]))=(rect("bounds"),rect("boundingBox")){if r-l>gr-gl+4.&&b-t>gb-gt+4.{style.box_size=Some(Size{width:(r-l)*xx+24.,height:(b-t)*xx+24.});}}
    if c.remaining()>=6 && c.u16().ok()?==1 && c.u32().ok()?==16 {if let Some(warp)=descriptor(&mut c,0,&mut 10_000){if warp.find("warpStyle").and_then(Value::text).is_some_and(|s|s!="warpNone"&&s!="none"){notes.push("The Photoshop text warp was omitted.".into());}}}
    if !style.is_valid(){return None;}notes.push("Editable text retains the Photoshop preview until it is edited; installed fonts may change the result.".into());Some((serde_json::to_value(style).ok()?,notes))
}
pub(super) fn shape(extra:&BTreeMap<String,&[u8]>)->Option<(ShapeSpec,[f64;3],Vec<String>)>{
    let fill=parse_descriptor(extra.get("SoCo")?,0)?;let color=["Rd  ","Grn ","Bl  "].map(|k|fill.find(k).and_then(Value::number).map(unit));let color=[color[0]?,color[1]?,color[2]?];
    let stroke=extra.get("vstk").and_then(|data|parse_descriptor(data,0));if stroke.as_ref().and_then(|v|v.find("fillEnabled")).and_then(Value::number)==Some(0.){return None;}
    let origin=parse_descriptor(extra.get("vogk")?,4)?;let kind=origin.find("keyOriginType")?.number()? as i32;let box_value=origin.find("keyOriginShapeBBox")?;
    let left=box_value.find("Left")?.number()?;let top=box_value.find("Top ").or_else(||box_value.find("Top"))?.number()?;let right=box_value.find("Rght")?.number()?;let bottom=box_value.find("Btom")?.number()?;
    let rect=Rect{x:left.floor(),y:top.floor(),width:right.ceil()-left.floor(),height:bottom.ceil()-top.floor()};if rect.width<1.||rect.height<1.||rect.width>MAX_SIDE as f64||rect.height>MAX_SIDE as f64||rect.width*rect.height>MAX_PIXELS as f64{return None;}
    let mut radius=0.;if kind==2 {let radii=origin.find("keyOriginRRectRadii")?;let values=["topLeft","topRight","bottomRight","bottomLeft"].map(|k|radii.find(k).and_then(Value::number));let values=[values[0]?,values[1]?,values[2]?,values[3]?];let low=values.iter().copied().fold(f64::INFINITY,f64::min);radius=values.iter().copied().fold(0.,f64::max);if radius-low>0.5{return None;}}
    let spec=match kind{1|2=>ShapeSpec::Rectangle{rect,corner_radius:radius},5=>ShapeSpec::Ellipse{rect},_=>return None};let mut notes=Vec::new();if stroke.as_ref().and_then(|s|s.find("strokeEnabled")).and_then(Value::number)==Some(1.){notes.push("The Photoshop vector stroke was omitted.".into());}Some((spec,color,notes))
}
#[cfg(test)]mod tests {use super::*;
    #[test]fn engine_dictionary_handles_utf16_escaped_strings_and_refuses_unclosed_strings(){let bytes=br"<< /Text (\376\377\000A\000B) /N 12.5 /A [ true false ] >>";let value=EngineData{bytes,at:0,left:100}.parse(0).unwrap();assert_eq!(value["Text"],"AB");assert_eq!(value["N"],12.5);assert!(EngineData{bytes:b"(unterminated",at:0,left:100}.parse(0).is_none());}
    #[test]fn recursive_engine_dictionary_is_bounded(){let mut bytes=vec![b'[';40];bytes.extend(vec![b']';40]);assert!(EngineData{bytes:&bytes,at:0,left:1000}.parse(0).is_none());assert!(EngineData{bytes:b"[1 2 3]",at:0,left:2}.parse(0).is_none());}
}

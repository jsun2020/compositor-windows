//! Bounded Windows PDF / HEIF decoding. No document is changed until all pages decode.
#[cfg(windows)]
#[path="native_images_windows.rs"] mod platform;
#[cfg(any(windows,test))]
#[path="bundled_heif.rs"] mod bundled_heif;
pub const MAX_BYTES:usize=512*1024*1024;
pub const MAX_PIXELS:u64=100_000_000;
pub struct Page {pub name:String,pub png:Vec<u8>}
pub struct Decoded {pub resolution:f64,pub pages:Vec<Page>}

pub fn page_size(width:f64,height:f64,fitting:Option<[u32;2]>,remaining:u64)->Result<[u32;2],String>{
    if !width.is_finite()||!height.is_finite()||width<=0.||height<=0.{return Err("Invalid page dimensions".into());}
    let scale=if let Some([w,h])=fitting{if w==0||h==0||w>30000||h>30000{return Err("Invalid destination canvas".into());}(w as f64/width).min(h as f64/height)}else{300./96.};
    let w=(width*scale).round();let h=(height*scale).round();
    if !(1.0..=30000.).contains(&w)||!(1.0..=30000.).contains(&h)||w*h>remaining.min(MAX_PIXELS)as f64{return Err("PDF pages exceed the remaining 100 MP document budget or 30,000 pixel side limit".into());}
    Ok([w as u32,h as u32])
}
pub fn wire(decoded:Decoded)->Result<Vec<u8>,String>{
    if decoded.pages.is_empty()||decoded.pages.len()>10000{return Err("Invalid image page count".into());}
    let header=serde_json::to_vec(&serde_json::json!({"resolution":decoded.resolution,"images":decoded.pages.iter().map(|p|serde_json::json!({"name":p.name,"length":p.png.len()})).collect::<Vec<_>>()})).map_err(|e|e.to_string())?;
    let size=decoded.pages.iter().try_fold(8usize+header.len(),|sum,p|sum.checked_add(p.png.len())).filter(|n|*n<=MAX_BYTES).ok_or("Decoded image transfer exceeds 512 MiB")?;
    let mut result=Vec::new();result.try_reserve_exact(size).map_err(|_|"Not enough memory for decoded images")?;
    result.extend(b"NIM1");result.extend((header.len()as u32).to_le_bytes());result.extend(header);for p in decoded.pages{result.extend(p.png);}Ok(result)
}
pub fn decode(path:&str,fitting:Option<[u32;2]>,remaining:u64)->Result<Vec<u8>,String>{
    let path=std::path::Path::new(path);let meta=std::fs::metadata(path).map_err(|e|format!("Cannot read image: {e}"))?;
    if !meta.is_file()||meta.len()>MAX_BYTES as u64{return Err("Choose an image under 512 MiB".into());}
    let ext=path.extension().and_then(|v|v.to_str()).unwrap_or("").to_ascii_lowercase();
    if !["pdf","heic","heif","hif"].contains(&ext.as_str()){return Err("Unsupported native image format".into());}
    #[cfg(windows)]{wire(platform::decode(path,ext=="pdf",fitting,remaining.min(MAX_PIXELS))?)}
    #[cfg(not(windows))]{let _=(fitting,remaining);Err("PDF and HEIF native decoding requires Windows".into())}
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn pdf_print_size_fit_rotation_and_budget(){assert_eq!(page_size(816.,1056.,None,MAX_PIXELS).unwrap(),[2550,3300]);assert_eq!(page_size(1056.,816.,Some([800,600]),MAX_PIXELS).unwrap(),[776,600]);assert!(page_size(816.,1056.,None,8_414_999).is_err());assert!(page_size(f64::NAN,10.,None,MAX_PIXELS).is_err());assert!(page_size(10.,10.,Some([0,100]),MAX_PIXELS).is_err());}
    #[test]fn transfer_carries_exact_page_lengths_and_bytes(){let data=wire(Decoded{resolution:300.,pages:vec![Page{name:"Page 1".into(),png:vec![1,2,3]},Page{name:"Page 2".into(),png:vec![4,5]}]}).unwrap();assert_eq!(&data[..4],b"NIM1");let n=u32::from_le_bytes(data[4..8].try_into().unwrap())as usize;let h:serde_json::Value=serde_json::from_slice(&data[8..8+n]).unwrap();assert_eq!(h["images"][0]["length"],3);assert_eq!(&data[8+n..],&[1,2,3,4,5]);}
}

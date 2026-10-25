//! Mac 1.4.10 ImageFolder: immediate image children, natural filename ordering, no recursion.
use std::{cmp::Ordering,path::Path};
pub fn images(path:&Path)->Result<Vec<String>,String>{
 let mut files=Vec::new();
 for entry in std::fs::read_dir(path).map_err(|e|format!("Cannot read {}: {e}",path.display()))? {
  let entry=entry.map_err(|e|e.to_string())?;let p=entry.path();let name=entry.file_name().to_string_lossy().into_owned();
  if name.starts_with('.') || !entry.file_type().map_err(|e|e.to_string())?.is_file() {continue;}
  let ext=p.extension().and_then(|v|v.to_str()).unwrap_or("").to_ascii_lowercase();
  if !["png","jpg","jpeg","tif","tiff","exr","hdr","webp","bmp","gif","heic","heif","hif","pdf","psd","psb","dng","cr2","cr3","crw","nef","nrw","arw","srf","sr2","raf","rw2","orf","pef","ptx","raw","rwl","srw","3fr","fff","iiq","kdc","dcr","mos","mef","erf","x3f"].contains(&ext.as_str()){continue;}
  if files.len()>=10_000 {return Err(format!("{} contains more than 10,000 images.",path.display()));}
  files.push(p.to_string_lossy().into_owned());
 }
 files.sort_by(|a,b|natural(a,b).then_with(||a.cmp(b)));
 if files.is_empty(){return Err(format!("{} has no images to import.",path.display()));}
 Ok(files)
}
fn natural(a:&str,b:&str)->Ordering{
 let a=a.to_lowercase();let b=b.to_lowercase();let(mut a,mut b)=(a.as_bytes(),b.as_bytes());
 while !a.is_empty()&&!b.is_empty(){
  if a[0].is_ascii_digit()&&b[0].is_ascii_digit(){
   let na=a.iter().take_while(|v|v.is_ascii_digit()).count();let nb=b.iter().take_while(|v|v.is_ascii_digit()).count();
   let aa=&a[..na];let bb=&b[..nb];let aa=&aa[aa.iter().take_while(|v|**v==b'0').count()..];let bb=&bb[bb.iter().take_while(|v|**v==b'0').count()..];
   let order=aa.len().cmp(&bb.len()).then_with(||aa.cmp(bb));if order!=Ordering::Equal{return order;}a=&a[na..];b=&b[nb..];
  }else{let order=a[0].cmp(&b[0]);if order!=Ordering::Equal{return order;}a=&a[1..];b=&b[1..];}
 }a.len().cmp(&b.len())
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn directory_import_is_flat_filters_hidden_and_sorts_numerically(){
  let root=std::env::temp_dir().join(format!("compositor-folder-{}",std::process::id()));std::fs::create_dir_all(root.join("nested")).unwrap();
  for name in ["image10.PNG","image2.png","image1.tif",".hidden.png","notes.txt","nested/image3.png"]{std::fs::write(root.join(name),b"fixture").unwrap();}
  let names:Vec<_>=images(&root).unwrap().into_iter().map(|p|Path::new(&p).file_name().unwrap().to_string_lossy().into_owned()).collect();
  assert_eq!(names,["image1.tif","image2.png","image10.PNG"]);std::fs::remove_dir_all(root).unwrap();
 }
 #[test]fn numeric_names_do_not_overflow(){assert_eq!(natural("frame9999999999999999999999999.png","frame10000000000000000000000000.png"),Ordering::Less);}
}

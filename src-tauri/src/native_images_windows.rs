use super::{Decoded,Page,MAX_BYTES,page_size};
use windows::{core::HSTRING,Data::Pdf::PdfDocument,Storage::{StorageFile},Win32::System::WinRT::{RoInitialize,RoUninitialize,RO_INIT_MULTITHREADED}};

#[path="native_pdf_windows.rs"] mod pixel_pdf;

struct Apartment;
impl Drop for Apartment{fn drop(&mut self){unsafe{RoUninitialize()}}}
fn error(e:windows::core::Error)->String{format!("Windows image decoder: {e}")}
pub fn decode(path:&std::path::Path,pdf:bool,fitting:Option<[u32;2]>,remaining:u64)->Result<Decoded,String>{
    unsafe{RoInitialize(RO_INIT_MULTITHREADED)}.map_err(error)?;let _apartment=Apartment;
    if !pdf{return super::bundled_heif::decode(path,remaining);}
    let file=StorageFile::GetFileFromPathAsync(&HSTRING::from(path.as_os_str())).map_err(error)?.join().map_err(error)?;
    decode_pdf(&file,path,fitting,remaining)
}

fn decode_pdf(file:&StorageFile,path:&std::path::Path,fitting:Option<[u32;2]>,mut remaining:u64)->Result<Decoded,String>{
    let document=PdfDocument::LoadFromFileAsync(file).map_err(error)?.join().map_err(|e|format!("Cannot open PDF (password-protected files must be unlocked first): {e}"))?;
    let count=document.PageCount().map_err(error)?;if count==0||count>10000{return Err("PDF must contain 1–10,000 pages".into());}
    let mut dimensions=Vec::new();
    for i in 0..count{
        let page=document.GetPage(i).map_err(error)?;let size=page.Size().map_err(error)?;
        let [w,h]=page_size(size.Width as f64,size.Height as f64,fitting,remaining)?;remaining-=u64::from(w)*u64::from(h);dimensions.push([w,h]);page.Close().map_err(error)?;
    }
    let renderer=pixel_pdf::Renderer::new()?;
    let mut pages=Vec::new();let mut total=0usize;
    for (i,[w,h])in dimensions.into_iter().enumerate(){
        let page=document.GetPage(i as u32).map_err(error)?;
        let rendered=renderer.render(&page,w,h);let closed=page.Close();
        let png=rendered?;closed.map_err(error)?;
        total=total.checked_add(png.len()).filter(|v|*v<=MAX_BYTES).ok_or("Rendered PDF exceeds 512 MiB")?;
        let name=if count==1{path.file_stem().unwrap_or_default().to_string_lossy().into_owned()}else{format!("Page {}",i+1)};
        pages.push(Page{name,png});
    }
    Ok(Decoded{resolution:300.,pages})
}

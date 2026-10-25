use tauri::{Manager,ipc::{InvokeBody,Request,Response}};
fn folder(app:&tauri::AppHandle)->Result<std::path::PathBuf,String>{Ok(app.path().app_data_dir().map_err(|e|e.to_string())?.join("CMYK Profiles"))}
#[tauri::command]
pub async fn list_color_profiles(app:tauri::AppHandle)->Result<Vec<crate::color_profiles::ProfileInfo>,String>{
    let folder=folder(&app)?;tauri::async_runtime::spawn_blocking(move||crate::color_profiles::list(&folder)).await.map_err(|e|e.to_string())
}
#[tauri::command]
pub async fn proof_color_profile(app:tauri::AppHandle,path:String,import:bool)->Result<crate::color_profiles::Proof,String>{
    let folder=folder(&app)?;tauri::async_runtime::spawn_blocking(move||crate::color_profiles::proof(&path,&folder,import)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn import_cmyk_file(app:tauri::AppHandle,path:String,profile:Option<String>)->Result<Response,String>{
    let folder=folder(&app)?;tauri::async_runtime::spawn_blocking(move||crate::color_profiles::import_file(&path,profile.as_deref(),&folder)).await.map_err(|e|e.to_string())?.map(Response::new)
}
#[derive(serde::Deserialize)]struct Options{profile:String,format:String,quality:f64,dpi:f64}
#[tauri::command]
pub async fn export_cmyk(request:Request<'_>)->Result<Response,String>{
    let options=request.headers().get("color-options").and_then(|v|v.to_str().ok()).ok_or("Missing color options")?;
    let o:Options=serde_json::from_str(&super::percent_decode(options)).map_err(|e|e.to_string())?;
    let pixels=match request.body(){InvokeBody::Raw(b)if b.len()<=512*1024*1024=>b.clone(),_=>return Err("Expected PNG data under 512 MiB".into())};
    tauri::async_runtime::spawn_blocking(move||crate::color_profiles::export(&pixels,&o.profile,&o.format,o.quality,o.dpi)).await.map_err(|e|e.to_string())?.map(Response::new)
}

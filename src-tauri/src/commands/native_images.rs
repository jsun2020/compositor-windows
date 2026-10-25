#[tauri::command]
pub async fn decode_native_image(path:String,fitting:Option<[u32;2]>,remaining:u64)->Result<tauri::ipc::Response,String>{
    tauri::async_runtime::spawn_blocking(move||crate::native_images::decode(&path,fitting,remaining)).await
        .map_err(|e|format!("Image decoder stopped: {e}"))?.map(tauri::ipc::Response::new)
}

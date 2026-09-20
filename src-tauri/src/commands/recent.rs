use std::fs;
use tauri::{AppHandle, Manager};

fn store_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("recent.json"))
}

#[tauri::command]
pub fn recent_packages(app: AppHandle) -> Result<Vec<String>, String> {
    let p = store_path(&app)?;
    if !p.exists() { return Ok(vec![]); }
    let text = fs::read_to_string(&p).map_err(|e| e.to_string())?;
    let list: Vec<String> = serde_json::from_str(&text).unwrap_or_default();
    Ok(list.into_iter().filter(|s| std::path::Path::new(s).is_dir()).collect())
}

#[tauri::command]
pub fn add_recent_package(app: AppHandle, path: String) -> Result<(), String> {
    let mut list = recent_packages(app.clone())?;
    list.retain(|p| p != &path);
    list.insert(0, path);
    list.truncate(10);
    fs::write(store_path(&app)?, serde_json::to_string(&list).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

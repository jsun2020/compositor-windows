use super::percent_decode;
use std::fs;
use std::path::PathBuf;
use tauri::ipc::{InvokeBody, Request, Response};

const MAX_FILE: u64 = 512 * 1024 * 1024;

#[tauri::command]
pub fn read_file(path: String) -> Result<Response, String> {
    let p = PathBuf::from(&path);
    let meta = fs::metadata(&p).map_err(|e| format!("Cannot read {}: {e}", p.display()))?;
    if !meta.is_file() || meta.len() > MAX_FILE { return Err(format!("{} is not a readable file under 512 MiB.", p.display())); }
    Ok(Response::new(fs::read(&p).map_err(|e| format!("Cannot read {}: {e}", p.display()))?))
}

/// Header `path` (percent-encoded), raw body. Writes to a sibling temp file then renames over the target.
#[tauri::command]
pub fn write_file(request: Request<'_>) -> Result<(), String> {
    let path = request.headers().get("path").and_then(|v| v.to_str().ok()).ok_or("missing header path")?;
    let path = PathBuf::from(percent_decode(path));
    let bytes = match request.body() { InvokeBody::Raw(b) => b, _ => return Err("expected raw body".into()) };
    let tmp = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    fs::write(&tmp, bytes).map_err(|e| format!("Cannot write {}: {e}", tmp.display()))?;
    if let Err(e) = fs::rename(&tmp, &path) { let _ = fs::remove_file(&tmp); return Err(format!("Cannot replace {}: {e}", path.display())); }
    Ok(())
}

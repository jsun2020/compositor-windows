use super::percent_decode;
use crate::atomic;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::ipc::{Request, Response};
use tauri::State;

const MAX_MANIFEST: u64 = 4 * 1024 * 1024;
const MAX_ASSET: u64 = 512 * 1024 * 1024;

pub fn valid_image_name(name: &str) -> bool {
    let re = regex::Regex::new(r"^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}(\.mask)?\.png$").unwrap();
    re.is_match(name)
}

#[derive(Serialize)]
pub struct PackageHeader { pub manifest: String, pub image_names: Vec<String> }

fn io(e: std::io::Error, what: &str, path: &Path) -> String { format!("{what} {}: {e}", path.display()) }

/// Rejects a symlink and a path that escapes `root` after canonicalization, matching
/// `ProjectStore.checkFile` on macOS: `isSymbolicLink != true` and a prefix-match resolved
/// path. Returns the file's metadata (never a symlink's, since that case is already rejected)
/// so callers do not need a second stat call.
fn checked_metadata(root: &Path, file: &Path) -> Result<fs::Metadata, String> {
    let meta = fs::symlink_metadata(file).map_err(|e| io(e, "Cannot read", file))?;
    if meta.file_type().is_symlink() {
        return Err(format!("{} is a symbolic link, which this project format does not allow.", file.display()));
    }
    let canonical_root = fs::canonicalize(root).map_err(|e| io(e, "Cannot read", root))?;
    let canonical_file = fs::canonicalize(file).map_err(|e| io(e, "Cannot read", file))?;
    if !canonical_file.starts_with(&canonical_root) {
        return Err(format!("{} is outside the project package.", file.display()));
    }
    Ok(meta)
}

#[tauri::command]
pub fn read_package_manifest(path: String) -> Result<PackageHeader, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() { return Err(format!("{} is not a Compositor project folder.", root.display())); }
    let manifest_path = root.join("manifest.json");
    let meta = checked_metadata(&root, &manifest_path)?;
    if !meta.is_file() || meta.len() > MAX_MANIFEST { return Err("This project exceeds the supported manifest size.".into()); }
    let manifest = fs::read_to_string(&manifest_path).map_err(|e| io(e, "Cannot read", &manifest_path))?;
    let mut image_names = Vec::new();
    if let Ok(entries) = fs::read_dir(root.join("images")) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if valid_image_name(&name) && entry.file_type().map(|t| t.is_file()).unwrap_or(false) { image_names.push(name); }
        }
    }
    image_names.sort();
    Ok(PackageHeader { manifest, image_names })
}

#[tauri::command]
pub fn read_package_image(path: String, name: String) -> Result<Response, String> {
    if !valid_image_name(&name) { return Err("Invalid image name.".into()); }
    let root = PathBuf::from(&path);
    let file = root.join("images").join(&name);
    let meta = checked_metadata(&root, &file)?;
    if !meta.is_file() || meta.len() > MAX_ASSET { return Err("This project exceeds the supported asset size.".into()); }
    Ok(Response::new(fs::read(&file).map_err(|e| io(e, "Cannot read", &file))?))
}

#[derive(Default)]
pub struct PendingWrites(pub Mutex<HashMap<String, (PathBuf, PathBuf)>>); // token -> (stage, final)

#[tauri::command]
pub fn write_package_begin(path: String, pending: State<PendingWrites>) -> Result<String, String> {
    let final_path = PathBuf::from(&path);
    let stage = atomic::stage_dir(&final_path).map_err(|e| io(e, "Cannot save to", &final_path))?;
    fs::create_dir(stage.join("images")).map_err(|e| io(e, "Cannot create", &stage))?;
    let token = uuid::Uuid::new_v4().to_string();
    pending.0.lock().unwrap().insert(token.clone(), (stage, final_path));
    Ok(token)
}

fn stage_for(token: &str, pending: &State<PendingWrites>) -> Result<(PathBuf, PathBuf), String> {
    pending.0.lock().unwrap().get(token).cloned().ok_or_else(|| "Unknown save token.".to_string())
}

#[tauri::command]
pub fn write_package_manifest(token: String, manifest: String, pending: State<PendingWrites>) -> Result<(), String> {
    let (stage, _) = stage_for(&token, &pending)?;
    if manifest.len() as u64 > MAX_MANIFEST { return Err("Manifest too large.".into()); }
    let p = stage.join("manifest.json");
    fs::write(&p, manifest).map_err(|e| io(e, "Cannot write", &p))
}

#[tauri::command]
pub fn write_package_image(request: Request<'_>, pending: State<PendingWrites>) -> Result<(), String> {
    let header = |k: &str| request.headers().get(k).and_then(|v| v.to_str().ok()).map(|s| s.to_string()).ok_or_else(|| format!("missing header {k}"));
    let token = percent_decode(&header("token")?);
    let name = percent_decode(&header("name")?);
    if !valid_image_name(&name) { return Err("Invalid image name.".into()); }
    let (stage, _) = stage_for(&token, &pending)?;
    let bytes = match request.body() { tauri::ipc::InvokeBody::Raw(b) => b.clone(), _ => return Err("expected raw body".into()) };
    if bytes.len() as u64 > MAX_ASSET { return Err("Asset too large.".into()); }
    let p = stage.join("images").join(&name);
    fs::write(&p, bytes).map_err(|e| io(e, "Cannot write", &p))
}

#[tauri::command]
pub fn write_package_commit(token: String, pending: State<PendingWrites>) -> Result<(), String> {
    let (stage, final_path) = pending.0.lock().unwrap().remove(&token).ok_or_else(|| "Unknown save token.".to_string())?;
    atomic::commit(&stage, &final_path).map_err(|e| io(e, "Cannot replace", &final_path))
}

#[tauri::command]
pub fn write_package_abort(token: String, pending: State<PendingWrites>) {
    if let Some((stage, _)) = pending.0.lock().unwrap().remove(&token) { atomic::abort(&stage); }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("compositor-package-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn checked_metadata_accepts_a_plain_file_inside_the_root() {
        let root = temp();
        let file = root.join("manifest.json");
        fs::write(&file, b"{}").unwrap();
        let meta = checked_metadata(&root, &file).unwrap();
        assert!(meta.is_file());
    }

    #[test]
    fn checked_metadata_rejects_a_path_outside_the_root() {
        let root = temp();
        fs::create_dir_all(root.join("images")).unwrap();
        // A name that resolves outside `root/images` once joined and canonicalized (the
        // package root only ever contains `root` itself, so an ancestor's file is "outside").
        let outside = root.parent().unwrap().join(format!("outside-{}.png", uuid::Uuid::new_v4()));
        fs::write(&outside, b"x").unwrap();
        // Simulate what an escaping asset path would resolve to: a file that is not inside
        // `root` at all, checked against `root` as the required prefix.
        assert!(checked_metadata(&root, &outside).is_err());
    }

    // A `checked_metadata_rejects_a_symlink` test (create a symlink inside `root` pointing
    // outside it, assert `checked_metadata` rejects it) was not added: creating a filesystem
    // symlink on Windows requires either Developer Mode or the `SeCreateSymbolicLinkPrivilege`
    // right, neither of which this sandbox grants, so `std::os::windows::fs::symlink_file`
    // fails with "A required privilege is not held by the client" before the check under test
    // ever runs. The rejection itself (`meta.file_type().is_symlink()`) is a single boolean
    // check with no Windows-specific behavior, and is covered by code review; running it here
    // would require CI to grant that privilege, which is out of scope for this pass.

    #[test]
    fn image_names_are_uuid_pngs_only() {
        assert!(valid_image_name("E621E1F8-C36C-495A-93FC-0C247A3E6E5F.png"));
        assert!(valid_image_name("e621e1f8-c36c-495a-93fc-0c247a3e6e5f.mask.png"));
        assert!(!valid_image_name("../../outside.png"));
        assert!(!valid_image_name("E621E1F8-C36C-495A-93FC-0C247A3E6E5F.jpg"));
        assert!(!valid_image_name("images/E621E1F8-C36C-495A-93FC-0C247A3E6E5F.png"));
    }
}

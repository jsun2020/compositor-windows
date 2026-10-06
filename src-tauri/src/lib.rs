mod atomic;
mod commands;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::package::PendingWrites::default())
        .manage(commands::raw::RawJobs::default())
        .invoke_handler(tauri::generate_handler![
            commands::package::read_package_manifest, commands::package::read_package_image,
            commands::package::write_package_begin, commands::package::write_package_manifest,
            commands::package::write_package_image, commands::package::write_package_commit, commands::package::write_package_abort,
            commands::files::read_file, commands::files::write_file,
            commands::raw::raw_inspect, commands::raw::raw_develop, commands::raw::raw_cancel,
            commands::clipboard::read_clipboard_image, commands::clipboard::write_clipboard_image,
            commands::recent::recent_packages, commands::recent::add_recent_package,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Compositor");
}

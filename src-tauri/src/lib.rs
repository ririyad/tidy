use tidy_core::{Vault, VaultSummary};

#[tauri::command]
fn select_vault(app: tauri::AppHandle) -> Result<Option<VaultSummary>, String> {
    use tauri_plugin_dialog::DialogExt;

    let path = app
        .dialog()
        .file()
        .set_title("Choose a Tidy vault folder")
        .blocking_pick_folder();

    let Some(file_path) = path else {
        return Ok(None);
    };

    let path = file_path
        .into_path()
        .map_err(|error| format!("invalid vault path: {error}"))?;

    Vault::initialize(path).map(Some).map_err(|error| error.to_string())
}

#[tauri::command]
fn get_app_name() -> String {
    tidy_core::APP_NAME.to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![select_vault, get_app_name])
        .run(tauri::generate_context!())
        .expect("error while running Tidy");
}

use tauri::Runtime;

#[tauri::command]
pub fn start_drag<R: Runtime>(window: tauri::Window<R>) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

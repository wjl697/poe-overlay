use std::sync::{Arc, Mutex};
use notify::{Watcher, RecursiveMode, Event, Config as NotifyConfig};
use tauri::{AppHandle, Manager, Emitter};

pub struct FileWatcherState {
    pub watcher: Mutex<Option<notify::RecommendedWatcher>>,
    pub debounce_counter: Arc<Mutex<u64>>,
}

#[tauri::command]
pub fn start_file_watcher(app_handle: AppHandle, path: String) -> Result<(), String> {
    let state = app_handle.state::<FileWatcherState>();
    
    // First parse immediately when starting to watch
    if let Ok(doc) = crate::parser::parse_file(&path) {
        let _ = app_handle.emit("parsed-document", &doc);
    }
    
    let app_handle_clone = app_handle.clone();
    let path_clone = String::from(&path);
    let counter = state.debounce_counter.clone();

    let mut watcher = notify::RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| match res {
            Ok(event) => {
                // If it's an event that touches file contents
                if event.kind.is_modify() || event.kind.is_create() {
                    let next_val = {
                        let mut lock = counter.lock().unwrap();
                        *lock += 1;
                        *lock
                    };
                    let app = app_handle_clone.clone();
                    let p = path_clone.clone();
                    let c = counter.clone();
                    
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                        let current_val = *c.lock().unwrap();
                        if current_val == next_val {
                            if let Ok(doc) = crate::parser::parse_file(&p) {
                                let _ = app.emit("parsed-document", &doc);
                            }
                        }
                    });
                }
            }
            Err(e) => log::error!("watch error: {:?}", e),
        },
        NotifyConfig::default(),
    ).map_err(|e| e.to_string())?;

    // Watch path
    watcher
        .watch(std::path::Path::new(&path), RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;

    // Store in state to keep it alive
    let mut lock = state.watcher.lock().unwrap();
    *lock = Some(watcher);

    Ok(())
}

#[tauri::command]
pub fn stop_file_watcher(app_handle: AppHandle) -> Result<(), String> {
    let state = app_handle.state::<FileWatcherState>();
    let mut lock = state.watcher.lock().unwrap();
    *lock = None; // Drop watcher
    Ok(())
}

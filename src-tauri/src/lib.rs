mod win_api;
mod watcher;
mod drag;
pub mod parser;


// 1. Mouse Passthrough Command
// ==========================================
#[tauri::command]
fn set_passthrough(window: tauri::Window, passthrough: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static SESSION: AtomicUsize = AtomicUsize::new(0);

        if passthrough {
            let session = SESSION.fetch_add(1, Ordering::SeqCst) + 1;
            let window_clone = window.clone();
            
            tauri::async_runtime::spawn(async move {
                let mut top_bar_visible = false;
                while SESSION.load(Ordering::SeqCst) == session {
                    if let Ok(hwnd_raw) = window_clone.hwnd() {
                        let hwnd = windows::Win32::Foundation::HWND(hwnd_raw.0 as *mut core::ffi::c_void);
                        let mut cursor = windows::Win32::Foundation::POINT::default();
                        let mut rect = windows::Win32::Foundation::RECT::default();
                        
                        unsafe {
                            let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut cursor);
                            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect);
                        }
                        
                        // Keep hotzone static at 48px
                        let hotzone_height = 48;
                        
                        let is_in_hotzone = cursor.x >= rect.left && cursor.x <= rect.right && cursor.y >= rect.top && cursor.y <= rect.top + hotzone_height;
                        
                        if top_bar_visible != is_in_hotzone {
                            top_bar_visible = is_in_hotzone;
                            win_api::set_window_passthrough_internal(hwnd, !top_bar_visible);
                        }
                    }
                    
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                }
            });
        } else {
            SESSION.fetch_add(1, Ordering::SeqCst);
            if let Ok(hwnd_raw) = window.hwnd() {
                let hwnd = windows::Win32::Foundation::HWND(hwnd_raw.0 as *mut core::ffi::c_void);
                win_api::set_window_passthrough_internal(hwnd, false);
            }
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Mouse passthrough is only supported on Windows".into())
    }
}

// ==========================================
// 1b. Set Window Shadow Command
// ==========================================
#[tauri::command]
fn set_shadow(window: tauri::WebviewWindow, shadow: bool) {
    #[cfg(target_os = "windows")]
    win_api::set_window_shadow(&window, shadow);
    let _ = shadow; // suppress unused warning on non-windows
}

// ==========================================
// 1c. Window Controls (affects ALL windows)
// ==========================================
#[tauri::command]
fn minimize_window(app: tauri::AppHandle) {
    use tauri::Manager;
    for (_, window) in app.webview_windows() {
        let _ = window.minimize();
    }
}

#[tauri::command]
fn close_window(app: tauri::AppHandle) {
    // Close all windows first, then exit the process cleanly
    app.exit(0);
}

// ==========================================
// 1d. Open External URL
// ==========================================
#[tauri::command]
fn open_url(url: String) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", &url])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
    }
}

// ==========================================
// 2. Tauri Setup & Initialization
// ==========================================
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_store::Builder::new().build())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    .manage(watcher::FileWatcherState {
        watcher: std::sync::Mutex::new(None),
        debounce_counter: std::sync::Arc::new(std::sync::Mutex::new(0)),
    })
    .invoke_handler(tauri::generate_handler![
        set_passthrough,
        set_shadow,
        minimize_window,
        close_window,
        watcher::start_file_watcher,
        watcher::stop_file_watcher,
        drag::start_drag,
        open_url
    ])
    .setup(|app| {
      // 默认启动时为普通模式，开启系统阴影
      #[cfg(target_os = "windows")]
      {
        use tauri::Manager;
        if let Some(window) = app.get_webview_window("main") {
          win_api::set_window_shadow(&window, true);
        }
      }

      // 设置运行时窗口图标（tauri dev 模式下 bundle.icon 不会自动应用到窗口）
      {
        use tauri::Manager;
        let icon = tauri::include_image!("icons/icon.png");
        if let Some(win) = app.get_webview_window("main") {
          let _ = win.set_icon(icon);
        }
      }
      // Sync ActionBar visibility: when main window is restored, also show the ActionBar
      {
        use tauri::Manager;
        let handle = app.handle().clone();
        if let Some(main_win) = app.get_webview_window("main") {
          let _ = main_win.on_window_event(move |event| {
            match event {
              tauri::WindowEvent::Focused(true) => {
                // 当主窗口获得焦点时，确保控制条显示
                if let Some(actionbar) = handle.get_webview_window("actionbar") {
                  let _ = actionbar.unminimize();
                  let _ = actionbar.show();
                }
              }
              tauri::WindowEvent::Resized(_) => {
                // 当主窗口被最小化（例如点击任务栏图标）时，同步最小化控制条
                if let Some(main) = handle.get_webview_window("main") {
                  if let Ok(true) = main.is_minimized() {
                    if let Some(actionbar) = handle.get_webview_window("actionbar") {
                      let _ = actionbar.minimize();
                    }
                  }
                }
              }
              tauri::WindowEvent::CloseRequested { api, .. } => {
                // 当通过任务栏右键或其他系统级方式关闭主窗口时，拦截默认行为，直接退出整个应用
                api.prevent_close();
                handle.exit(0);
              }
              _ => {}
            }
          });
        }
      }

      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

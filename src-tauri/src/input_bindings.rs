use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const EVENT_PREV: &str = "action-bar-prev";
const EVENT_NEXT: &str = "action-bar-next";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InputBindingKind {
    None,
    Keyboard,
    Mouse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InputBinding {
    pub kind: InputBindingKind,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputBindingsConfig {
    pub enabled: bool,
    pub hide_action_bar_when_active: bool,
    pub prev_step: InputBinding,
    pub next_step: InputBinding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputBindingsApplyResult {
    pub prev_registered: bool,
    pub next_registered: bool,
    pub mouse_supported: bool,
    pub can_hide_action_bar: bool,
    pub action_bar_hidden: bool,
    pub errors: Vec<String>,
}

#[derive(Default)]
pub struct InputBindingState {
    pub registered_shortcuts: Mutex<Vec<String>>,
    pub action_bar_hidden: Mutex<bool>,
}

pub fn is_action_bar_hidden(state: &InputBindingState) -> bool {
    *state.action_bar_hidden.lock().unwrap()
}

#[tauri::command]
pub fn apply_input_bindings(
    app: AppHandle,
    state: State<'_, InputBindingState>,
    config: InputBindingsConfig,
) -> Result<InputBindingsApplyResult, String> {
    clear_registered_shortcuts(&app, &state);
    set_mouse_bindings(&app, None, None)?;

    if !config.enabled {
        set_action_bar_visible(&app, &state, true);
        return Ok(InputBindingsApplyResult {
            prev_registered: false,
            next_registered: false,
            mouse_supported: cfg!(target_os = "windows"),
            can_hide_action_bar: false,
            action_bar_hidden: false,
            errors: Vec::new(),
        });
    }

    if let Some(conflict) = detect_conflict(&config.prev_step, &config.next_step) {
        set_action_bar_visible(&app, &state, true);
        return Ok(InputBindingsApplyResult {
            prev_registered: false,
            next_registered: false,
            mouse_supported: cfg!(target_os = "windows"),
            can_hide_action_bar: false,
            action_bar_hidden: false,
            errors: vec![conflict],
        });
    }

    let mut errors = Vec::new();
    let mut prev_registered = false;
    let mut next_registered = false;
    let mut mouse_prev = None;
    let mut mouse_next = None;

    match register_binding(&app, &state, &config.prev_step, EVENT_PREV, "上一步") {
        BindingRegistration::Registered => prev_registered = true,
        BindingRegistration::Mouse(button) => mouse_prev = Some(button),
        BindingRegistration::Skipped => {}
        BindingRegistration::Failed(message) => errors.push(message),
    }

    match register_binding(&app, &state, &config.next_step, EVENT_NEXT, "下一步") {
        BindingRegistration::Registered => next_registered = true,
        BindingRegistration::Mouse(button) => mouse_next = Some(button),
        BindingRegistration::Skipped => {}
        BindingRegistration::Failed(message) => errors.push(message),
    }

    match set_mouse_bindings(&app, mouse_prev, mouse_next) {
        Ok(()) => {
            if mouse_prev.is_some() {
                prev_registered = true;
            }
            if mouse_next.is_some() {
                next_registered = true;
            }
        }
        Err(message) => {
            if mouse_prev.is_some() || mouse_next.is_some() {
                errors.push(message);
            }
        }
    }

    let can_hide_action_bar = prev_registered && next_registered;
    let action_bar_hidden = config.hide_action_bar_when_active && can_hide_action_bar;
    set_action_bar_visible(&app, &state, !action_bar_hidden);

    Ok(InputBindingsApplyResult {
        prev_registered,
        next_registered,
        mouse_supported: cfg!(target_os = "windows"),
        can_hide_action_bar,
        action_bar_hidden,
        errors,
    })
}

fn clear_registered_shortcuts(app: &AppHandle, state: &InputBindingState) {
    let shortcuts = {
        let mut registered = state.registered_shortcuts.lock().unwrap();
        std::mem::take(&mut *registered)
    };

    for shortcut in shortcuts {
        let _ = app.global_shortcut().unregister(shortcut.as_str());
    }
}

fn set_action_bar_visible(
    app: &AppHandle,
    state: &InputBindingState,
    visible: bool,
) {
    if let Some(actionbar) = app.get_webview_window("actionbar") {
        if visible {
            let _ = actionbar.unminimize();
            let _ = actionbar.show();
        } else {
            let _ = actionbar.hide();
        }
    }

    *state.action_bar_hidden.lock().unwrap() = !visible;
}

fn detect_conflict(left: &InputBinding, right: &InputBinding) -> Option<String> {
    let left = binding_identity(left)?;
    let right = binding_identity(right)?;

    if left == right {
        Some("上一步和下一步不能使用同一输入".into())
    } else {
        None
    }
}

fn binding_identity(binding: &InputBinding) -> Option<String> {
    match binding.kind {
        InputBindingKind::None => None,
        InputBindingKind::Keyboard => normalize_keyboard_value(&binding.value)
            .map(|value| format!("keyboard:{value}")),
        InputBindingKind::Mouse => normalize_mouse_value(&binding.value)
            .map(|value| format!("mouse:{value}")),
    }
}

enum BindingRegistration {
    Registered,
    Mouse(MouseButton),
    Skipped,
    Failed(String),
}

fn register_binding(
    app: &AppHandle,
    state: &InputBindingState,
    binding: &InputBinding,
    event_name: &'static str,
    label: &str,
) -> BindingRegistration {
    match binding.kind {
        InputBindingKind::None => BindingRegistration::Skipped,
        InputBindingKind::Keyboard => {
            let Some(shortcut) = normalize_keyboard_value(&binding.value) else {
                return BindingRegistration::Failed(format!("{label} 绑定失败：不支持的单键"));
            };

            let event_name = event_name.to_string();
            match app.global_shortcut().on_shortcut(shortcut, move |app, _, event| {
                if event.state == ShortcutState::Pressed {
                    let _ = app.emit(event_name.as_str(), ());
                }
            }) {
                Ok(()) => {
                    state
                        .registered_shortcuts
                        .lock()
                        .unwrap()
                        .push(shortcut.to_string());
                    BindingRegistration::Registered
                }
                Err(error) => BindingRegistration::Failed(format!("{label} 绑定失败：{error}")),
            }
        }
        InputBindingKind::Mouse => {
            let Some(button) = parse_mouse_button(&binding.value) else {
                return BindingRegistration::Failed(format!("{label} 绑定失败：不支持的鼠标输入"));
            };

            BindingRegistration::Mouse(button)
        }
    }
}

fn normalize_keyboard_value(value: &str) -> Option<&'static str> {
    const SUPPORTED_KEYS: &[&str] = &[
        "F1",
        "F2",
        "F3",
        "F4",
        "F5",
        "F6",
        "F7",
        "F8",
        "F9",
        "F10",
        "F11",
        "F12",
        "KeyA",
        "KeyB",
        "KeyC",
        "KeyD",
        "KeyE",
        "KeyF",
        "KeyG",
        "KeyH",
        "KeyI",
        "KeyJ",
        "KeyK",
        "KeyL",
        "KeyM",
        "KeyN",
        "KeyO",
        "KeyP",
        "KeyQ",
        "KeyR",
        "KeyS",
        "KeyT",
        "KeyU",
        "KeyV",
        "KeyW",
        "KeyX",
        "KeyY",
        "KeyZ",
        "Digit0",
        "Digit1",
        "Digit2",
        "Digit3",
        "Digit4",
        "Digit5",
        "Digit6",
        "Digit7",
        "Digit8",
        "Digit9",
        "Numpad0",
        "Numpad1",
        "Numpad2",
        "Numpad3",
        "Numpad4",
        "Numpad5",
        "Numpad6",
        "Numpad7",
        "Numpad8",
        "Numpad9",
        "Space",
        "Tab",
        "Enter",
        "Backspace",
        "Escape",
        "PageUp",
        "PageDown",
        "Home",
        "End",
        "Insert",
        "Delete",
        "ArrowUp",
        "ArrowDown",
        "ArrowLeft",
        "ArrowRight",
    ];

    if SUPPORTED_KEYS.contains(&value) {
        return Some(match value {
            "F1" => "F1",
            "F2" => "F2",
            "F3" => "F3",
            "F4" => "F4",
            "F5" => "F5",
            "F6" => "F6",
            "F7" => "F7",
            "F8" => "F8",
            "F9" => "F9",
            "F10" => "F10",
            "F11" => "F11",
            "F12" => "F12",
            "KeyA" => "KeyA",
            "KeyB" => "KeyB",
            "KeyC" => "KeyC",
            "KeyD" => "KeyD",
            "KeyE" => "KeyE",
            "KeyF" => "KeyF",
            "KeyG" => "KeyG",
            "KeyH" => "KeyH",
            "KeyI" => "KeyI",
            "KeyJ" => "KeyJ",
            "KeyK" => "KeyK",
            "KeyL" => "KeyL",
            "KeyM" => "KeyM",
            "KeyN" => "KeyN",
            "KeyO" => "KeyO",
            "KeyP" => "KeyP",
            "KeyQ" => "KeyQ",
            "KeyR" => "KeyR",
            "KeyS" => "KeyS",
            "KeyT" => "KeyT",
            "KeyU" => "KeyU",
            "KeyV" => "KeyV",
            "KeyW" => "KeyW",
            "KeyX" => "KeyX",
            "KeyY" => "KeyY",
            "KeyZ" => "KeyZ",
            "Digit0" => "Digit0",
            "Digit1" => "Digit1",
            "Digit2" => "Digit2",
            "Digit3" => "Digit3",
            "Digit4" => "Digit4",
            "Digit5" => "Digit5",
            "Digit6" => "Digit6",
            "Digit7" => "Digit7",
            "Digit8" => "Digit8",
            "Digit9" => "Digit9",
            "Numpad0" => "Numpad0",
            "Numpad1" => "Numpad1",
            "Numpad2" => "Numpad2",
            "Numpad3" => "Numpad3",
            "Numpad4" => "Numpad4",
            "Numpad5" => "Numpad5",
            "Numpad6" => "Numpad6",
            "Numpad7" => "Numpad7",
            "Numpad8" => "Numpad8",
            "Numpad9" => "Numpad9",
            "Space" => "Space",
            "Tab" => "Tab",
            "Enter" => "Enter",
            "Backspace" => "Backspace",
            "Escape" => "Escape",
            "PageUp" => "PageUp",
            "PageDown" => "PageDown",
            "Home" => "Home",
            "End" => "End",
            "Insert" => "Insert",
            "Delete" => "Delete",
            "ArrowUp" => "ArrowUp",
            "ArrowDown" => "ArrowDown",
            "ArrowLeft" => "ArrowLeft",
            "ArrowRight" => "ArrowRight",
            _ => unreachable!(),
        });
    }

    match value {
        _ => None,
    }
}

fn normalize_mouse_value(value: &str) -> Option<&'static str> {
    match value {
        "side1" => Some("side1"),
        "side2" => Some("side2"),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MouseButton {
    Side1,
    Side2,
}

fn parse_mouse_button(value: &str) -> Option<MouseButton> {
    match normalize_mouse_value(value) {
        Some("side1") => Some(MouseButton::Side1),
        Some("side2") => Some(MouseButton::Side2),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn set_mouse_bindings(
    app: &AppHandle,
    prev: Option<MouseButton>,
    next: Option<MouseButton>,
) -> Result<(), String> {
    windows_mouse::set_mouse_bindings(app.clone(), prev, next)
}

#[cfg(not(target_os = "windows"))]
fn set_mouse_bindings(
    _app: &AppHandle,
    prev: Option<MouseButton>,
    next: Option<MouseButton>,
) -> Result<(), String> {
    if prev.is_some() || next.is_some() {
        Err("当前平台不支持鼠标侧键绑定".into())
    } else {
        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod windows_mouse {
    use std::sync::{Mutex, OnceLock};

    use tauri::{AppHandle, Emitter};
    use core::ffi::c_void;

    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, HC_ACTION, HHOOK, MSLLHOOKSTRUCT, SetWindowsHookExW, UnhookWindowsHookEx,
        WH_MOUSE_LL, WM_XBUTTONDOWN, XBUTTON1, XBUTTON2,
    };

    use super::{MouseButton, EVENT_NEXT, EVENT_PREV};

    #[derive(Default)]
    struct MouseHookState {
        hook: Option<isize>,
        app: Option<AppHandle>,
        prev: Option<MouseButton>,
        next: Option<MouseButton>,
    }

    fn state() -> &'static Mutex<MouseHookState> {
        static STATE: OnceLock<Mutex<MouseHookState>> = OnceLock::new();
        STATE.get_or_init(|| Mutex::new(MouseHookState::default()))
    }

    pub fn set_mouse_bindings(
        app: AppHandle,
        prev: Option<MouseButton>,
        next: Option<MouseButton>,
    ) -> Result<(), String> {
        let mut guard = state().lock().unwrap();

        guard.prev = prev;
        guard.next = next;
        guard.app = Some(app);

        if prev.is_none() && next.is_none() {
            if let Some(raw_hook) = guard.hook.take() {
                let _ = unsafe { UnhookWindowsHookEx(HHOOK(raw_hook as *mut c_void)) };
            }
            return Ok(());
        }

        if guard.hook.is_none() {
            let hook = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), None, 0) }
                .map_err(|error| error.to_string())?;
            guard.hook = Some(hook.0 as isize);
        }

        Ok(())
    }

    unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 && wparam.0 as u32 == WM_XBUTTONDOWN {
            let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let button = match hiword(info.mouseData) {
                value if value == XBUTTON1 => Some(MouseButton::Side1),
                value if value == XBUTTON2 => Some(MouseButton::Side2),
                _ => None,
            };

            if let Some(button) = button {
                if let Ok(guard) = state().lock() {
                    if let Some(app) = guard.app.as_ref() {
                        let event = if guard.prev == Some(button) {
                            Some(EVENT_PREV)
                        } else if guard.next == Some(button) {
                            Some(EVENT_NEXT)
                        } else {
                            None
                        };

                        if let Some(event) = event {
                            let _ = app.emit(event, ());
                        }
                    }
                }
            }
        }

        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    fn hiword(value: u32) -> u16 {
        ((value >> 16) & 0xFFFF) as u16
    }
}

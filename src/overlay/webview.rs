use std::sync::mpsc::Sender;
use std::collections::HashMap;
use std::sync::Mutex;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Com::StructuredStorage::*;
use webview2_com::*;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use crate::events::AppEvent;
use base64::{Engine as _, engine::general_purpose};
use crate::overlay::webview_env;

struct ComSafe<T>(T);
unsafe impl<T> Send for ComSafe<T> {}
unsafe impl<T> Sync for ComSafe<T> {}

lazy_static::lazy_static! {
    static ref OVERLAY_CONTROLLERS: Mutex<HashMap<isize, ComSafe<ICoreWebView2Controller>>> = Mutex::new(HashMap::new());
}

pub fn register_controller(hwnd: HWND, controller: ICoreWebView2Controller) {
    if let Ok(mut lock) = OVERLAY_CONTROLLERS.lock() {
        lock.insert(hwnd.0 as isize, ComSafe(controller));
    }
}

pub fn handle_overlay_message<F>(json: &str, sender: &Sender<AppEvent>, settings: &crate::settings::Settings, post_message: F) 
where F: FnOnce(&str) {
    if json.contains("\"action\":\"dismiss\"") {
        let _ = sender.send(AppEvent::UserDismissed);
    } else if json.contains("\"action\":\"ready\"") {
        let mode_str = match settings.mode {
            crate::settings::BreakMode::Soft => "soft",
            crate::settings::BreakMode::Hard => "hard",
        };
        let anim_path = settings.overlay_animation.clone();
        let final_media_path = if anim_path == "default.webm" || !anim_path.contains(std::path::MAIN_SEPARATOR) {
            format!("https://pausecat.app/assets/{}", anim_path)
        } else {
            format!("https://pausecat.app/local/{}", general_purpose::STANDARD.encode(&anim_path))
        };
        let messages_json = serde_json::to_string(&settings.break_messages).unwrap_or_else(|_| "[]".to_string());
        let init_msg = format!(
            "{{\"action\":\"init\", \"duration\": {}, \"mode\": \"{}\", \"mediaPath\": \"{}\", \"isDark\": {}, \"bubbleOpacity\": {}, \"bubbleSize\": {}, \"bubblePosX\": {}, \"bubblePosY\": {}, \"animationStyle\": \"{}\", \"breakMessages\": {}, \"randomizeMessages\": {}, \"showWorkStatus\": {}, \"workDurationSecs\": {}, \"breakStyle\": \"{}\", \"customText\": \"{}\"}}",
            settings.break_duration_secs, mode_str, final_media_path, crate::system::is_dark_mode(),
            settings.bubble_opacity, settings.bubble_size, settings.bubble_pos_x, settings.bubble_pos_y,
            settings.animation_style, messages_json, settings.randomize_messages, settings.show_work_duration_status, settings.work_duration_secs,
            settings.break_style, settings.custom_text.replace("\"", "\\\"")
        );
        post_message(&init_msg);
    }
}

pub fn handle_resource_request(uri: &str, assets_path: &std::path::Path) -> Option<(Vec<u8>, String)> {
    if uri.starts_with("https://pausecat.app/") {
        let path_part = uri.trim_start_matches("https://pausecat.app/");
        let target_path = if path_part.starts_with("assets/") {
            assets_path.join(path_part.trim_start_matches("assets/"))
        } else if path_part.starts_with("local/") {
            let encoded = path_part.trim_start_matches("local/");
            if let Ok(path_bytes) = general_purpose::STANDARD.decode(encoded) {
                std::path::PathBuf::from(String::from_utf8(path_bytes).unwrap_or_default())
            } else { std::path::PathBuf::new() }
        } else { std::path::PathBuf::new() };

        if target_path.exists() && target_path.is_file() {
            if let Ok(content) = std::fs::read(&target_path) {
                let ext = target_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let mime = match ext.to_lowercase().as_str() {
                    "ico" => "image/x-icon", "webm" => "video/webm", "mp4" => "video/mp4",
                    "png" => "image/png", "jpg" | "jpeg" => "image/jpeg", "gif" => "image/gif",
                    _ => "application/octet-stream",
                };
                return Some((content, mime.to_string()));
            }
        }
    }
    None
}

pub fn on_overlay_controller_completed(result: windows::core::Result<()>, controller: Option<ICoreWebView2Controller>, hwnd: HWND) -> windows::core::Result<()> {
    result?;
    let controller = controller.ok_or_else(|| windows::core::Error::from_hresult(HRESULT(-1)))?;
    let _ = unsafe { controller.SetIsVisible(true) };
    let mut rect = RECT::default();
    unsafe { let _ = GetClientRect(hwnd, &mut rect); }
    let _ = unsafe { controller.SetBounds(rect) };
    register_controller(hwnd, controller);
    Ok(())
}

const OVERLAY_ANTI_ZOOM_SCRIPT: &str = "
    window.addEventListener('wheel', function(e) { if (e.ctrlKey) e.preventDefault(); }, { passive: false });
    window.addEventListener('keydown', function(e) { if (e.ctrlKey && (e.key === '+' || e.key === '-' || e.key === '0' || e.key === '=')) e.preventDefault(); });
    document.addEventListener('touchstart', function(e) { if (e.touches.length > 1) e.preventDefault(); }, { passive: false });
    document.addEventListener('gesturestart', function(e) { e.preventDefault(); });
";

pub fn init(hwnd: HWND, settings: crate::settings::Settings) -> windows::core::Result<()> {
    let env = webview_env::get_global_env().ok_or_else(|| windows::core::Error::from_hresult(HRESULT(-1)))?;
    let env_inner = env.clone();
    unsafe {
        env.CreateCoreWebView2Controller(hwnd, 
            &CreateCoreWebView2ControllerCompletedHandler::create(
                Box::new(move |result, controller| {
                    on_overlay_controller_completed(result, controller, hwnd)?;
                    if let Ok(lock) = OVERLAY_CONTROLLERS.lock() {
                        if let Some(safe_controller) = lock.get(&(hwnd.0 as isize)) {
                            let webview = safe_controller.0.CoreWebView2()?;
                            let ws = webview.Settings()?;
                            let _ = ws.SetIsWebMessageEnabled(true);
                            let _ = ws.SetAreDefaultContextMenusEnabled(false);
                            let _ = ws.SetAreDevToolsEnabled(false);
                            let _ = ws.SetIsZoomControlEnabled(false);
                            let _ = ws.SetIsStatusBarEnabled(false);
                            let _ = webview.AddScriptToExecuteOnDocumentCreated(&HSTRING::from(OVERLAY_ANTI_ZOOM_SCRIPT), None);
                            let assets_path = webview_env::get_assets_path();
                            let env_res = env_inner.clone();
                            let _ = webview.AddWebResourceRequestedFilter(w!("https://pausecat.app/*"), COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL);
                            let _ = webview.add_WebResourceRequested(&WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
                                if let (Some(args), env) = (args, &env_res) {
                                    let request = args.Request()?;
                                    let mut uri_ptr = PWSTR::null();
                                    let _ = request.Uri(&mut uri_ptr);
                                    let uri = uri_ptr.to_string().unwrap_or_default();
                                    if let Some((content, mime)) = handle_resource_request(&uri, &assets_path) {
                                        let stream = CreateStreamOnHGlobal(HGLOBAL(std::ptr::null_mut()), true)?;
                                        let _ = stream.Write(content.as_ptr() as *const _, content.len() as u32, None);
                                        let _ = stream.Seek(0, STREAM_SEEK_SET, None);
                                        let response = env.CreateWebResourceResponse(Some(&stream), 200, w!("OK"), &HSTRING::from(format!("Content-Type: {}\r\n", mime)))?;
                                        let _ = args.SetResponse(&response);
                                    }
                                    CoTaskMemFree(Some(uri_ptr.0 as *const _));
                                }
                                Ok(())
                            })), &mut 0);
                            let sender_h = GetPropW(hwnd, w!("Sender"));
                            let sender = &*(sender_h.0 as *const Sender<AppEvent>);
                            let sender_c = sender.clone();
                            let wv_c = webview.clone();
                            let settings_c = settings.clone();
                            let _ = webview.add_WebMessageReceived(&WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
                                if let Some(args) = args {
                                    let mut msg = PWSTR::null();
                                    if args.WebMessageAsJson(&mut msg).is_ok() {
                                        let json = msg.to_string().unwrap_or_default();
                                        handle_overlay_message(&json, &sender_c, &settings_c, |m| { let _ = wv_c.PostWebMessageAsJson(&HSTRING::from(m)); });
                                        CoTaskMemFree(Some(msg.0 as *const _));
                                    }
                                }
                                Ok(())
                            })), &mut 0);
                            let _ = webview.NavigateToString(&HSTRING::from(include_str!("../../assets/overlay.html")));
                        }
                    }
                    Ok(())
                })
            )
        )?;
    }
    Ok(())
}

pub fn resize_controller(hwnd: HWND) {
    if let Ok(lock) = OVERLAY_CONTROLLERS.lock() {
        if let Some(safe_controller) = lock.get(&(hwnd.0 as isize)) {
            let mut rect = RECT::default();
            unsafe { let _ = GetClientRect(hwnd, &mut rect); let _ = safe_controller.0.SetBounds(rect); }
        }
    }
}

pub fn unregister_controller(hwnd: HWND) {
    if let Ok(mut lock) = OVERLAY_CONTROLLERS.lock() { lock.remove(&(hwnd.0 as isize)); }
}

pub fn update_theme(hwnd: HWND, is_dark: bool) {
    if let Ok(lock) = OVERLAY_CONTROLLERS.lock() {
        if let Some(safe_controller) = lock.get(&(hwnd.0 as isize)) {
            if let Ok(webview) = unsafe { safe_controller.0.CoreWebView2() } {
                let _ = unsafe { webview.PostWebMessageAsJson(&HSTRING::from(format!("{{\"action\":\"theme_changed\", \"isDark\": {}}}", is_dark))) };
            }
        }
    }
}

#[cfg(test)]
mod internal_tests {
    use super::*;
    #[test]
    fn test_on_overlay_controller_completed_error() {
        let hwnd = HWND(std::ptr::null_mut());
        let res = on_overlay_controller_completed(Err(windows::core::Error::from_hresult(HRESULT(-1))), None, hwnd);
        assert!(res.is_err());
    }
    #[test]
    fn test_handle_resource_request_logic() {
        let assets_path = webview_env::get_assets_path();
        let res = handle_resource_request("https://pausecat.app/assets/pauseCat.ico", &assets_path);
        assert!(res.is_some());
        assert_eq!(res.unwrap().1, "image/x-icon");
        let local_path = assets_path.join("default.webm");
        let encoded = general_purpose::STANDARD.encode(local_path.to_str().unwrap());
        assert!(handle_resource_request(&format!("https://pausecat.app/local/{}", encoded), &assets_path).is_some());
        assert!(handle_resource_request("https://google.com", &assets_path).is_none());
    }
}

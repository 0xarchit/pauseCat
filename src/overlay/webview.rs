use std::sync::mpsc::Sender;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Com::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::UI::Shell::SHCreateStreamOnFileEx;
use webview2_com::*;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use crate::events::AppEvent;
use crate::overlay::webview_env;
use std::path::PathBuf;

struct ComSafe<T>(T);
unsafe impl<T> Send for ComSafe<T> {}
unsafe impl<T> Sync for ComSafe<T> {}

static OVERLAY_CONTROLLERS: OnceLock<Mutex<HashMap<isize, ComSafe<ICoreWebView2Controller>>>> = OnceLock::new();
static LOCAL_ASSET_REGISTRY: OnceLock<Mutex<HashMap<String, PathBuf>>> = OnceLock::new();

fn get_controllers() -> &'static Mutex<HashMap<isize, ComSafe<ICoreWebView2Controller>>> {
    OVERLAY_CONTROLLERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_asset_registry() -> &'static Mutex<HashMap<String, PathBuf>> {
    LOCAL_ASSET_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_controller(hwnd: HWND, controller: ICoreWebView2Controller) {
    if let Ok(mut lock) = get_controllers().lock() {
        lock.insert(hwnd.0 as isize, ComSafe(controller));
    }
}

pub fn handle_overlay_message<F>(_hwnd: HWND, json: &str, sender: &Sender<AppEvent>, settings: &crate::settings::Settings, post_message: F) 
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
            let path = PathBuf::from(&anim_path);
            let id = if let Some(name) = path.file_name() {
                name.to_string_lossy().to_string()
            } else {
                "custom".to_string()
            };
            if let Ok(mut lock) = get_asset_registry().lock() {
                lock.insert(id.clone(), path);
            }
            format!("https://pausecat.app/local/{}", id)
        };
        let messages_json = serde_json::to_string(&settings.break_messages).unwrap_or_else(|_| "[]".to_string());
        let init_msg = format!(
            "{{\"action\":\"init\", \"duration\": {}, \"mode\": \"{}\", \"mediaPath\": \"{}\", \"isDark\": {}, \"bubbleOpacity\": {}, \"bubbleSize\": {}, \"bubblePosX\": {}, \"bubblePosY\": {}, \"animationStyle\": \"{}\", \"breakMessages\": {}, \"randomizeMessages\": {}, \"showWorkStatus\": {}, \"workDurationSecs\": {}, \"breakStyle\": \"{}\", \"customText\": \"{}\", \"videoVolume\": {}, \"textAnimation\": \"{}\", \"textRotationX\": {}, \"textRotationY\": {}, \"textRotationZ\": {}, \"textColor\": \"{}\", \"textOpacity\": {}, \"textGlow\": {}, \"textGlowEnabled\": {}, \"textGlowColor\": \"{}\", \"textDepth\": {}, \"adaptiveTextColor\": {}}}",
            settings.break_duration_secs, mode_str, final_media_path, crate::system::is_dark_mode(),
            settings.bubble_opacity, settings.bubble_size, settings.bubble_pos_x, settings.bubble_pos_y,
            settings.animation_style, messages_json, settings.randomize_messages, settings.show_work_duration_status, settings.work_duration_secs,
            settings.break_style, settings.custom_text.replace("\"", "\\\""), settings.video_volume,
            settings.text_animation, settings.text_rotation_x, settings.text_rotation_y, settings.text_rotation_z, settings.text_color, settings.text_opacity,
            settings.text_glow, settings.text_glow_enabled, settings.text_glow_color, settings.text_depth, settings.adaptive_text_color
        );
        post_message(&init_msg);
    }
}

pub fn handle_resource_stream_request(uri: &str, assets_path: &std::path::Path) -> Option<(IStream, String)> {
    if uri.starts_with("https://pausecat.app/") {
        let path_part = uri.trim_start_matches("https://pausecat.app/");
        let file_name = if path_part.starts_with("assets/") {
            path_part.trim_start_matches("assets/")
        } else {
            ""
        };

        if !file_name.is_empty() {
            let target_path = assets_path.join(file_name);
            if target_path.exists() && target_path.is_file() {
                if let Ok(stream) = create_file_stream(&target_path) {
                    return Some((stream, get_mime_type(file_name)));
                }
            }

            if let Ok(mut exe_path) = std::env::current_exe() {
                exe_path.pop();
                let fallback_path = exe_path.join("assets").join(file_name);
                if fallback_path.exists() && fallback_path.is_file() {
                    if let Ok(stream) = create_file_stream(&fallback_path) {
                        return Some((stream, get_mime_type(file_name)));
                    }
                }
            }
        } else if path_part.starts_with("local/") {
            let id = path_part.trim_start_matches("local/");
            if let Ok(lock) = get_asset_registry().lock() {
                if let Some(target_path) = lock.get(id) {
                    if target_path.exists() && target_path.is_file() {
                        if let Ok(stream) = create_file_stream(target_path) {
                            let ext = target_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                            return Some((stream, get_mime_type(ext)));
                        }
                    }
                }
            }
        }
    }
    None
}

fn create_file_stream(path: &std::path::Path) -> windows::core::Result<IStream> {
    unsafe {
        let path_h = HSTRING::from(path.to_str().unwrap_or_default());
        SHCreateStreamOnFileEx(
            windows::core::PCWSTR(path_h.as_ptr()),
            STGM_READ.0 as u32,
            FILE_ATTRIBUTE_NORMAL.0,
            false,
            None,
        )
    }
}

fn get_mime_type(path_or_ext: &str) -> String {
    let ext = if path_or_ext.contains('.') {
        path_or_ext.split('.').last().unwrap_or("")
    } else {
        path_or_ext
    };
    
    match ext.to_lowercase().as_str() {
        "ico" => "image/x-icon",
        "webm" => "video/webm",
        "mp4" => "video/mp4",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        _ => "application/octet-stream",
    }.to_string()
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
    let env_res = env.clone();
    unsafe {
        env.CreateCoreWebView2Controller(hwnd, 
            &CreateCoreWebView2ControllerCompletedHandler::create(
                Box::new(move |result, controller| {
                    on_overlay_controller_completed(result, controller, hwnd)?;
                    if let Ok(lock) = get_controllers().lock() {
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
                            let env_inner = env_res.clone();
                            let _ = webview.AddWebResourceRequestedFilter(w!("https://pausecat.app/*"), COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL);
                            let _ = webview.add_WebResourceRequested(&WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
                                if let (Some(args), env) = (args, &env_inner) {
                                    let request = args.Request()?;
                                    let mut uri_ptr = PWSTR::null();
                                    let _ = request.Uri(&mut uri_ptr);
                                    let uri = uri_ptr.to_string().unwrap_or_default();
                                    if let Some((stream, mime)) = handle_resource_stream_request(&uri, &assets_path) {
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
                                if let (Some(args), hwnd) = (args, hwnd) {
                                    let mut msg = PWSTR::null();
                                    if args.WebMessageAsJson(&mut msg).is_ok() {
                                        let json = msg.to_string().unwrap_or_default();
                                        handle_overlay_message(hwnd, &json, &sender_c, &settings_c, |m| { let _ = wv_c.PostWebMessageAsJson(&HSTRING::from(m)); });
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
    if let Ok(lock) = get_controllers().lock() {
        if let Some(safe_controller) = lock.get(&(hwnd.0 as isize)) {
            let mut rect = RECT::default();
            unsafe { let _ = GetClientRect(hwnd, &mut rect); let _ = safe_controller.0.SetBounds(rect); }
        }
    }
}

pub fn unregister_controller(hwnd: HWND) {
    if let Ok(mut lock) = get_controllers().lock() { lock.remove(&(hwnd.0 as isize)); }
}

pub fn update_theme(hwnd: HWND, is_dark: bool) {
    if let Ok(lock) = get_controllers().lock() {
        if let Some(safe_controller) = lock.get(&(hwnd.0 as isize)) {
            if let Ok(webview) = unsafe { safe_controller.0.CoreWebView2() } {
                let _ = unsafe { webview.PostWebMessageAsJson(&HSTRING::from(format!("{{\"action\":\"theme_changed\", \"isDark\": {}}}", is_dark))) };
            }
        }
    }
}

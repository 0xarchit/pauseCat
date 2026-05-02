use std::sync::mpsc::Sender;
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::PathBuf;
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

pub fn init(hwnd: HWND, settings: crate::settings::Settings) -> windows::core::Result<()> {
    let env = webview_env::get_global_env().ok_or_else(|| windows::core::Error::from_hresult(HRESULT(-1)))?;
    let env_inner = env.clone();

    unsafe {
        env.CreateCoreWebView2Controller(hwnd, 
            &CreateCoreWebView2ControllerCompletedHandler::create(
                Box::new(move |result, controller| {
                    result?;
                    let controller = controller.ok_or_else(|| windows::core::Error::from_hresult(HRESULT(-1)))?;
                    
                    let _ = controller.SetIsVisible(true);
                    let mut rect = RECT::default();
                    let _ = GetClientRect(hwnd, &mut rect);
                    let _ = controller.SetBounds(rect);

                    register_controller(hwnd, controller.clone());
                    
                    let webview = controller.CoreWebView2()?;
                    let webview_settings = webview.Settings()?;
                    let _ = webview_settings.SetIsWebMessageEnabled(true);
                    let _ = webview_settings.SetAreDefaultContextMenusEnabled(false);
                    let _ = webview_settings.SetAreDevToolsEnabled(false);

                    let assets_path = webview_env::get_assets_path();
                    let env_resource = env_inner.clone();
                    let _ = webview.AddWebResourceRequestedFilter(w!("https://pausecat.app/*"), COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL);
                    let _ = webview.add_WebResourceRequested(
                        &WebResourceRequestedEventHandler::create(
                            Box::new(move |_, args| {
                                if let (Some(args), env) = (args, &env_resource) {
                                    let request = args.Request()?;
                                    let mut uri_ptr = PWSTR::null();
                                    let _ = request.Uri(&mut uri_ptr);
                                    let uri = uri_ptr.to_string().unwrap_or_default();
                                    
                                    if uri.starts_with("https://pausecat.app/") {
                                        let path_part = uri.trim_start_matches("https://pausecat.app/");
                                        let target_path = if path_part.starts_with("assets/") {
                                            assets_path.join(path_part.trim_start_matches("assets/"))
                                        } else if path_part.starts_with("local/") {
                                            let encoded = path_part.trim_start_matches("local/");
                                            if let Ok(path_bytes) = general_purpose::STANDARD.decode(encoded) {
                                                PathBuf::from(String::from_utf8(path_bytes).unwrap_or_default())
                                            } else { PathBuf::new() }
                                        } else { PathBuf::new() };

                                        if target_path.exists() && target_path.is_file() {
                                            let stream = match std::fs::read(&target_path) {
                                                Ok(content) => {
                                                    let stream = CreateStreamOnHGlobal(HGLOBAL(std::ptr::null_mut()), true)?;
                                                    let mut written = 0u32;
                                                    let _ = stream.Write(content.as_ptr() as *const _, content.len() as u32, Some(&mut written));
                                                    let _ = stream.Seek(0, STREAM_SEEK_SET, None);
                                                    Some(stream)
                                                }
                                                _ => None,
                                            };

                                            if let Some(stream) = stream {
                                                let ext = target_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                                                let mime = match ext.to_lowercase().as_str() {
                                                    "webm" => "video/webm",
                                                    "mp4" => "video/mp4",
                                                    "png" => "image/png",
                                                    "jpg" | "jpeg" => "image/jpeg",
                                                    "gif" => "image/gif",
                                                    _ => "application/octet-stream",
                                                };
                                                let headers = format!("Content-Type: {}\r\n", mime);
                                                let response = env.CreateWebResourceResponse(Some(&stream), 200, w!("OK"), &HSTRING::from(headers))?;
                                                let _ = args.SetResponse(&response);
                                            }
                                        }
                                    }
                                    CoTaskMemFree(Some(uri_ptr.0 as *const _));
                                }
                                Ok(())
                            })
                        ),
                        &mut 0i64
                    );

                    let sender_handle = GetPropW(hwnd, w!("Sender"));
                    let sender = &*(sender_handle.0 as *const Sender<AppEvent>);
                    let sender_clone = sender.clone();
                    let webview_clone = webview.clone();
                    let settings_clone = settings.clone();

                    let mut token = 0i64;
                    let _ = webview.add_WebMessageReceived(
                        &WebMessageReceivedEventHandler::create(
                            Box::new(move |_, args| {
                                if let Some(args) = args {
                                    let mut message = PWSTR::null();
                                    if args.WebMessageAsJson(&mut message).is_ok() {
                                        let json = message.to_string().unwrap_or_default();
                                        if json.contains("\"action\":\"dismiss\"") {
                                            let _ = sender_clone.send(AppEvent::UserDismissed);
                                        } else if json.contains("\"action\":\"ready\"") {
                                            let mode_str = match settings_clone.mode {
                                                crate::settings::BreakMode::Soft => "soft",
                                                crate::settings::BreakMode::Hard => "hard",
                                            };
                                            let anim_path = settings_clone.overlay_animation.clone();
                                            let final_media_path = if anim_path == "default.webm" || !anim_path.contains(std::path::MAIN_SEPARATOR) {
                                                format!("https://pausecat.app/assets/{}", anim_path)
                                            } else {
                                                format!("https://pausecat.app/local/{}", general_purpose::STANDARD.encode(&anim_path))
                                            };
                                            let init_msg = format!(
                                                "{{\"action\":\"init\", \"duration\": {}, \"mode\": \"{}\", \"mediaPath\": \"{}\"}}",
                                                settings_clone.break_duration_secs, mode_str, final_media_path
                                            );
                                            let _ = webview_clone.PostWebMessageAsJson(PCWSTR(HSTRING::from(init_msg).as_ptr()));
                                        }
                                        CoTaskMemFree(Some(message.0 as *const _));
                                    }
                                }
                                Ok(())
                            })
                        ),
                        &mut token
                    );

                    let html = include_str!("../../assets/overlay.html");
                    let _ = webview.NavigateToString(PCWSTR(HSTRING::from(html).as_ptr()));
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
            unsafe {
                let _ = GetClientRect(hwnd, &mut rect);
                let _ = safe_controller.0.SetBounds(rect);
            }
        }
    }
}

pub fn unregister_controller(hwnd: HWND) {
    if let Ok(mut lock) = OVERLAY_CONTROLLERS.lock() {
        lock.remove(&(hwnd.0 as isize));
    }
}

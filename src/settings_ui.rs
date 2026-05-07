use std::sync::mpsc::Sender;
use std::collections::HashMap;
use std::sync::Mutex;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Com::StructuredStorage::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::Controls::Dialogs::*;
use webview2_com::*;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use crate::events::AppEvent;
use crate::settings::Settings;
use base64::{Engine as _, engine::general_purpose};
use crate::overlay::webview_env;
use std::path::PathBuf;

struct ComSafe<T>(T);
unsafe impl<T> Send for ComSafe<T> {}
unsafe impl<T> Sync for ComSafe<T> {}

lazy_static::lazy_static! {
    static ref CONTROLLERS: Mutex<HashMap<isize, ComSafe<ICoreWebView2Controller>>> = Mutex::new(HashMap::new());
}

pub struct SettingsWindow {
    pub hwnd: HWND,
}

impl SettingsWindow {
    pub fn new(sender: Sender<AppEvent>, current_settings: Settings) -> windows::core::Result<Self> {
        unsafe {
            let instance: HINSTANCE = GetModuleHandleW(None)?.into();
            let class_name = w!("PauseCatSettingsClass");

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(settings_wnd_proc),
                hInstance: instance,
                lpszClassName: class_name,
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
                ..Default::default()
            };

            RegisterClassExW(&wnd_class);

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                w!("PauseCat Settings"),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                CW_USEDEFAULT, CW_USEDEFAULT, 540, 750,
                None, None, Some(instance), None
            )?;

            SetPropW(hwnd, w!("Sender"), Some(HANDLE(Box::into_raw(Box::new(sender)) as *mut _)))?;
            SetPropW(hwnd, w!("Settings"), Some(HANDLE(Box::into_raw(Box::new(current_settings)) as *mut _)))?;

            let is_dark = crate::system::is_dark_mode();
            crate::system::apply_immersive_dark_mode(hwnd, is_dark);

            Self::init_webview(hwnd)?;

            #[cfg(not(test))]
            let _ = ShowWindow(hwnd, SW_SHOW);
            #[cfg(test)]
            let _ = ShowWindow(hwnd, SW_HIDE);

            let _ = UpdateWindow(hwnd);

            Ok(Self { hwnd })
        }
    }

    fn init_webview(hwnd: HWND) -> windows::core::Result<()> {
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

                        CONTROLLERS.lock().unwrap().insert(hwnd.0 as isize, ComSafe(controller.clone()));

                        let webview = controller.CoreWebView2()?;
                        let webview_settings = webview.Settings()?;
                        
                        let _ = webview_settings.SetIsWebMessageEnabled(true);
                        let _ = webview_settings.SetAreDefaultContextMenusEnabled(false);
                        let _ = webview_settings.SetAreDevToolsEnabled(false);
                        let _ = webview_settings.SetIsZoomControlEnabled(false);
                        let _ = webview_settings.SetIsStatusBarEnabled(false);

                        let _ = webview.AddScriptToExecuteOnDocumentCreated(w!("
                            window.addEventListener('wheel', function(e) {
                                if (e.ctrlKey) {
                                    e.preventDefault();
                                }
                            }, { passive: false });
                            window.addEventListener('keydown', function(e) {
                                if (e.ctrlKey && (e.key === '+' || e.key === '-' || e.key === '0' || e.key === '=')) {
                                    e.preventDefault();
                                }
                            });
                            document.addEventListener('touchstart', function(e) {
                                if (e.touches.length > 1) e.preventDefault();
                            }, { passive: false });
                            document.addEventListener('gesturestart', function(e) {
                                e.preventDefault();
                            });
                        "), None);

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
                                                if let Ok(content) = std::fs::read(&target_path) {
                                                    let stream = CreateStreamOnHGlobal(HGLOBAL(std::ptr::null_mut()), true)?;
                                                    let mut written = 0u32;
                                                    let _ = stream.Write(content.as_ptr() as *const _, content.len() as u32, Some(&mut written));
                                                    let _ = stream.Seek(0, STREAM_SEEK_SET, None);
                                                    
                                                    let ext = target_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                                                    let mime = match ext.to_lowercase().as_str() {
                                                        "ico" => "image/x-icon",
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
                        
                        let mut token = 0i64;
                        let _ = webview.add_WebMessageReceived(
                            &WebMessageReceivedEventHandler::create(
                                Box::new(move |_, args| {
                                    if let Some(args) = args {
                                        let mut message = PWSTR::null();
                                        if args.WebMessageAsJson(&mut message).is_ok() {
                                            let json = message.to_string().unwrap_or_default();
                                            if json.contains("\"action\":\"save\"") {
                                                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json) {
                                                    if let Ok(new_settings) = serde_json::from_value::<Settings>(data["settings"].clone()) {
                                                        let _ = sender_clone.send(AppEvent::ConfigChanged(new_settings));
                                                        let _ = sender_clone.send(AppEvent::SettingsClosed); 
                                                    } else {
                                                        log::error!("Failed to parse Settings from JSON");
                                                    }
                                                }
                                            } else if json.contains("\"action\":\"close\"") {
                                                let _ = sender_clone.send(AppEvent::SettingsClosed);
                                            } else if json.contains("\"action\":\"get_apps\"") {
                                                let apps = crate::system::get_running_apps();
                                                let apps_json = serde_json::to_string(&apps).unwrap_or_default();
                                                let msg = format!("{{\"action\":\"apps_list\", \"apps\": {}}}", apps_json);
                                                let hmsg = HSTRING::from(msg);
                                                let _ = webview_clone.PostWebMessageAsJson(PCWSTR(hmsg.as_ptr()));
                                            } else if json.contains("\"action\":\"check_updates\"") {
                                                let _ = sender_clone.send(AppEvent::CheckForUpdates);
                                            } else if json.contains("\"action\":\"start_update\"") {
                                                let _ = sender_clone.send(AppEvent::StartUpdate);
                                            } else if json.contains("\"action\":\"select_media\"") {
                                                if let Some(path) = pick_file() {
                                                    let msg = format!("{{\"action\":\"media_selected\", \"path\":\"{}\"}}", path.replace('\\', "/"));
                                                    let hmsg = HSTRING::from(msg);
                                                    let _ = webview_clone.PostWebMessageAsJson(PCWSTR(hmsg.as_ptr()));
                                                }
                                            }
                                            CoTaskMemFree(Some(message.0 as *const _));
                                        }
                                    }
                                    Ok(())
                                })
                            ),
                            &mut token
                        );

                        let html = include_str!("../assets/settings.html");
                        let hhtml = HSTRING::from(html);
                        let _ = webview.NavigateToString(PCWSTR(hhtml.as_ptr()));

                        let settings_handle = GetPropW(hwnd, w!("Settings"));
                        let settings = &*(settings_handle.0 as *const Settings);
                        let json_settings = serde_json::to_string(settings).unwrap_or_default();
                        
                        let is_dark = crate::system::is_dark_mode();
                        let load_msg = format!("{{\"action\":\"load\", \"settings\": {}, \"isDark\": {}}}", json_settings, is_dark);
                        let hload = HSTRING::from(load_msg);
                        let _ = webview.PostWebMessageAsJson(PCWSTR(hload.as_ptr()));

                        Ok(())
                    })
                )
            )?;
        }
        Ok(())
    }

    pub fn update_theme(&self, is_dark: bool) {
        if let Ok(lock) = CONTROLLERS.lock() {
            if let Some(safe_controller) = lock.get(&(self.hwnd.0 as isize)) {
                if let Ok(webview) = unsafe { safe_controller.0.CoreWebView2() } {
                    let msg = format!("{{\"action\":\"theme_changed\", \"isDark\": {}}}", is_dark);
                    let hmsg = HSTRING::from(msg);
                    let _ = unsafe { webview.PostWebMessageAsJson(PCWSTR(hmsg.as_ptr())) };
                }
            }
        }
    }

    pub fn send_update_status(&self, info: crate::updater::UpdateInfo) {
        if let Ok(lock) = CONTROLLERS.lock() {
            if let Some(safe_controller) = lock.get(&(self.hwnd.0 as isize)) {
                if let Ok(webview) = unsafe { safe_controller.0.CoreWebView2() } {
                    let info_json = serde_json::to_string(&info).unwrap_or_default();
                    let msg = format!("{{\"action\":\"update_status\", \"info\": {}}}", info_json);
                    let hmsg = HSTRING::from(msg);
                    let _ = unsafe { webview.PostWebMessageAsJson(PCWSTR(hmsg.as_ptr())) };
                }
            }
        }
    }

    pub fn send_update_progress(&self, percentage: u32) {
        if let Ok(lock) = CONTROLLERS.lock() {
            if let Some(safe_controller) = lock.get(&(self.hwnd.0 as isize)) {
                if let Ok(webview) = unsafe { safe_controller.0.CoreWebView2() } {
                    let msg = format!("{{\"action\":\"update_progress\", \"percentage\": {}}}", percentage);
                    let hmsg = HSTRING::from(msg);
                    let _ = unsafe { webview.PostWebMessageAsJson(PCWSTR(hmsg.as_ptr())) };
                }
            }
        }
    }

    pub fn send_update_error(&self, error: String) {
        if let Ok(lock) = CONTROLLERS.lock() {
            if let Some(safe_controller) = lock.get(&(self.hwnd.0 as isize)) {
                if let Ok(webview) = unsafe { safe_controller.0.CoreWebView2() } {
                    let msg = format!("{{\"action\":\"update_error\", \"error\": \"{}\"}}", error.replace('"', "\\\""));
                    let hmsg = HSTRING::from(msg);
                    let _ = unsafe { webview.PostWebMessageAsJson(PCWSTR(hmsg.as_ptr())) };
                }
            }
        }
    }
}

fn pick_file() -> Option<String> {
    unsafe {
        let mut file_path = [0u16; 1024];
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.lpstrFile = PWSTR(file_path.as_mut_ptr());
        ofn.nMaxFile = 1024;
        ofn.lpstrFilter = w!("Media Files\0*.png;*.jpg;*.jpeg;*.gif;*.mp4;*.webm\0All Files\0*.*\0");
        ofn.nFilterIndex = 1;
        ofn.Flags = OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER;

        if GetOpenFileNameW(&mut ofn).as_bool() {
            Some(PWSTR(file_path.as_mut_ptr()).to_string().unwrap_or_default())
        } else {
            None
        }
    }
}

impl Drop for SettingsWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn settings_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_SIZE => {
            if let Ok(lock) = CONTROLLERS.lock() {
                if let Some(safe_controller) = lock.get(&(hwnd.0 as isize)) {
                    let mut rect = RECT::default();
                    let _ = GetClientRect(hwnd, &mut rect);
                    let _ = safe_controller.0.SetBounds(rect);
                }
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let sender_handle = GetPropW(hwnd, w!("Sender"));
            if !sender_handle.is_invalid() {
                let sender = unsafe { &*(sender_handle.0 as *const Sender<AppEvent>) };
                let _ = sender.send(AppEvent::SettingsClosed);
            }
            LRESULT(0) // Prevent immediate destruction, wait for Drop
        }
        WM_DESTROY => {
            let sender_handle = RemovePropW(hwnd, w!("Sender")).unwrap_or_default();
            if !sender_handle.is_invalid() {
                drop(unsafe { Box::from_raw(sender_handle.0 as *mut Sender<AppEvent>) });
            }
            
            let _ = RemovePropW(hwnd, w!("Settings")).map(|h| if !h.is_invalid() { drop(unsafe { Box::from_raw(h.0 as *mut Settings) }); });
            
            if let Ok(mut lock) = CONTROLLERS.lock() {
                lock.remove(&(hwnd.0 as isize));
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(test)]
mod internal_tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn test_settings_wnd_proc_branches() {
        let (tx, _rx) = mpsc::channel::<AppEvent>();
        let settings = Settings::default();
        let state = Box::into_raw(Box::new((tx, settings)));
        
        unsafe {
            let hwnd = HWND(std::ptr::null_mut());
            
            // Test WM_CREATE
            let cs = CREATESTRUCTW {
                lpCreateParams: state as *mut _,
                ..Default::default()
            };
            settings_wnd_proc(hwnd, WM_CREATE, WPARAM(0), LPARAM(&cs as *const _ as isize));
            
            // Test WM_PAINT
            settings_wnd_proc(hwnd, WM_PAINT, WPARAM(0), LPARAM(0));
            
            // Test WM_SIZE
            settings_wnd_proc(hwnd, WM_SIZE, WPARAM(0), LPARAM(100 | (100 << 16)));
            
            // Test WM_CLOSE
            settings_wnd_proc(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));

            // Test default match
            settings_wnd_proc(hwnd, WM_COMMAND, WPARAM(999), LPARAM(0));
            
            // Test WM_DESTROY
            settings_wnd_proc(hwnd, WM_DESTROY, WPARAM(0), LPARAM(0));
        }
    }
}

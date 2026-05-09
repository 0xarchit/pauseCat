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
use crate::overlay::webview_env;

struct ComSafe<T>(T);
unsafe impl<T> Send for ComSafe<T> {}
unsafe impl<T> Sync for ComSafe<T> {}

lazy_static::lazy_static! {
    static ref CONTROLLERS: Mutex<HashMap<isize, ComSafe<ICoreWebView2Controller>>> = Mutex::new(HashMap::new());
}

pub struct SettingsWindow {
    pub hwnd: HWND,
}

pub fn handle_settings_message<F, P>(json: &str, sender: &Sender<AppEvent>, post_message: F, pick_file_fn: P) 
where F: FnOnce(&str), P: FnOnce() -> Option<String> {
    if json.contains("\"action\":\"save\"") {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json) {
            if let Ok(new_settings) = serde_json::from_value::<Settings>(data["settings"].clone()) {
                let _ = sender.send(AppEvent::ConfigChanged(new_settings));
                let _ = sender.send(AppEvent::SettingsClosed); 
            }
        }
    } else if json.contains("\"action\":\"close\"") {
        let _ = sender.send(AppEvent::SettingsClosed);
    } else if json.contains("\"action\":\"get_apps\"") {
        let apps = crate::system::get_running_apps();
        let apps_json = serde_json::to_string(&apps).unwrap_or_default();
        post_message(&format!("{{\"action\":\"apps_list\", \"apps\": {}}}", apps_json));
    } else if json.contains("\"action\":\"check_updates\"") {
        let _ = sender.send(AppEvent::CheckForUpdates);
    } else if json.contains("\"action\":\"start_update\"") {
        let _ = sender.send(AppEvent::StartUpdate);
    } else if json.contains("\"action\":\"select_media\"") {
        if let Some(path) = pick_file_fn() {
            post_message(&format!("{{\"action\":\"media_selected\", \"path\":\"{}\"}}", path.replace('\\', "/")));
        }
    } else if json.contains("\"action\":\"retry_sync\"") {
        let _ = sender.send(AppEvent::RetryAssetSync);
    }
}

pub fn build_update_status_msg(info: &crate::updater::UpdateInfo) -> String {
    let info_json = serde_json::to_string(info).unwrap_or_default();
    format!("{{\"action\":\"update_status\", \"info\": {}}}", info_json)
}

pub fn build_update_progress_msg(percentage: u32) -> String {
    format!("{{\"action\":\"update_progress\", \"percentage\": {}}}", percentage)
}

pub fn build_update_error_msg(error: &str) -> String {
    format!("{{\"action\":\"update_error\", \"error\": \"{}\"}}", error.replace('"', "\\\""))
}

pub fn on_controller_completed(
    result: windows::core::Result<()>, 
    controller: Option<ICoreWebView2Controller>, 
    hwnd: HWND,
) -> windows::core::Result<()> {
    result?;
    let controller = controller.ok_or_else(|| windows::core::Error::from_hresult(HRESULT(-1)))?;
    let _ = unsafe { controller.SetIsVisible(true) };
    let mut rect = RECT::default();
    unsafe { let _ = GetClientRect(hwnd, &mut rect); }
    let _ = unsafe { controller.SetBounds(rect) };
    CONTROLLERS.lock().unwrap().insert(hwnd.0 as isize, ComSafe(controller));
    Ok(())
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
                WINDOW_EX_STYLE::default(), class_name, w!("PauseCat Settings"),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                CW_USEDEFAULT, CW_USEDEFAULT, 540, 750,
                None, None, Some(instance), None
            )?;
            SetPropW(hwnd, w!("Sender"), Some(HANDLE(Box::into_raw(Box::new(sender)) as *mut _)))?;
            SetPropW(hwnd, w!("Settings"), Some(HANDLE(Box::into_raw(Box::new(current_settings)) as *mut _)))?;
            crate::system::apply_immersive_dark_mode(hwnd, crate::system::is_dark_mode());
            Self::init_webview(hwnd)?;
            let _ = ShowWindow(hwnd, if cfg!(test) { SW_HIDE } else { SW_SHOW });
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
                        on_controller_completed(result, controller, hwnd)?;
                        if let Ok(lock) = CONTROLLERS.lock() {
                            if let Some(safe_controller) = lock.get(&(hwnd.0 as isize)) {
                                let webview = safe_controller.0.CoreWebView2()?;
                                let ws = webview.Settings()?;
                                let _ = (ws.SetIsWebMessageEnabled(true), ws.SetAreDefaultContextMenusEnabled(false), ws.SetAreDevToolsEnabled(false), ws.SetIsZoomControlEnabled(false), ws.SetIsStatusBarEnabled(false));
                                let assets_path = webview_env::get_assets_path();
                                let env_res = env_inner.clone();
                                let _ = webview.AddWebResourceRequestedFilter(w!("https://pausecat.app/*"), COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL);
                                let _ = webview.add_WebResourceRequested(&WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
                                    if let (Some(args), env) = (args, &env_res) {
                                        let request = args.Request()?;
                                        let mut uri_ptr = PWSTR::null();
                                        let _ = request.Uri(&mut uri_ptr);
                                        let uri = uri_ptr.to_string().unwrap_or_default();
                                        if let Some((content, mime)) = crate::overlay::webview::handle_resource_request(&uri, &assets_path) {
                                            let stream = CreateStreamOnHGlobal(HGLOBAL(std::ptr::null_mut()), true)?;
                                            let _ = (stream.Write(content.as_ptr() as *const _, content.len() as u32, None), stream.Seek(0, STREAM_SEEK_SET, None));
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
                                let _ = webview.add_WebMessageReceived(&WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
                                    if let Some(args) = args {
                                        let mut msg = PWSTR::null();
                                        if args.WebMessageAsJson(&mut msg).is_ok() {
                                            let json = msg.to_string().unwrap_or_default();
                                            handle_settings_message(&json, &sender_c, |m| { let _ = wv_c.PostWebMessageAsJson(&HSTRING::from(m)); }, pick_file);
                                            CoTaskMemFree(Some(msg.0 as *const _));
                                        }
                                    }
                                    Ok(())
                                })), &mut 0);

                                let assets_path = webview_env::get_assets_path();
                                let env_res = env_inner.clone();
                                let _ = webview.AddWebResourceRequestedFilter(w!("https://pausecat.app/*"), COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL);
                                let _ = webview.add_WebResourceRequested(&WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
                                    if let (Some(args), env) = (args, &env_res) {
                                        let request = args.Request()?;
                                        let mut uri_ptr = PWSTR::null();
                                        let _ = request.Uri(&mut uri_ptr);
                                        let uri = uri_ptr.to_string().unwrap_or_default();
                                        if let Some((content, mime)) = crate::overlay::webview::handle_resource_request(&uri, &assets_path) {
                                            let stream = CreateStreamOnHGlobal(HGLOBAL(std::ptr::null_mut()), true)?;
                                            let _ = (stream.Write(content.as_ptr() as *const _, content.len() as u32, None), stream.Seek(0, STREAM_SEEK_SET, None));
                                            let response = env.CreateWebResourceResponse(Some(&stream), 200, w!("OK"), &HSTRING::from(format!("Content-Type: {}\r\n", mime)))?;
                                            let _ = args.SetResponse(&response);
                                        }
                                        CoTaskMemFree(Some(uri_ptr.0 as *const _));
                                    }
                                    Ok(())
                                })), &mut 0);

                                let _ = webview.NavigateToString(&HSTRING::from(include_str!("../assets/settings.html")));
                                let settings_h = GetPropW(hwnd, w!("Settings"));
                                let settings = &*(settings_h.0 as *const Settings);
                                
                                let mut asset_path = webview_env::get_assets_path();
                                asset_path.push("default.webm");
                                let asset_ready = asset_path.exists() && asset_path.metadata().map(|m| m.len() > 0).unwrap_or(false);

                                let logo_path = "https://pausecat.app/assets/pauseCat.ico";

                                let msg = format!(
                                    "{{\"action\":\"load\", \"settings\": {}, \"isDark\": {}, \"version\": \"{}\", \"assetReady\": {}, \"logoPath\": \"{}\"}}", 
                                    serde_json::to_string(settings).unwrap_or_default(), 
                                    crate::system::is_dark_mode(),
                                    env!("CARGO_PKG_VERSION"),
                                    asset_ready,
                                    logo_path
                                );
                                let _ = webview.PostWebMessageAsJson(&HSTRING::from(msg));
                            }
                        }
                        Ok(())
                    })
                )
            )?;
        }
        Ok(())
    }

    pub fn post_web_message(&self, msg: &str) {
        if let Ok(lock) = CONTROLLERS.lock() {
            if let Some(safe_controller) = lock.get(&(self.hwnd.0 as isize)) {
                if let Ok(webview) = unsafe { safe_controller.0.CoreWebView2() } {
                    let _ = unsafe { webview.PostWebMessageAsJson(&HSTRING::from(msg)) };
                }
            }
        }
    }

    pub fn update_theme(&self, is_dark: bool) { self.post_web_message(&format!("{{\"action\":\"theme_changed\", \"isDark\": {}}}", is_dark)); }
    pub fn send_update_status(&self, info: crate::updater::UpdateInfo) { self.post_web_message(&build_update_status_msg(&info)); }
    pub fn send_update_progress(&self, percentage: u32) { self.post_web_message(&build_update_progress_msg(percentage)); }
    pub fn send_update_error(&self, error: String) { self.post_web_message(&build_update_error_msg(&error)); }
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
        if GetOpenFileNameW(&mut ofn).as_bool() { Some(PWSTR(file_path.as_mut_ptr()).to_string().unwrap_or_default()) } else { None }
    }
}

impl Drop for SettingsWindow { fn drop(&mut self) { unsafe { let _ = DestroyWindow(self.hwnd); } } }

unsafe extern "system" fn settings_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_SIZE => {
            if let Ok(lock) = CONTROLLERS.lock() {
                if let Some(safe_controller) = lock.get(&(hwnd.0 as isize)) {
                    let mut rect = RECT::default();
                    let _ = (GetClientRect(hwnd, &mut rect), safe_controller.0.SetBounds(rect));
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
            LRESULT(0)
        }
        WM_DESTROY => {
            let sender_handle = RemovePropW(hwnd, w!("Sender")).unwrap_or_default();
            if !sender_handle.is_invalid() { drop(unsafe { Box::from_raw(sender_handle.0 as *mut Sender<AppEvent>) }); }
            let _ = RemovePropW(hwnd, w!("Settings")).map(|h| if !h.is_invalid() { drop(unsafe { Box::from_raw(h.0 as *mut Settings) }); });
            if let Ok(mut lock) = CONTROLLERS.lock() { lock.remove(&(hwnd.0 as isize)); }
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
    fn test_handle_settings_message_logic() {
        let (tx, rx) = std::sync::mpsc::channel();
        let settings = Settings::default();
        let settings_json = serde_json::to_string(&settings).unwrap();
        let json = format!("{{\"action\":\"save\", \"settings\": {}}}", settings_json);
        handle_settings_message(&json, &tx, |_| {}, || None);
        assert!(matches!(rx.try_recv(), Ok(AppEvent::ConfigChanged(_))));
        assert!(matches!(rx.try_recv(), Ok(AppEvent::SettingsClosed)));
        handle_settings_message("{\"action\":\"close\"}", &tx, |_| {}, || None);
        assert!(matches!(rx.try_recv(), Ok(AppEvent::SettingsClosed)));
        handle_settings_message("{\"action\":\"check_updates\"}", &tx, |_| {}, || None);
        assert!(matches!(rx.try_recv(), Ok(AppEvent::CheckForUpdates)));
        handle_settings_message("{\"action\":\"start_update\"}", &tx, |_| {}, || None);
        assert!(matches!(rx.try_recv(), Ok(AppEvent::StartUpdate)));
        handle_settings_message("{\"action\":\"get_apps\"}", &tx, |msg| { assert!(msg.contains("\"action\":\"apps_list\"")); }, || None);
        handle_settings_message("{\"action\":\"select_media\"}", &tx, |msg| {
            assert!(msg.contains("\"action\":\"media_selected\""));
            assert!(msg.contains("test/path.png"));
        }, || Some("test\\path.png".to_string()));
    }
    #[test]
    fn test_message_builders() {
        let info = crate::updater::UpdateInfo { available: true, latest_version: "v1".to_string(), changelog: "notes".to_string() };
        assert!(build_update_status_msg(&info).contains("update_status"));
        assert!(build_update_progress_msg(50).contains("50"));
        assert!(build_update_error_msg("test \"error\"").contains("test \\\"error\\\""));
    }
    #[test]
    fn test_on_controller_completed_error() {
        let hwnd = HWND(std::ptr::null_mut());
        let res = on_controller_completed(Err(windows::core::Error::from_hresult(HRESULT(-1))), None, hwnd);
        assert!(res.is_err());
    }
    #[test]
    fn test_settings_wnd_proc_branches() {
        let (tx, _rx) = mpsc::channel::<AppEvent>();
        let settings = Settings::default();
        let state = Box::into_raw(Box::new((tx, settings)));
        unsafe {
            let hwnd = HWND(std::ptr::null_mut());
            let cs = CREATESTRUCTW { lpCreateParams: state as *mut _, ..Default::default() };
            settings_wnd_proc(hwnd, WM_CREATE, WPARAM(0), LPARAM(&cs as *const _ as isize));
            // Test common UI messages
            settings_wnd_proc(hwnd, WM_PAINT, WPARAM(0), LPARAM(0));
            settings_wnd_proc(hwnd, WM_ERASEBKGND, WPARAM(0), LPARAM(0));
            settings_wnd_proc(hwnd, WM_SETFOCUS, WPARAM(0), LPARAM(0));
            settings_wnd_proc(hwnd, WM_KILLFOCUS, WPARAM(0), LPARAM(0));
            settings_wnd_proc(hwnd, WM_MOVE, WPARAM(0), LPARAM(0));
            settings_wnd_proc(hwnd, WM_ACTIVATE, WPARAM(0), LPARAM(0));
            
            settings_wnd_proc(hwnd, WM_SIZE, WPARAM(0), LPARAM(100 | (100 << 16)));
            settings_wnd_proc(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
            settings_wnd_proc(hwnd, WM_COMMAND, WPARAM(999), LPARAM(0));
            settings_wnd_proc(hwnd, WM_DESTROY, WPARAM(0), LPARAM(0));
        }
    }
}

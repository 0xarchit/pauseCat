use std::sync::mpsc::Sender;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::Controls::Dialogs::*;
use webview2_com::*;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use crate::events::AppEvent;
use crate::settings::Settings;

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
                hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as *mut _),
                ..Default::default()
            };

            RegisterClassExW(&wnd_class);

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                w!("PauseCat Settings"),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                CW_USEDEFAULT, CW_USEDEFAULT, 450, 650,
                None, None, Some(instance), None
            )?;

            // Store sender in window property
            let sender_ptr = Box::into_raw(Box::new(sender));
            SetPropW(hwnd, w!("Sender"), Some(HANDLE(sender_ptr as *mut _)))?;

            // Store current settings in window property
            let settings_ptr = Box::into_raw(Box::new(current_settings));
            SetPropW(hwnd, w!("Settings"), Some(HANDLE(settings_ptr as *mut _)))?;

            Self::init_webview(hwnd)?;

            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = UpdateWindow(hwnd);

            Ok(Self { hwnd })
        }
    }

    fn init_webview(hwnd: HWND) -> windows::core::Result<()> {
        unsafe {
            CreateCoreWebView2EnvironmentWithOptions(None, None, None, 
                &CreateCoreWebView2EnvironmentCompletedHandler::create(
                    Box::new(move |result, env| {
                        result?;
                        let env = env.ok_or_else(|| windows::core::Error::from_hresult(HRESULT(-1)))?;
                        
                        env.CreateCoreWebView2Controller(hwnd, 
                            &CreateCoreWebView2ControllerCompletedHandler::create(
                                Box::new(move |result, controller| {
                                    result?;
                                    let controller = controller.ok_or_else(|| windows::core::Error::from_hresult(HRESULT(-1)))?;
                                    
                                    // Store controller in window property
                                    let controller_raw = controller.as_raw();
                                    let _ = SetPropW(hwnd, w!("Controller"), Some(HANDLE(controller_raw as *mut _)));
                                    
                                    let mut rect = RECT::default();
                                    let _ = GetClientRect(hwnd, &mut rect);
                                    let _ = controller.Bounds(&mut rect);

                                    let webview = controller.CoreWebView2()?;
                                    let webview_settings = webview.Settings()?;
                                    let _ = webview_settings.SetIsWebMessageEnabled(true);

                                    let sender_handle = GetPropW(hwnd, w!("Sender"));
                                    if !sender_handle.is_invalid() {
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
                                                                        let _ = sender_clone.send(AppEvent::UserDismissed); 
                                                                    }
                                                                }
                                                            } else if json.contains("\"action\":\"close\"") {
                                                                let _ = sender_clone.send(AppEvent::UserDismissed);
                                                            } else if json.contains("\"action\":\"select_media\"") {
                                                                if let Some(path) = Self::pick_file() {
                                                                    let path_json = path.replace('\\', "/");
                                                                    let msg = format!("{{\"action\":\"media_selected\", \"path\":\"{}\"}}", path_json);
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
                                    }

                                    let html = include_str!("../assets/settings.html");
                                    let hhtml = HSTRING::from(html);
                                    let _ = webview.NavigateToString(PCWSTR(hhtml.as_ptr()));

                                    let settings_handle = GetPropW(hwnd, w!("Settings"));
                                    if !settings_handle.is_invalid() {
                                        let settings = &*(settings_handle.0 as *const Settings);
                                        let json_settings = serde_json::to_string(settings).unwrap_or_default();
                                        let msg = format!("{{\"action\":\"load\", \"settings\": {}}}", json_settings);
                                        let hmsg = HSTRING::from(msg);
                                        let _ = webview.PostWebMessageAsJson(PCWSTR(hmsg.as_ptr()));
                                    }

                                    Ok(())
                                })
                            )
                        )?;
                        Ok(())
                    })
                )
            )?;
        }
        Ok(())
    }

    fn pick_file() -> Option<String> {
        unsafe {
            let mut file_path = [0u16; 260];
            let mut ofn = OPENFILENAMEW::default();
            ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
            ofn.lpstrFile = PWSTR(file_path.as_mut_ptr());
            ofn.nMaxFile = 260;
            ofn.lpstrFilter = w!("Media Files\0*.png;*.jpg;*.jpeg;*.gif;*.mp4;*.webm\0All Files\0*.*\0");
            ofn.nFilterIndex = 1;
            ofn.Flags = OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST | OFN_NOCHANGEDIR;

            if GetOpenFileNameW(&mut ofn).as_bool() {
                Some(PWSTR(file_path.as_mut_ptr()).to_string().unwrap_or_default())
            } else {
                None
            }
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
            let controller_handle = GetPropW(hwnd, w!("Controller"));
            if !controller_handle.is_invalid() {
                let controller: ICoreWebView2Controller = std::mem::transmute_copy(&(controller_handle.0 as *const ()));
                let mut rect = RECT::default();
                let _ = GetClientRect(hwnd, &mut rect);
                let _ = controller.Bounds(&mut rect);
                std::mem::forget(controller);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let sender_handle = RemovePropW(hwnd, w!("Sender"));
            if let Ok(h) = sender_handle {
                if !h.is_invalid() { drop(Box::from_raw(h.0 as *mut Sender<AppEvent>)); }
            }
            
            let settings_handle = RemovePropW(hwnd, w!("Settings"));
            if let Ok(h) = settings_handle {
                if !h.is_invalid() { drop(Box::from_raw(h.0 as *mut Settings)); }
            }

            let controller_handle = RemovePropW(hwnd, w!("Controller"));
            if let Ok(h) = controller_handle {
                if !h.is_invalid() {
                    let controller: ICoreWebView2Controller = std::mem::transmute_copy(&(h.0 as *const ()));
                    drop(controller);
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

use std::sync::mpsc::Sender;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Com::*;
use webview2_com::*;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use crate::events::AppEvent;

/// Manages the WebView2 instance for the break overlay.
pub struct WebViewLayer;

impl WebViewLayer {
    /// Starts the asynchronous initialization of WebView2.
    pub fn init(hwnd: HWND, sender: Sender<AppEvent>) -> windows::core::Result<()> {
        unsafe {
            // Ensure COM is initialized for this thread
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

            CreateCoreWebView2EnvironmentWithOptions(None, None, None, 
                &CreateCoreWebView2EnvironmentCompletedHandler::create(
                    Box::new(move |result: windows::core::Result<()>, env: Option<ICoreWebView2Environment>| {
                        result?;
                        let env = env.ok_or_else(|| windows::core::Error::from_hresult(HRESULT(-1)))?;
                        
                        env.CreateCoreWebView2Controller(hwnd, 
                            &CreateCoreWebView2ControllerCompletedHandler::create(
                                Box::new(move |result: windows::core::Result<()>, controller: Option<ICoreWebView2Controller>| {
                                    result?;
                                    let controller = controller.ok_or_else(|| windows::core::Error::from_hresult(HRESULT(-1)))?;
                                    
                                    // 1. Configure transparency
                                    let config: ICoreWebView2Controller2 = controller.cast()?;
                                    let mut color = COREWEBVIEW2_COLOR { A: 0, R: 0, G: 0, B: 0 };
                                    let _ = config.DefaultBackgroundColor(&mut color);

                                    // 2. Set initial bounds
                                    let mut rect = RECT::default();
                                    let _ = GetClientRect(hwnd, &mut rect);
                                    let _ = controller.Bounds(&mut rect);

                                    let webview = controller.CoreWebView2()?;
                                    
                                    // 3. Configure settings
                                    let settings = webview.Settings()?;
                                    let _ = settings.SetIsWebMessageEnabled(true);
                                    let _ = settings.SetAreDefaultContextMenusEnabled(false);
                                    let _ = settings.SetAreDevToolsEnabled(false);

                                    // 4. Handle messages (JS -> Rust)
                                    let sender_clone = sender.clone();
                                    let mut token = 0i64;
                                    let _ = webview.add_WebMessageReceived(
                                        &WebMessageReceivedEventHandler::create(
                                            Box::new(move |_, args: Option<ICoreWebView2WebMessageReceivedEventArgs>| {
                                                if let Some(args) = args {
                                                    let mut message = PWSTR::null();
                                                    if args.WebMessageAsJson(&mut message).is_ok() {
                                                        let json = message.to_string().unwrap_or_default();
                                                        if json.contains("\"action\":\"dismiss\"") {
                                                            let _ = sender_clone.send(AppEvent::UserDismissed);
                                                        }
                                                        CoTaskMemFree(Some(message.0 as *const _));
                                                    }
                                                }
                                                Ok(())
                                            })
                                        ),
                                        &mut token
                                    );

                                    // 5. Navigate to embedded HTML
                                    let html = include_str!("../../assets/overlay.html");
                                    let hhtml = HSTRING::from(html);
                                    let _ = webview.NavigateToString(PCWSTR(hhtml.as_ptr()));

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
}

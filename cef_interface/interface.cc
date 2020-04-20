#include "interface.hh"

#include <chrono>    // std::chrono::seconds
#include <iostream>  // std::cout, std::endl
#include <thread>    // std::this_thread::sleep_for

#include "app.hh"
#include "client.hh"

extern "C" RustRefApp cef_interface_add_ref_app(MyApp* ptr) {
  ptr->AddRef();

  RustRefApp r;
  r.ptr = ptr;
  return r;
}
extern "C" int cef_interface_release_ref_app(MyApp* app_ptr) {
  app_ptr->Release();
  return 0;
}

extern "C" RustRefClient cef_interface_add_ref_client(MyClient* ptr) {
  ptr->AddRef();

  RustRefClient r;
  r.ptr = ptr;
  return r;
}
extern "C" int cef_interface_release_ref_client(MyClient* client_ptr) {
  client_ptr->Release();
  return 0;
}

extern "C" RustRefBrowser cef_interface_add_ref_browser(CefBrowser* ptr) {
  ptr->AddRef();

  RustRefBrowser r;
  r.ptr = ptr;
  return r;
}
extern "C" int cef_interface_release_ref_browser(CefBrowser* browser_ptr) {
  browser_ptr->Release();
  return 0;
}

extern "C" RustRefApp cef_interface_create_app(Callbacks callbacks) {
  CefRefPtr<MyApp> app = new MyApp(callbacks);

  return cef_interface_add_ref_app(app);
}

extern "C" int cef_interface_initialize(MyApp* app_ptr) {
  // Enable High-DPI support on Windows 7 or newer.
  CefEnableHighDPISupport();

  // Structure for passing command-line arguments.
  // The definition of this structure is platform-specific.
  CefMainArgs main_args;

  // Populate this structure to customize CEF behavior.
  CefSettings settings;
  // sandboxing needs you to "use the same executable for the browser process
  // and all sub-processes" so we disable it
  settings.no_sandbox = true;
  settings.windowless_rendering_enabled = true;

  // fixes cef firing winproc events that cc catches
  settings.external_message_pump = true;

  // We need to have the main thread process work
  // so that it can paint
  settings.multi_threaded_message_loop = false;

#if defined(WIN32) || defined(_WIN32) || \
    defined(__WIN32) && !defined(__CYGWIN__)
  const char* cef_simple_name = "cefsimple.exe";
#else
  const char* cef_simple_name = "cefsimple";
#endif

  // Specify the path for the sub-process executable.
  CefString(&settings.browser_subprocess_path).FromASCII(cef_simple_name);

  // Initialize CEF in the main process.
  if (!CefInitialize(main_args, settings, app_ptr, NULL)) {
    return -1;
  }
  return 0;
}

// Browser

extern "C" int cef_interface_create_browser(MyClient* client_ptr,
                                            const char* startup_url) {
  // Create the browser window.
  CefWindowInfo windowInfo;
  windowInfo.SetAsWindowless(0);

  const CefString& url = startup_url;
  CefBrowserSettings settings;
  settings.windowless_frame_rate = 30;

  CefRefPtr<CefDictionaryValue> extra_info = CefDictionaryValue::Create();
  extra_info->SetInt("bap", 23);

  bool browser = CefBrowserHost::CreateBrowser(windowInfo, client_ptr, url,
                                               settings, extra_info, NULL);

  if (!browser) {
    return -1;
  }

  return 0;
}

extern "C" int cef_interface_browser_get_identifier(CefBrowser* browser_ptr) {
  return browser_ptr->GetIdentifier();
}

extern "C" int cef_interface_browser_load_url(CefBrowser* browser_ptr,
                                              const char* url) {
  browser_ptr->GetMainFrame()->LoadURL(url);
  return 0;
}

extern "C" int cef_interface_browser_execute_javascript(CefBrowser* browser_ptr,
                                                        const char* code) {
  CefRefPtr<CefFrame> frame = browser_ptr->GetMainFrame();
  if (!frame) {
    return -1;
  }

  frame->ExecuteJavaScript(code, frame->GetURL(), 0);

  return 0;
}

extern "C" int cef_interface_browser_click(CefBrowser* browser_ptr,
                                           int x,
                                           int y) {
  auto browser_host = browser_ptr->GetHost();

  CefMouseEvent event = CefMouseEvent();
  event.x = x;
  event.y = y;

  browser_host->SendMouseClickEvent(
      event, CefBrowserHost::MouseButtonType::MBT_LEFT, false, 1);

  browser_host->SendMouseClickEvent(
      event, CefBrowserHost::MouseButtonType::MBT_LEFT, true, 1);

  return 0;
}

extern "C" int cef_interface_browser_close(CefBrowser* browser_ptr) {
  auto browser_host = browser_ptr->GetHost();

  // force_close: true because we don't want popups!
  browser_host->CloseBrowser(true);

  return 0;
}

extern "C" int cef_interface_step() {
  CefDoMessageLoopWork();
  return 0;
}

extern "C" int cef_interface_shutdown() {
  CefShutdown();
  return 0;
}

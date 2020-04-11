#include "interface.hh"
#include "app.hh"
#include "client.hh"

#include <chrono>    // std::chrono::seconds
#include <iostream>  // std::cout, std::endl
#include <thread>    // std::this_thread::sleep_for

extern "C" RustRefApp cef_interface_create_app(
    OnContextInitializedCallback on_context_initialized_callback,
    OnAfterCreatedCallback on_after_created_callback,
    OnBeforeCloseCallback on_before_close_callback,
    OnPaintCallback on_paint_callback) {
  CefRefPtr<MyApp> app =
      new MyApp(on_context_initialized_callback, on_after_created_callback,
                on_before_close_callback, on_paint_callback);

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

  // Specify the path for the sub-process executable.
  CefString(&settings.browser_subprocess_path).FromASCII("cefsimple.exe");

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
  windowInfo.SetAsWindowless(NULL);

  const CefString& url = startup_url;
  CefBrowserSettings settings;
  settings.windowless_frame_rate = 30;

  CefBrowserHost::CreateBrowser(windowInfo, client_ptr, url, settings, NULL,
                                NULL);

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

extern "C" int cef_interface_browser_close(CefBrowser* browser_ptr) {
  auto browser_host = browser_ptr->GetHost();

  // force_close: true because we don't want popups!
  browser_host->CloseBrowser(true);

  return 0;
}

// extern "C" int cef_free() {
//   // We must close browser (and wait for it to close) before calling
//   CefShutdown

//   // TODO move this logic into rust?

//   rust_print("CloseBrowser");
//   app->client->browser_->GetHost()->CloseBrowser(false);

//   while (app && app->client && app->client->browser_) {
//     rust_print("waiting");

//     CefDoMessageLoopWork();

//     std::this_thread::sleep_for(std::chrono::milliseconds(24));
//   }
//   // rust_print("wait: wait_for_browser_close");
//   // wait_for_browser_close();

//   // rust_print("wait: CefRunMessageLoop");
//   // CefRunMessageLoop();

//   rust_print("CefShutdown");
//   CefShutdown();

//   return 0;
// }

extern "C" int cef_interface_step() {
  CefDoMessageLoopWork();
  return 0;
}

extern "C" int cef_interface_shutdown() {
  CefShutdown();
  return 0;
}

RustRefApp cef_interface_add_ref_app(MyApp* ptr) {
  ptr->AddRef();

  RustRefApp r;
  r.ptr = ptr;
  return r;
}
int cef_interface_release_ref_app(MyApp* app_ptr) {
  app_ptr->Release();
  return 0;
}

RustRefClient cef_interface_add_ref_client(MyClient* ptr) {
  ptr->AddRef();

  RustRefClient r;
  r.ptr = ptr;
  return r;
}
int cef_interface_release_ref_client(MyClient* client_ptr) {
  client_ptr->Release();
  return 0;
}

RustRefBrowser cef_interface_add_ref_browser(CefBrowser* ptr) {
  ptr->AddRef();

  RustRefBrowser r;
  r.ptr = ptr;
  return r;
}
int cef_interface_release_ref_browser(CefBrowser* browser_ptr) {
  browser_ptr->Release();
  return 0;
}

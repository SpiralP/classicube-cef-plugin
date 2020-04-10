#include "interface.hh"

#include <chrono>    // std::chrono::seconds
#include <iostream>  // std::cout, std::endl
#include <thread>    // std::this_thread::sleep_for

#include "app.cc"

CefRefPtr<MyApp> app;

extern "C" int cef_init(OnPaintCallback onPaintCallback) {
  // Enable High-DPI support on Windows 7 or newer.
  CefEnableHighDPISupport();

  // Structure for passing command-line arguments.
  // The definition of this structure is platform-specific.
  CefMainArgs main_args;

  // Populate this structure to customize CEF behavior.
  CefSettings settings;
  settings.no_sandbox = true;
  settings.windowless_rendering_enabled = true;

  // fixes cef firing winproc events that cc catches
  settings.external_message_pump = true;

  // We need to have the main thread process work
  // so that it can paint
  settings.multi_threaded_message_loop = false;

  // Specify the path for the sub-process executable.
  CefString(&settings.browser_subprocess_path).FromASCII("cefsimple.exe");

  app = new MyApp(onPaintCallback);

  // Initialize CEF in the main process.
  if (!CefInitialize(main_args, settings, app.get(), NULL)) {
    return -1;
  }

  return 0;
}

extern "C" int cef_free() {
  // We must close browser (and wait for it to close) before calling CefShutdown

  // TODO move this logic into rust?

  rust_print("CloseBrowser");
  app->client->browser_->GetHost()->CloseBrowser(false);

  while (app && app->client && app->client->browser_) {
    rust_print("waiting");

    CefDoMessageLoopWork();

    std::this_thread::sleep_for(std::chrono::milliseconds(24));
  }
  // rust_print("wait: wait_for_browser_close");
  // wait_for_browser_close();

  // rust_print("wait: CefRunMessageLoop");
  // CefRunMessageLoop();

  rust_print("CefShutdown");
  CefShutdown();

  return 0;
}

extern "C" int cef_step() {
  CefDoMessageLoopWork();

  return 0;
}

extern "C" int cef_load(const char* url) {
  app->client->browser_->GetMainFrame()->LoadURL(url);

  return 0;
}

extern "C" int cef_run_script(const char* code) {
  auto frame = app->client->browser_->GetMainFrame();

  frame->ExecuteJavaScript(code, frame->GetURL(), 0);
  return 0;
}

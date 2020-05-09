#include "interface.hh"

#include <include/base/cef_bind.h>
#include <include/cef_origin_whitelist.h>
#include <include/wrapper/cef_closure_task.h>
#if defined(OS_MACOSX)
#include <include/wrapper/cef_library_loader.h>
#endif

#include <chrono>    // std::chrono::seconds
#include <iostream>  // std::cout, std::endl
#include <thread>    // std::this_thread::sleep_for

#include "app.hh"
#include "client.hh"

extern "C" RustRefApp cef_interface_add_ref_app(MyApp* app) {
  app->AddRef();

  RustRefApp r;
  r.ptr = app;
  return r;
}
extern "C" int cef_interface_release_ref_app(MyApp* app) {
  app->Release();
  return 0;
}

extern "C" RustRefClient cef_interface_add_ref_client(MyClient* client) {
  client->AddRef();

  RustRefClient r;
  r.ptr = client;
  return r;
}
extern "C" int cef_interface_release_ref_client(MyClient* client) {
  client->Release();
  return 0;
}

extern "C" RustRefBrowser cef_interface_add_ref_browser(CefBrowser* browser) {
  browser->AddRef();

  RustRefBrowser r;
  r.ptr = browser;
  return r;
}
extern "C" int cef_interface_release_ref_browser(CefBrowser* browser) {
  browser->Release();
  return 0;
}

extern "C" RustRefString cef_interface_new_ref_string(const char* c_str,
                                                      size_t len) {
  char* copy = new char[len + 1]();
  strcpy(copy, c_str);

  RustRefString r;
  r.ptr = copy;
  r.len = len;
  return r;
}
extern "C" int cef_interface_delete_ref_string(const char* c_str) {
  delete[] c_str;
  return 0;
}

extern "C" RustRefApp cef_interface_create_app(Callbacks callbacks) {
  CefRefPtr<MyApp> app = new MyApp(callbacks);

  return cef_interface_add_ref_app(app);
}

extern "C" int cef_interface_initialize(MyApp* app) {
#if defined(OS_MACOSX)
  if (!cef_load_library("./cef/Chromium Embedded Framework.framework/Chromium "
                        "Embedded Framework")) {
    return 1;
  }
#endif

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

  settings.background_color = 0xFFFFFFFF;

  CefString(&settings.log_file).FromASCII("cef-binary.log");

#if defined(_WIN64) || defined(_WIN32)
  const char* cef_exe_path = "cef.exe";
#elif defined(OS_MACOSX)
  const char* cef_exe_path = "./cef/cef.app/Contents/MacOS/cef";

  CefString(&settings.main_bundle_path).FromASCII("./cef");

  // this needs full path or crashes!
  char cwd[PATH_MAX + 1];
  if (!getcwd(cwd, sizeof(cwd))) {
    return -1;
  }
  std::string full_path(cwd);
  full_path += "/cef/Chromium Embedded Framework.framework";
  CefString(&settings.framework_dir_path).FromASCII(full_path.c_str());
#else
  const char* cef_exe_path = "cef";

  // linux had trouble finding locales
  CefString(&settings.locales_dir_path).FromASCII("./cef/cef_binary/locales");
#endif

  // Specify the path for the sub-process executable.
  CefString(&settings.browser_subprocess_path).FromASCII(cef_exe_path);

  // Initialize CEF in the main process.
  if (!CefInitialize(main_args, settings, app, NULL)) {
    return -1;
  }
  return 0;
}

// Browser

extern "C" int cef_interface_create_browser(MyClient* client,
                                            const char* startup_url,
                                            int frame_rate) {
  // Create the browser window.
  CefWindowInfo windowInfo;
  windowInfo.SetAsWindowless(nullptr);

  const CefString& url = startup_url;
  CefBrowserSettings settings;

  settings.windowless_frame_rate = frame_rate;

  bool browser = CefBrowserHost::CreateBrowser(windowInfo, client, url,
                                               settings, nullptr, nullptr);

  if (!browser) {
    return -1;
  }

  return 0;
}

extern "C" int cef_interface_browser_get_identifier(CefBrowser* browser) {
  return browser->GetIdentifier();
}

extern "C" int cef_interface_browser_load_url(CefBrowser* browser,
                                              const char* url) {
  auto frame = browser->GetMainFrame();
  frame->LoadURL(url);

  return 0;
}

extern "C" int cef_interface_browser_execute_javascript(CefBrowser* browser,
                                                        const char* code) {
  CefRefPtr<CefFrame> frame = browser->GetMainFrame();
  if (!frame) {
    return -1;
  }

  frame->ExecuteJavaScript(code, frame->GetURL(), 0);

  return 0;
}

extern "C" int cef_interface_browser_eval_javascript(CefBrowser* browser,
                                                     uint64_t task_id,
                                                     const char* c_code) {
  auto frame = browser->GetMainFrame();

  CefString script(c_code);
  CefString script_url(frame->GetURL());
  int start_line = 0;

  auto message = CefProcessMessage::Create("EvalJavascript");
  CefRefPtr<CefListValue> args = message->GetArgumentList();
  args->SetBinary(0, CefBinaryValue::Create(&task_id, sizeof(uint64_t)));
  args->SetString(1, script);
  args->SetString(2, script_url);
  args->SetInt(3, start_line);

  frame->SendProcessMessage(PID_RENDERER, message);

  return 0;
}

extern "C" int cef_interface_browser_send_click(CefBrowser* browser,
                                                int x,
                                                int y) {
  auto browser_host = browser->GetHost();

  CefMouseEvent event = CefMouseEvent();
  event.x = x;
  event.y = y;

  browser_host->SendMouseClickEvent(
      event, CefBrowserHost::MouseButtonType::MBT_LEFT, false, 1);

  browser_host->SendMouseClickEvent(
      event, CefBrowserHost::MouseButtonType::MBT_LEFT, true, 1);

  return 0;
}

extern "C" int cef_interface_browser_send_text(CefBrowser* browser,
                                               const char* text) {
  auto browser_host = browser->GetHost();

  for (const char* c = text; *c; ++c) {
    CefKeyEvent event = CefKeyEvent();
    event.type = KEYEVENT_CHAR;
    event.character = *c;
    event.unmodified_character = *c;
    event.windows_key_code = *c;
    event.native_key_code = *c;

    browser_host->SendKeyEvent(event);
  }

  return 0;
}

extern "C" int cef_interface_browser_reload(CefBrowser* browser) {
  browser->Reload();
  return 0;
}

extern "C" int cef_interface_browser_was_resized(CefBrowser* browser) {
  browser->GetHost()->WasResized();
  return 0;
}

extern "C" int cef_interface_browser_close(CefBrowser* browser) {
  auto browser_host = browser->GetHost();

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

#if defined(OS_MACOSX)
  cef_unload_library();
#endif

  return 0;
}

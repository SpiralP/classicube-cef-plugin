#include "interface.hh"

#include <include/base/cef_bind.h>
#include <include/cef_origin_whitelist.h>
#include <include/cef_request_context.h>
#include <include/cef_request_context_handler.h>
#include <include/wrapper/cef_closure_task.h>
#include <include/wrapper/cef_stream_resource_handler.h>
#if defined(OS_MACOSX)
#include <include/wrapper/cef_library_loader.h>
#endif

#ifdef _WIN32
#include <direct.h>  // getcwd
#define getcwd _getcwd
#else
#include <unistd.h>  // getcwd
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

// functions to rust

extern "C" int cef_interface_execute_process(int argc,
                                             const char* const argv[]) {
  rust_debug("cef_interface_execute_process");

#if defined(OS_MACOSX)
  if (!cef_load_library("./cef/Chromium Embedded Framework.framework/Chromium "
                        "Embedded Framework")) {
    rust_warn("cef_interface_execute_process cef_load_library");
    return 1;
  }
#endif

#if defined(_WIN64) || defined(_WIN32)
  CefMainArgs main_args(GetModuleHandle(NULL));
#else
  CefMainArgs main_args(argc, const_cast<char**>(argv));
#endif

  CefRefPtr<CefApp> app(new MyApp({}));

  // CEF applications have multiple sub-processes (render, plugin, GPU, etc)
  // that share the same executable. This function checks the command-line and,
  // if this is a sub-process, executes the appropriate logic.
  int exit_code = CefExecuteProcess(main_args, app, nullptr);

#if defined(OS_MACOSX)
  if (!cef_unload_library()) {
    rust_warn("cef_interface_execute_process cef_unload_library");
    return 1;
  }
#endif

  return exit_code;
}

extern "C" RustRefApp cef_interface_create_app(Callbacks callbacks) {
  CefRefPtr<MyApp> app = new MyApp(callbacks);

  return cef_interface_add_ref_app(app.get());
}

// Implementation of the factory for for creating schema handlers.
class LocalSchemeHandlerFactory : public CefSchemeHandlerFactory {
 public:
  LocalSchemeHandlerFactory() {}

  // Return a new scheme handler instance to handle the request.
  CefRefPtr<CefResourceHandler> Create(CefRefPtr<CefBrowser> browser,
                                       CefRefPtr<CefFrame> frame,
                                       const CefString& scheme_name,
                                       CefRefPtr<CefRequest> request) override {
    CEF_REQUIRE_IO_THREAD();

    std::string scheme_name_utf8 = scheme_name.ToString();
    std::string url_utf8 = request->GetURL().ToString();
    auto ret =
        rust_handle_scheme_create(cef_interface_add_ref_browser(browser.get()),
                                  scheme_name_utf8.c_str(), url_utf8.c_str());

    if (!ret.mime_type) {
      // an empty reference to allow default handling of the request
      return nullptr;
    } else {
      return new CefStreamResourceHandler(
          ret.mime_type,
          CefStreamReader::CreateForData(ret.data, ret.data_size));
    }
  }

  IMPLEMENT_REFCOUNTING(LocalSchemeHandlerFactory);
  DISALLOW_COPY_AND_ASSIGN(LocalSchemeHandlerFactory);
};

extern "C" int cef_interface_initialize(MyApp* app, CefInitializePaths paths) {
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

  CefString(&settings.browser_subprocess_path)
      .FromString(paths.browser_subprocess_path);

  CefString(&settings.root_cache_path).FromString(paths.root_cache_path);

  CefString(&settings.main_bundle_path).FromString(paths.main_bundle_path);
  CefString(&settings.framework_dir_path).FromString(paths.framework_dir_path);

  CefString(&settings.resources_dir_path).FromString(paths.resources_dir_path);
  CefString(&settings.locales_dir_path).FromString(paths.locales_dir_path);

  // Initialize CEF in the main process.
  if (!CefInitialize(main_args, settings, app, NULL)) {
    rust_warn("CefInitialize failed!");
    return -1;
  }

  if (!CefRegisterSchemeHandlerFactory("local", "",
                                       new LocalSchemeHandlerFactory())) {
    rust_warn("CefRegisterSchemeHandlerFactory failed!");
    return -1;
  }

  // if (!CefAddCrossOriginWhitelistEntry("local://media", "http", "", true)) {
  //   rust_warn("CefAddCrossOriginWhitelistEntry failed!");
  //   return -1;
  // }

  return 0;
}

// Browser

extern "C" int cef_interface_create_browser(MyClient* client,
                                            const char* startup_url,
                                            int frame_rate,
                                            bool insecure,
                                            uint32_t background_color) {
  // Create the browser window.
  CefWindowInfo window_info;
  window_info.SetAsWindowless(0);

  const CefString& url = startup_url;
  CefBrowserSettings settings;
  settings.background_color = background_color;
  settings.windowless_frame_rate = frame_rate;

  settings.tab_to_links = STATE_DISABLED;
  settings.javascript_close_windows = STATE_DISABLED;
  settings.javascript_dom_paste = STATE_DISABLED;
  settings.javascript_access_clipboard = STATE_DISABLED;

  CefRefPtr<CefDictionaryValue> extra_info = nullptr;
  CefRefPtr<CefRequestContext> request_context = nullptr;

  // if (insecure) {
  // this is pretty useless because it needs to be on the subprocess's
  // OnBeforeCommandLineProcessing
  // settings.web_security = STATE_DISABLED;

  // CefRequestContextSettings request_context_settings;
  // request_context_settings.ignore_certificate_errors = true;

  // request_context =
  //     CefRequestContext::CreateContext(request_context_settings, nullptr);

  // if (!request_context->RegisterSchemeHandlerFactory(
  //         "test", "", new TestSchemeFactory())) {
  //   rust_warn("CefRegisterSchemeHandlerFactory");
  // }
  // }

  bool browser = CefBrowserHost::CreateBrowser(
      window_info, client, url, settings, extra_info, request_context);

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

extern "C" int cef_interface_browser_execute_javascript_on_frame(
    CefBrowser* browser,
    const char* frame_name,
    const char* code) {
#if CEF_VERSION_MAJOR >= 122
  std::vector<CefString> ids;
#else
  std::vector<int64_t> ids;
#endif
  browser->GetFrameIdentifiers(ids);

  for (auto id : ids) {
#if CEF_VERSION_MAJOR >= 122
    auto frame = browser->GetFrameByIdentifier(id);
#else
    auto frame = browser->GetFrame(id);
#endif
    if (frame) {
      std::string url = frame->GetURL().ToString();
      if (url.rfind(frame_name, 0) == 0) {
        frame->ExecuteJavaScript(code, frame->GetURL(), 0);
        return 0;
      }
    }
  }

  return -1;
}

extern "C" int cef_interface_browser_eval_javascript(CefBrowser* browser,
                                                     uint64_t task_id,
                                                     const char* code) {
  auto frame = browser->GetMainFrame();

  CefString script(code);
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

extern "C" int cef_interface_browser_eval_javascript_on_frame(
    CefBrowser* browser,
    const char* frame_name,
    uint64_t task_id,
    const char* code) {
#if CEF_VERSION_MAJOR >= 122
  std::vector<CefString> ids;
#else
  std::vector<int64_t> ids;
#endif
  browser->GetFrameIdentifiers(ids);

  for (auto id : ids) {
#if CEF_VERSION_MAJOR >= 122
    auto frame = browser->GetFrameByIdentifier(id);
#else
    auto frame = browser->GetFrame(id);
#endif
    if (frame) {
      std::string url = frame->GetURL().ToString();
      if (url.rfind(frame_name, 0) == 0) {
        CefString script(code);
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
    }
  }

  return -1;
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
  browser->ReloadIgnoreCache();
  return 0;
}

extern "C" int cef_interface_browser_was_resized(CefBrowser* browser) {
  browser->GetHost()->WasResized();
  return 0;
}

extern "C" int cef_interface_browser_open_dev_tools(CefBrowser* browser) {
  auto browser_host = browser->GetHost();

  CefWindowInfo window_info;
#if defined(_WIN64) || defined(_WIN32)
  window_info.SetAsPopup(0, "devtools");
#endif

  CefBrowserSettings settings;
  CefPoint inspect_element_at;
  browser_host->ShowDevTools(window_info, nullptr, settings,
                             inspect_element_at);

  return 0;
}

extern "C" int cef_interface_browser_set_audio_muted(CefBrowser* browser,
                                                     bool mute) {
  auto browser_host = browser->GetHost();
  browser_host->SetAudioMuted(mute);
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

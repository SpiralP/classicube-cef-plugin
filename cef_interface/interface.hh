#pragma once

#include <cstddef>
#include <cstdint>

// TODO use a namespace for cef_interface_ prefix!

class MyApp;
class MyClient;
class CefBrowser;
class CefV8Value;

struct RustRefApp {
  MyApp* ptr;
};

extern "C" RustRefApp cef_interface_add_ref_app(MyApp* app);
extern "C" int cef_interface_release_ref_app(MyApp* app);

struct RustRefClient {
  MyClient* ptr;
};

extern "C" RustRefClient cef_interface_add_ref_client(MyClient* client);
extern "C" int cef_interface_release_ref_client(MyClient* client);

struct RustRefBrowser {
  CefBrowser* ptr;
};

extern "C" RustRefBrowser cef_interface_add_ref_browser(CefBrowser* browser);
extern "C" int cef_interface_release_ref_browser(CefBrowser* browser);

struct RustRefString {
  const char* ptr;
  size_t len;
};

/// must call cef_interface_delete_ref_string
extern "C" RustRefString cef_interface_new_ref_string(const char* c_str,
                                                      size_t len);
extern "C" int cef_interface_delete_ref_string(const char* c_str);

/// Called on the browser process UI thread immediately after the CEF context
/// has been initialized.
typedef void (*OnContextInitializedCallback)(RustRefClient client);

/// Called after a new browser is created.
typedef void (*OnAfterCreatedCallback)(RustRefBrowser browser);

/// Called just before a browser is destroyed.
typedef void (*OnBeforeCloseCallback)(RustRefBrowser browser);

typedef void (*OnPaintCallback)(RustRefBrowser browser,
                                const void* pixels,
                                int width,
                                int height);

/// Called when the browser is done loading the MAIN frame.
typedef void (*OnLoadEndCallback)(RustRefBrowser browser);

/// Called when the page title changes.
typedef void (*OnTitleChangeCallback)(RustRefBrowser browser,
                                      const char* title);

struct RustRect {
  int x;
  int y;
  int width;
  int height;
};

typedef RustRect (*GetViewRectCallback)(RustRefBrowser browser);

enum FFIRustV8ValueTag : uint8_t {
  Unknown,
  Array,
  ArrayBuffer,
  Bool,
  Date,
  Double,
  Function,
  Int,
  Null,
  Object,
  String,
  UInt,
  Undefined,
};

struct FFIRustV8Value {
  FFIRustV8ValueTag tag;
  union {
    bool bool_;
    double double_;
    int32_t int_;
    RustRefString string;
    uint32_t uint;
  };
};

struct FFIRustV8Response {
  bool success;
  union {
    FFIRustV8Value result;
    bool error;
  };
};

typedef void (*OnJavascriptCallback)(RustRefBrowser browser,
                                     uint64_t id,
                                     FFIRustV8Response v8_response);

typedef bool (*OnCertificateErrorCallback)(RustRefBrowser browser);

struct Callbacks {
  OnContextInitializedCallback on_context_initialized;
  OnAfterCreatedCallback on_after_created;
  OnBeforeCloseCallback on_before_close;
  OnPaintCallback on_paint;
  OnLoadEndCallback on_load_end;
  OnTitleChangeCallback on_title_change;
  GetViewRectCallback get_view_rect;
  OnJavascriptCallback on_javascript;
  OnCertificateErrorCallback on_certificate_error;
};

struct CefInitializePaths {
  const char* browser_subprocess_path;
  const char* root_cache_path;

  const char* resources_dir_path;
  const char* locales_dir_path;

  const char* main_bundle_path;
  const char* framework_dir_path;
};

// functions to rust

extern "C" RustRefApp cef_interface_create_app(Callbacks callbacks);

extern "C" int cef_interface_shutdown();
extern "C" int cef_interface_step();

extern "C" int cef_interface_initialize(MyApp* app, CefInitializePaths paths);

// Browser

extern "C" int cef_interface_create_browser(MyClient* client,
                                            const char* startup_url,
                                            int frame_rate,
                                            bool insecure,
                                            uint32_t background_color);
extern "C" int cef_interface_browser_get_identifier(CefBrowser* browser);
extern "C" int cef_interface_browser_load_url(CefBrowser* browser,
                                              const char* url);
extern "C" int cef_interface_browser_execute_javascript(CefBrowser* browser,
                                                        const char* code);
extern "C" int cef_interface_browser_execute_javascript_on_frame(
    CefBrowser* browser,
    const char* frame_name,
    const char* code);
extern "C" int cef_interface_browser_eval_javascript(CefBrowser* browser,
                                                     uint64_t task_id,
                                                     const char* code);
extern "C" int cef_interface_browser_eval_javascript_on_frame(
    CefBrowser* browser,
    const char* frame_name,
    uint64_t task_id,
    const char* code);
extern "C" int cef_interface_browser_send_click(CefBrowser* browser,
                                                int x,
                                                int y);
extern "C" int cef_interface_browser_send_text(CefBrowser* browser,
                                               const char* text);
extern "C" int cef_interface_browser_reload(CefBrowser* browser);

extern "C" int cef_interface_browser_was_resized(CefBrowser* browser);
extern "C" int cef_interface_browser_open_dev_tools(CefBrowser* browser);
extern "C" int cef_interface_browser_set_audio_muted(CefBrowser* browser,
                                                     bool mute);

/// Tell browser to close, OnBeforeClose will be called soon
extern "C" int cef_interface_browser_close(CefBrowser* browser);

// functions from rust

extern "C" void rust_debug(const char* c_str);
extern "C" void rust_warn(const char* c_str);

struct RustSchemeReturn {
  /// must be a static data
  void* data;
  size_t data_size;

  const char* mime_type;
};
extern "C" RustSchemeReturn rust_handle_scheme_create(RustRefBrowser browser,
                                                      const char* scheme_name,
                                                      const char* url);

#pragma once

// TODO use a namespace for cef_interface_ prefix!

class MyApp;
class MyClient;
class CefBrowser;

struct RustRefApp {
  MyApp* ptr;
};

extern "C" RustRefApp cef_interface_add_ref_app(MyApp* app_ptr);
extern "C" int cef_interface_release_ref_app(MyApp* app_ptr);

struct RustRefClient {
  MyClient* ptr;
};

extern "C" RustRefClient cef_interface_add_ref_client(MyClient* client_ptr);
extern "C" int cef_interface_release_ref_client(MyClient* client_ptr);

struct RustRefBrowser {
  CefBrowser* ptr;
};

extern "C" RustRefBrowser cef_interface_add_ref_browser(
    CefBrowser* browser_ptr);
extern "C" int cef_interface_release_ref_browser(CefBrowser* browser_ptr);

/// Called on the browser process UI thread immediately after the CEF context
/// has been initialized.
typedef void (*OnContextInitializedCallback)(RustRefClient client);

/// Called just before a browser is destroyed.
typedef void (*OnBeforeCloseCallback)(RustRefBrowser browser);

typedef void (*OnPaintCallback)(RustRefBrowser browser,
                                const void* pixels,
                                int width,
                                int height);

/// Called when the browser is done loading the MAIN frame.
typedef void (*OnLoadEndCallback)(RustRefBrowser browser);

// functions to rust

extern "C" RustRefApp cef_interface_create_app(
    OnContextInitializedCallback on_context_initialized_callback,
    OnBeforeCloseCallback on_before_close_callback,
    OnPaintCallback on_paint_callback,
    OnLoadEndCallback on_load_end_callback);

extern "C" int cef_interface_shutdown();
extern "C" int cef_interface_step();

extern "C" int cef_interface_initialize(MyApp* app_ptr);

// Browser

extern "C" RustRefBrowser cef_interface_create_browser(MyClient* client_ptr,
                                                       const char* startup_url);
extern "C" int cef_interface_browser_get_identifier(CefBrowser* browser_ptr);
extern "C" int cef_interface_browser_load_url(CefBrowser* browser_ptr,
                                              const char* url);
extern "C" int cef_interface_browser_execute_javascript(CefBrowser* browser_ptr,
                                                        const char* code);

/// Tell browser to close, OnBeforeClose will be called soon?
extern "C" int cef_interface_browser_close(CefBrowser* browser_ptr);

// functions from rust
extern "C" void rust_print(const char* c_str);

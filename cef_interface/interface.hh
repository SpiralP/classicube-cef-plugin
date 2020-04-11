#pragma once

// TODO use a namespace for cef_interface_ prefix!

class MyApp;
class MyClient;
class CefBrowser;

struct RustRefApp {
  MyApp* ptr;
};

RustRefApp create_rust_ref_app(MyApp* app_ptr);
int cef_interface_release_rust_ref_app(MyApp* app_ptr);

struct RustRefClient {
  MyClient* ptr;
};

RustRefClient create_rust_ref_client(MyClient* client_ptr);
int cef_interface_release_rust_ref_client(MyClient* client_ptr);

struct RustRefBrowser {
  CefBrowser* ptr;
};

RustRefBrowser create_rust_ref_browser(CefBrowser* browser_ptr);
int cef_interface_release_rust_ref_browser(CefBrowser* browser_ptr);

// Called on the browser process UI thread immediately after the CEF context has
// been initialized.
typedef void (*OnContextInitializedCallback)(RustRefClient client);

// Called after a new browser is created. This callback will be the first
// notification that references |browser|.
typedef void (*OnAfterCreatedCallback)(RustRefBrowser browser);

// Called just before a browser is destroyed.
typedef void (*OnBeforeCloseCallback)(RustRefBrowser browser);

typedef void (*OnPaintCallback)(RustRefBrowser browser,
                                const void* pixels,
                                int width,
                                int height);

// functions to rust

extern "C" RustRefApp cef_interface_create_app(
    OnContextInitializedCallback on_context_initialized_callback,
    OnAfterCreatedCallback on_after_created_callback,
    OnBeforeCloseCallback on_before_close_callback,
    OnPaintCallback on_paint_callback);

extern "C" int cef_interface_initialize(MyApp* app_ptr);

extern "C" int cef_interface_create_browser(MyClient* client_ptr,
                                            const char* startup_url);
extern "C" int cef_interface_browser_load_url(CefBrowser* browser_ptr,
                                              const char* url);

extern "C" int cef_interface_step();

// extern "C" int cef_interface_free();
// extern "C" int cef_interface_load(const char* url);
// extern "C" int cef_interface_run_script(const char* code);

// functions from rust
extern "C" void rust_print(const char* c_str);

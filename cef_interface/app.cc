#include "app.hh"

// Minimal implementation of CefApp for the browser process.

MyApp::MyApp(OnContextInitializedCallback on_context_initialized_callback,
             OnBeforeCloseCallback on_before_close_callback,
             OnPaintCallback on_paint_callback) {
  this->on_context_initialized_callback = on_context_initialized_callback;

  this->client = new MyClient(on_before_close_callback, on_paint_callback);
}

// CefApp methods:
CefRefPtr<CefBrowserProcessHandler> MyApp::GetBrowserProcessHandler() {
  return this;
}

void MyApp::OnBeforeCommandLineProcessing(
    const CefString& process_type,
    CefRefPtr<CefCommandLine> command_line) {
  // Command-line flags can be modified in this callback.
  // |process_type| is empty for the browser process.
  command_line->AppendSwitchWithValue("autoplay-policy",
                                      "no-user-gesture-required");
  command_line->AppendSwitch("disable-extensions");
}

// CefBrowserProcessHandler methods:
void MyApp::OnContextInitialized() {
  on_context_initialized_callback(
      cef_interface_add_ref_client(this->client.get()));
}

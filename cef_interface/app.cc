#include "app.hh"

// Minimal implementation of CefApp for the browser process.

MyApp::MyApp(Callbacks callbacks) {
  this->on_context_initialized_callback =
      callbacks.on_context_initialized_callback;

  this->client = new MyClient(callbacks);
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
  if (on_context_initialized_callback) {
    on_context_initialized_callback(
        cef_interface_add_ref_client(this->client.get()));
  }
}

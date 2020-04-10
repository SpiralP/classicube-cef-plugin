
#include <include/cef_app.h>
#include "client.cc"

const char kStartupURL[] = "https://www.classicube.net/";

// Minimal implementation of CefApp for the browser process.
class MyApp : public CefApp, public CefBrowserProcessHandler {
 public:
  MyApp(OnPaintCallback onPaintCallback) {
    this->client = new MyClient(onPaintCallback);
  }

  CefRefPtr<MyClient> client;

  // CefApp methods:
  CefRefPtr<CefBrowserProcessHandler> GetBrowserProcessHandler() OVERRIDE {
    return this;
  }

  void OnBeforeCommandLineProcessing(
      const CefString& process_type,
      CefRefPtr<CefCommandLine> command_line) OVERRIDE {
    // Command-line flags can be modified in this callback.
    // |process_type| is empty for the browser process.
    command_line->AppendSwitchWithValue("autoplay-policy",
                                        "no-user-gesture-required");
    command_line->AppendSwitch("disable-extensions");
  }

  // CefBrowserProcessHandler methods:
  void OnContextInitialized() OVERRIDE {
    // Create the browser window.

    CefWindowInfo windowInfo;
    windowInfo.SetAsWindowless(NULL);

    const CefString& url = kStartupURL;
    CefBrowserSettings settings;
    settings.windowless_frame_rate = 30;

    CefBrowserHost::CreateBrowser(windowInfo, client.get(), url, settings, NULL,
                                  NULL);
  }

 private:
  IMPLEMENT_REFCOUNTING(MyApp);
  DISALLOW_COPY_AND_ASSIGN(MyApp);
};

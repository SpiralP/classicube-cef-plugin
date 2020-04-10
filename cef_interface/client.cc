
#include <include/cef_client.h>
#include <include/wrapper/cef_helpers.h>
#include "render_handle.cc"

class MyClient : public CefClient,
                 public CefDisplayHandler,
                 public CefLifeSpanHandler {
 public:
  MyClient(OnPaintCallback onPaintCallback) {
    this->renderHandler = new MyRenderHandler(onPaintCallback);
  }

  CefRefPtr<CefBrowser> browser_;
  CefRefPtr<CefRenderHandler> renderHandler;

  // CefClient methods:
  CefRefPtr<CefDisplayHandler> GetDisplayHandler() OVERRIDE { return this; }
  CefRefPtr<CefLifeSpanHandler> GetLifeSpanHandler() OVERRIDE { return this; }
  CefRefPtr<CefRenderHandler> GetRenderHandler() OVERRIDE {
    return renderHandler;
  }

  void OnTitleChange(CefRefPtr<CefBrowser> browser,
                     const CefString& title) OVERRIDE {
    // wprintf(L"OnTitleChange %s", title.c_str());
    rust_print("OnTitleChange");
  }

  void OnAfterCreated(CefRefPtr<CefBrowser> browser) OVERRIDE {
    rust_print("OnAfterCreated");

    CEF_REQUIRE_UI_THREAD();
    DCHECK(!browser_);
    browser_ = browser;
  }

  bool DoClose(CefRefPtr<CefBrowser> browser) OVERRIDE {
    rust_print("DoClose");

    // Must be executed on the UI thread.
    CEF_REQUIRE_UI_THREAD();

    // force close?
    // Allow the close. For windowed browsers this will result in the OS close
    // event being sent.
    return false;
  }

  void OnBeforeClose(CefRefPtr<CefBrowser> browser) OVERRIDE {
    rust_print("OnBeforeClose");

    CEF_REQUIRE_UI_THREAD();

    browser_ = nullptr;

    // rust_print("wake_browser_closed");
    // wake_browser_closed();
    // CefQuitMessageLoop();
  }

 private:
  IMPLEMENT_REFCOUNTING(MyClient);
  DISALLOW_COPY_AND_ASSIGN(MyClient);
};

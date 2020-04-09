#include "interface.hh"

#include <include/cef_app.h>
#include <include/cef_base.h>
#include <include/cef_browser.h>
#include <include/cef_command_line.h>
#include <include/cef_render_handler.h>
#include <include/views/cef_browser_view.h>
#include <include/views/cef_window.h>
#include <include/wrapper/cef_helpers.h>

#include <condition_variable>  // std::condition_variable
#include <mutex>               // std::mutex, std::unique_lock
#include <thread>

// std::mutex mtx;
// std::condition_variable cv;

// void wait_for_browser_close() {
//   using namespace std::chrono_literals;

//   std::unique_lock<std::mutex> lck(mtx);
//   cv.wait(lck);

//   std::this_thread::sleep_for(100ms);
// }

// void wake_browser_closed() {
//   std::unique_lock<std::mutex> lck(mtx);
//   cv.notify_one();
// }

class MyRenderHandler : public CefRenderHandler {
 public:
  MyRenderHandler(OnPaintCallback onPaintCallback) {
    this->onPaintCallback = onPaintCallback;
  }

  OnPaintCallback onPaintCallback;

  CefRefPtr<CefAccessibilityHandler> GetAccessibilityHandler() OVERRIDE {
    return nullptr;
  }

  bool GetRootScreenRect(CefRefPtr<CefBrowser> browser,
                         CefRect& rect) OVERRIDE {
    return false;
  }

  bool GetScreenInfo(CefRefPtr<CefBrowser> browser,
                     CefScreenInfo& screen_info) OVERRIDE {
    return false;
  }

  bool GetScreenPoint(CefRefPtr<CefBrowser> browser,
                      int viewX,
                      int viewY,
                      int& screenX,
                      int& screenY) OVERRIDE {
    return false;
  }

  void GetViewRect(CefRefPtr<CefBrowser> browser, CefRect& rect) OVERRIDE {
    printf("GetViewRect\n");
    rect.x = 0;
    rect.y = 0;
    rect.width = 1280;
    rect.height = 720;
  }

  void OnAcceleratedPaint(CefRefPtr<CefBrowser> browser,
                          CefRenderHandler::PaintElementType type,
                          const CefRenderHandler::RectList& dirtyRects,
                          void* shared_handle) OVERRIDE {
    printf("OnAcceleratedPaint\n");
  }

  void OnCursorChange(CefRefPtr<CefBrowser> browser,
                      CefCursorHandle cursor,
                      CefRenderHandler::CursorType type,
                      const CefCursorInfo& custom_cursor_info) OVERRIDE {
    //
  }

  void OnImeCompositionRangeChanged(
      CefRefPtr<CefBrowser> browser,
      const CefRange& selected_range,
      const CefRenderHandler::RectList& character_bounds) OVERRIDE {
    //
  }

  void OnPaint(CefRefPtr<CefBrowser> browser,
               CefRenderHandler::PaintElementType type,
               const CefRenderHandler::RectList& dirtyRects,
               const void* pixels,
               int width,
               int height) OVERRIDE {
    printf("OnPaint %d %d\n", width, height);

    onPaintCallback(pixels, width, height);
  }

  void OnPopupShow(CefRefPtr<CefBrowser> browser, bool show) OVERRIDE {
    //
  }

  void OnPopupSize(CefRefPtr<CefBrowser> browser,
                   const CefRect& rect) OVERRIDE {
    //
  }

  void OnScrollOffsetChanged(CefRefPtr<CefBrowser> browser,
                             double x,
                             double y) OVERRIDE {
    //
  }

  void OnTextSelectionChanged(CefRefPtr<CefBrowser> browser,
                              const CefString& selected_text,
                              const CefRange& selected_range) OVERRIDE {
    //
  }

  void OnVirtualKeyboardRequested(
      CefRefPtr<CefBrowser> browser,
      CefRenderHandler::TextInputMode input_mode) OVERRIDE {
    //
  }

  bool StartDragging(CefRefPtr<CefBrowser> browser,
                     CefRefPtr<CefDragData> drag_data,
                     CefRenderHandler::DragOperationsMask allowed_ops,
                     int x,
                     int y) OVERRIDE {
    return false;
  }

  void UpdateDragCursor(CefRefPtr<CefBrowser> browser,
                        CefRenderHandler::DragOperation operation) OVERRIDE {
    //
  }

 private:
  IMPLEMENT_REFCOUNTING(MyRenderHandler);
};

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
    wprintf(L"OnTitleChange %s\n", title.c_str());
  }

  void OnAfterCreated(CefRefPtr<CefBrowser> browser) OVERRIDE {
    printf("OnAfterCreated\n");

    CEF_REQUIRE_UI_THREAD();
    DCHECK(!browser_);
    browser_ = browser;
  }

  bool DoClose(CefRefPtr<CefBrowser> browser) OVERRIDE {
    printf("DoClose\n");

    // Must be executed on the UI thread.
    CEF_REQUIRE_UI_THREAD();

    // force close?
    // Allow the close. For windowed browsers this will result in the OS close
    // event being sent.
    return false;
  }

  void OnBeforeClose(CefRefPtr<CefBrowser> browser) OVERRIDE {
    printf("OnBeforeClose\n");

    CEF_REQUIRE_UI_THREAD();

    browser_ = nullptr;

    CefQuitMessageLoop();
  }

 private:
  IMPLEMENT_REFCOUNTING(MyClient);
  DISALLOW_COPY_AND_ASSIGN(MyClient);
};

const char kStartupURL[] = "";

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

  // We need to have the main thread process work
  // so that it can paint
  settings.multi_threaded_message_loop = false;

  // Specify the path for the sub-process executable.
  CefString(&settings.browser_subprocess_path).FromASCII("cef.exe");

  app = new MyApp(onPaintCallback);

  // Initialize CEF in the main process.
  if (!CefInitialize(main_args, settings, app.get(), NULL)) {
    return -1;
  }

  return 0;
}

extern "C" int cef_free() {
  printf("closebrowser\n");
  app->client->browser_->GetHost()->CloseBrowser(false);

  printf("wait\n");
  CefRunMessageLoop();

  printf("shut\n");
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

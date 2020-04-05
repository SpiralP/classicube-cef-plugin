
#include <include/cef_app.h>
#include <include/cef_base.h>
#include <include/cef_browser.h>
#include <include/cef_command_line.h>
#include <include/cef_render_handler.h>
#include <include/views/cef_browser_view.h>
#include <include/views/cef_window.h>
#include <include/wrapper/cef_helpers.h>

class MyRenderHandler : public CefRenderHandler {
 public:
  MyRenderHandler();

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

  void GetViewRect(CefRefPtr<CefBrowser> browser, CefRect& rect) = 0;

  void OnAcceleratedPaint(CefRefPtr<CefBrowser> browser,
                          CefRenderHandler::PaintElementType type,
                          const CefRenderHandler::RectList& dirtyRects,
                          void* shared_handle) OVERRIDE {
    //
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
               const void* buffer,
               int width,
               int height) OVERRIDE {
    //
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
};

extern "C" int cef_init() {
  // Enable High-DPI support on Windows 7 or newer.
  CefEnableHighDPISupport();

  // Structure for passing command-line arguments.
  // The definition of this structure is platform-specific.
  CefMainArgs main_args;

  // Optional implementation of the CefApp interface.
  // CefRefPtr<SimpleApp> app(new SimpleApp);

  // Populate this structure to customize CEF behavior.
  CefSettings settings;

  settings.no_sandbox = true;

  // Specify the path for the sub-process executable.
  CefString(&settings.browser_subprocess_path).FromASCII("cef.exe");

  // Initialize CEF in the main process.
  if (!CefInitialize(main_args, settings, NULL, NULL)) {
    return -1;
  }

  // Run the CEF message loop. This will block until CefQuitMessageLoop() is
  // called.
  CefRunMessageLoop();

  return 0;
}

extern "C" int cef_free() {
  // Shut down CEF.
  CefShutdown();

  return 0;
}

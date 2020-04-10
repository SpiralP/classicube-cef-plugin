#include <include/cef_render_handler.h>
#include "interface.hh"

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
    // If this method returns false the rectangle from GetViewRect will be used
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
    rust_print("GetViewRect");
    rect.x = 0;
    rect.y = 0;
    rect.width = 1920;
    rect.height = 1080;
  }

  void OnAcceleratedPaint(CefRefPtr<CefBrowser> browser,
                          CefRenderHandler::PaintElementType type,
                          const CefRenderHandler::RectList& dirtyRects,
                          void* shared_handle) OVERRIDE {
    rust_print("OnAcceleratedPaint");
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
    // rust_print("OnPaint");

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

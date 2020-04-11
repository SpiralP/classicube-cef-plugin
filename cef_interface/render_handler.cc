#include "render_handler.hh"

MyRenderHandler::MyRenderHandler(OnPaintCallback on_paint_callback) {
  this->on_paint_callback = on_paint_callback;
}

void MyRenderHandler::GetViewRect(CefRefPtr<CefBrowser> browser,
                                  CefRect& rect) {
  rust_print("GetViewRect");
  rect.x = 0;
  rect.y = 0;
  rect.width = 1920;
  rect.height = 1080;
}

void MyRenderHandler::OnPaint(CefRefPtr<CefBrowser> browser,
                              CefRenderHandler::PaintElementType type,
                              const CefRenderHandler::RectList& dirtyRects,
                              const void* pixels,
                              int width,
                              int height) {
  // rust_print("OnPaint");

  on_paint_callback(create_rust_ref_browser(browser.get()), pixels, width,
                    height);
}

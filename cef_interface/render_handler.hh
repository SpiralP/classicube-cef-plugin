#pragma once

#include <include/cef_base.h>
#include <include/cef_render_handler.h>
#include "interface.hh"

class MyRenderHandler : public CefRenderHandler {
 public:
  MyRenderHandler(OnPaintCallback on_paint_callback);

  void GetViewRect(CefRefPtr<CefBrowser> browser, CefRect& rect) OVERRIDE;

  void OnPaint(CefRefPtr<CefBrowser> browser,
               CefRenderHandler::PaintElementType type,
               const CefRenderHandler::RectList& dirtyRects,
               const void* pixels,
               int width,
               int height) OVERRIDE;

 private:
  OnPaintCallback on_paint_callback;

  IMPLEMENT_REFCOUNTING(MyRenderHandler);
  DISALLOW_COPY_AND_ASSIGN(MyRenderHandler);
};

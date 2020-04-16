#pragma once

#include <include/cef_client.h>
#include <include/wrapper/cef_helpers.h>
#include "interface.hh"

class MyClient : public CefClient,
                 public CefDisplayHandler,
                 public CefLifeSpanHandler,
                 public CefRenderHandler,
                 public CefLoadHandler {
 public:
  MyClient(OnBeforeCloseCallback on_before_close_callback,
           OnPaintCallback on_paint_callback,
           OnLoadEndCallback on_load_end_callback);

  // CefClient methods:
  CefRefPtr<CefDisplayHandler> GetDisplayHandler() OVERRIDE;
  CefRefPtr<CefLifeSpanHandler> GetLifeSpanHandler() OVERRIDE;
  CefRefPtr<CefRenderHandler> GetRenderHandler() OVERRIDE;
  CefRefPtr<CefLoadHandler> GetLoadHandler() OVERRIDE;

  // CefDisplayHandler
  void OnTitleChange(CefRefPtr<CefBrowser> browser,
                     const CefString& title) OVERRIDE;

  // CefLifeSpanHandler
  bool DoClose(CefRefPtr<CefBrowser> browser) OVERRIDE;
  void OnBeforeClose(CefRefPtr<CefBrowser> browser) OVERRIDE;

  // CefRenderHandler
  void GetViewRect(CefRefPtr<CefBrowser> browser, CefRect& rect) OVERRIDE;
  void OnPaint(CefRefPtr<CefBrowser> browser,
               CefRenderHandler::PaintElementType type,
               const CefRenderHandler::RectList& dirtyRects,
               const void* pixels,
               int width,
               int height) OVERRIDE;

  // CefLoadHandler
  void OnLoadEnd(CefRefPtr<CefBrowser> browser,
                 CefRefPtr<CefFrame> frame,
                 int httpStatusCode) OVERRIDE;

 private:
  OnBeforeCloseCallback on_before_close_callback;
  OnPaintCallback on_paint_callback;
  OnLoadEndCallback on_load_end_callback;
  // OnLoadErrorCallback on_load_error_callback;

  IMPLEMENT_REFCOUNTING(MyClient);
  DISALLOW_COPY_AND_ASSIGN(MyClient);
};

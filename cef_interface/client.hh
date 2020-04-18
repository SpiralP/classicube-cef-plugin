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
  MyClient(Callbacks callbacks);

  // CefClient methods:
  CefRefPtr<CefDisplayHandler> GetDisplayHandler() OVERRIDE;
  CefRefPtr<CefLifeSpanHandler> GetLifeSpanHandler() OVERRIDE;
  CefRefPtr<CefRenderHandler> GetRenderHandler() OVERRIDE;
  CefRefPtr<CefLoadHandler> GetLoadHandler() OVERRIDE;

  // CefDisplayHandler methods:
  void OnTitleChange(CefRefPtr<CefBrowser> browser,
                     const CefString& title) OVERRIDE;

  // CefLifeSpanHandler methods:
  void OnAfterCreated(CefRefPtr<CefBrowser> browser) OVERRIDE;
  bool DoClose(CefRefPtr<CefBrowser> browser) OVERRIDE;
  void OnBeforeClose(CefRefPtr<CefBrowser> browser) OVERRIDE;

  // CefRenderHandler methods:
  void GetViewRect(CefRefPtr<CefBrowser> browser, CefRect& rect) OVERRIDE;
  void OnPaint(CefRefPtr<CefBrowser> browser,
               CefRenderHandler::PaintElementType type,
               const CefRenderHandler::RectList& dirtyRects,
               const void* pixels,
               int width,
               int height) OVERRIDE;

  // CefLoadHandler methods:
  void OnLoadEnd(CefRefPtr<CefBrowser> browser,
                 CefRefPtr<CefFrame> frame,
                 int httpStatusCode) OVERRIDE;

 private:
  OnBeforeCloseCallback on_before_close_callback;
  OnPaintCallback on_paint_callback;
  OnLoadEndCallback on_load_end_callback;
  OnAfterCreatedCallback on_after_created_callback;

  IMPLEMENT_REFCOUNTING(MyClient);
  DISALLOW_COPY_AND_ASSIGN(MyClient);
};

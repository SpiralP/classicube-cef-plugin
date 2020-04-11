#pragma once

#include <include/cef_client.h>
#include <include/wrapper/cef_helpers.h>
#include "interface.hh"
#include "render_handler.hh"

class MyClient : public CefClient,
                 public CefDisplayHandler,
                 public CefLifeSpanHandler {
 public:
  MyClient(OnAfterCreatedCallback on_after_created_callback,
           OnBeforeCloseCallback on_before_close_callback,
           OnPaintCallback on_paint_callback);

  // CefClient methods:
  CefRefPtr<CefDisplayHandler> GetDisplayHandler() OVERRIDE;
  CefRefPtr<CefLifeSpanHandler> GetLifeSpanHandler() OVERRIDE;
  CefRefPtr<CefRenderHandler> GetRenderHandler() OVERRIDE;

  void OnTitleChange(CefRefPtr<CefBrowser> browser,
                     const CefString& title) OVERRIDE;

  void OnAfterCreated(CefRefPtr<CefBrowser> browser) OVERRIDE;

  bool DoClose(CefRefPtr<CefBrowser> browser) OVERRIDE;

  void OnBeforeClose(CefRefPtr<CefBrowser> browser) OVERRIDE;

 private:
  OnAfterCreatedCallback on_after_created_callback;
  OnBeforeCloseCallback on_before_close_callback;
  CefRefPtr<CefRenderHandler> render_handler;

  IMPLEMENT_REFCOUNTING(MyClient);
  DISALLOW_COPY_AND_ASSIGN(MyClient);
};

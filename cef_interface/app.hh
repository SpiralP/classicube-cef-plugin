#pragma once

#include <include/cef_app.h>
#include "client.hh"
#include "interface.hh"

// Minimal implementation of CefApp for the browser process.
class MyApp : public CefApp, public CefBrowserProcessHandler {
 public:
  MyApp(OnContextInitializedCallback on_context_initialized_callback,
        OnAfterCreatedCallback on_after_created_callback,
        OnBeforeCloseCallback on_before_close_callback,
        OnPaintCallback on_paint_callback);

  // CefApp methods:
  CefRefPtr<CefBrowserProcessHandler> GetBrowserProcessHandler() OVERRIDE;

  void OnBeforeCommandLineProcessing(
      const CefString& process_type,
      CefRefPtr<CefCommandLine> command_line) OVERRIDE;

  // CefBrowserProcessHandler methods:
  void OnContextInitialized() OVERRIDE;

 private:
  OnContextInitializedCallback on_context_initialized_callback;
  CefRefPtr<MyClient> client;

  IMPLEMENT_REFCOUNTING(MyApp);
  DISALLOW_COPY_AND_ASSIGN(MyApp);
};

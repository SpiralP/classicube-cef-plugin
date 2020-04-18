#pragma once

#include <include/cef_app.h>

#include "client.hh"
#include "interface.hh"

class MyApp : public CefApp, public CefBrowserProcessHandler {
 public:
  MyApp(Callbacks callbacks);

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

#pragma once

#include <include/cef_app.h>

#include "client.hh"
#include "interface.hh"

class MyApp : public CefApp,
              public CefBrowserProcessHandler,
              public CefRenderProcessHandler {
 public:
  MyApp(Callbacks callbacks);

  // CefApp methods:
  CefRefPtr<CefBrowserProcessHandler> GetBrowserProcessHandler() OVERRIDE;
  CefRefPtr<CefRenderProcessHandler> GetRenderProcessHandler() OVERRIDE;

  void OnBeforeCommandLineProcessing(
      const CefString& process_type,
      CefRefPtr<CefCommandLine> command_line) OVERRIDE;

  void OnRegisterCustomSchemes(
      CefRawPtr<CefSchemeRegistrar> registrar) OVERRIDE;

  // CefBrowserProcessHandler methods:
  void OnContextInitialized() OVERRIDE;

  // CefRenderProcessHandler methods:
  bool OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                CefRefPtr<CefFrame> frame,
                                CefProcessId source_process,
                                CefRefPtr<CefProcessMessage> message) OVERRIDE;

 private:
  Callbacks callbacks;
  CefRefPtr<MyClient> client;

  IMPLEMENT_REFCOUNTING(MyApp);
  DISALLOW_COPY_AND_ASSIGN(MyApp);
};

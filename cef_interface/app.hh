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
  CefRefPtr<CefBrowserProcessHandler> GetBrowserProcessHandler() override;
  CefRefPtr<CefRenderProcessHandler> GetRenderProcessHandler() override;

  void OnBeforeCommandLineProcessing(
      const CefString& process_type,
      CefRefPtr<CefCommandLine> command_line) override;

  void OnRegisterCustomSchemes(
      CefRawPtr<CefSchemeRegistrar> registrar) override;

  // CefBrowserProcessHandler methods:
  void OnContextInitialized() override;

  // CefRenderProcessHandler methods:
  bool OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                CefRefPtr<CefFrame> frame,
                                CefProcessId source_process,
                                CefRefPtr<CefProcessMessage> message) override;

 private:
  Callbacks callbacks;
  CefRefPtr<MyClient> client;

  IMPLEMENT_REFCOUNTING(MyApp);
  DISALLOW_COPY_AND_ASSIGN(MyApp);
};

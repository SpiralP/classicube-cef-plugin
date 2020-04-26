#pragma once

#include <include/cef_client.h>
#include <include/wrapper/cef_helpers.h>

#include "interface.hh"

class MyClient : public CefClient,
                 public CefDisplayHandler,
                 public CefLifeSpanHandler,
                 public CefRenderHandler,
                 public CefLoadHandler,
                 public CefRequestHandler,
                 public CefResourceRequestHandler,
                 public CefJSDialogHandler,
                 public CefDialogHandler,
                 public CefDownloadHandler {
 public:
  MyClient(Callbacks callbacks);

  // CefClient methods:
  CefRefPtr<CefDisplayHandler> GetDisplayHandler() OVERRIDE;
  CefRefPtr<CefLifeSpanHandler> GetLifeSpanHandler() OVERRIDE;
  CefRefPtr<CefRenderHandler> GetRenderHandler() OVERRIDE;
  CefRefPtr<CefLoadHandler> GetLoadHandler() OVERRIDE;
  CefRefPtr<CefRequestHandler> GetRequestHandler() OVERRIDE;
  CefRefPtr<CefJSDialogHandler> GetJSDialogHandler() OVERRIDE;
  CefRefPtr<CefDialogHandler> GetDialogHandler() OVERRIDE;
  CefRefPtr<CefDownloadHandler> GetDownloadHandler() OVERRIDE;

  // CefDisplayHandler methods:
  void OnTitleChange(CefRefPtr<CefBrowser> browser,
                     const CefString& title) OVERRIDE;

  void OnLoadingProgressChange(CefRefPtr<CefBrowser> browser,
                               double progress) OVERRIDE;

  // CefLifeSpanHandler methods:
  bool DoClose(CefRefPtr<CefBrowser> browser) OVERRIDE;
  void OnAfterCreated(CefRefPtr<CefBrowser> browser) OVERRIDE;
  void OnBeforeClose(CefRefPtr<CefBrowser> browser) OVERRIDE;
  bool OnBeforePopup(
      CefRefPtr<CefBrowser> browser,
      CefRefPtr<CefFrame> frame,
      const CefString& target_url,
      const CefString& target_frame_name,
      CefLifeSpanHandler::WindowOpenDisposition target_disposition,
      bool user_gesture,
      const CefPopupFeatures& popupFeatures,
      CefWindowInfo& windowInfo,
      CefRefPtr<CefClient>& client,
      CefBrowserSettings& settings,
      CefRefPtr<CefDictionaryValue>& extra_info,
      bool* no_javascript_access) OVERRIDE;

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

  // CefRequestHandler methods:
  CefRefPtr<CefResourceRequestHandler> GetResourceRequestHandler(
      CefRefPtr<CefBrowser> browser,
      CefRefPtr<CefFrame> frame,
      CefRefPtr<CefRequest> request,
      bool is_navigation,
      bool is_download,
      const CefString& request_initiator,
      bool& disable_default_handling) OVERRIDE;

  // CefResourceRequestHandler methods:
  CefResourceRequestHandler::ReturnValue OnBeforeResourceLoad(
      CefRefPtr<CefBrowser> browser,
      CefRefPtr<CefFrame> frame,
      CefRefPtr<CefRequest> request,
      CefRefPtr<CefRequestCallback> callback) OVERRIDE;

  // CefJSDialogHandler methods:
  bool OnJSDialog(CefRefPtr<CefBrowser> browser,
                  const CefString& origin_url,
                  CefJSDialogHandler::JSDialogType dialog_type,
                  const CefString& message_text,
                  const CefString& default_prompt_text,
                  CefRefPtr<CefJSDialogCallback> callback,
                  bool& suppress_message) OVERRIDE;

  // CefDialogHandler methods:
  bool OnFileDialog(CefRefPtr<CefBrowser> browser,
                    CefDialogHandler::FileDialogMode mode,
                    const CefString& title,
                    const CefString& default_file_path,
                    const std::vector<CefString>& accept_filters,
                    int selected_accept_filter,
                    CefRefPtr<CefFileDialogCallback> callback) OVERRIDE;

  // CefDownloadHandler methods:
  void OnBeforeDownload(CefRefPtr<CefBrowser> browser,
                        CefRefPtr<CefDownloadItem> download_item,
                        const CefString& suggested_name,
                        CefRefPtr<CefBeforeDownloadCallback> callback) OVERRIDE;

 private:
  OnBeforeCloseCallback on_before_close_callback;
  OnPaintCallback on_paint_callback;
  OnLoadEndCallback on_load_end_callback;
  OnAfterCreatedCallback on_after_created_callback;
  OnTitleChangeCallback on_title_change_callback;

  IMPLEMENT_REFCOUNTING(MyClient);
  DISALLOW_COPY_AND_ASSIGN(MyClient);
};

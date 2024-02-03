#include "client.hh"

#include "serialize.hh"

MyClient::MyClient(Callbacks callbacks_) {
  this->callbacks = callbacks_;
}

// CefClient methods:
CefRefPtr<CefDisplayHandler> MyClient::GetDisplayHandler() {
  return this;
}
CefRefPtr<CefLifeSpanHandler> MyClient::GetLifeSpanHandler() {
  return this;
}
CefRefPtr<CefRenderHandler> MyClient::GetRenderHandler() {
  return this;
}
CefRefPtr<CefLoadHandler> MyClient::GetLoadHandler() {
  return this;
}
CefRefPtr<CefRequestHandler> MyClient::GetRequestHandler() {
  return this;
}
CefRefPtr<CefJSDialogHandler> MyClient::GetJSDialogHandler() {
  return this;
}
CefRefPtr<CefDialogHandler> MyClient::GetDialogHandler() {
  return this;
}
CefRefPtr<CefDownloadHandler> MyClient::GetDownloadHandler() {
  return this;
}

bool MyClient::OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                        CefRefPtr<CefFrame> frame,
                                        CefProcessId source_process,
                                        CefRefPtr<CefProcessMessage> message) {
  // this is called on our main thread

  auto message_name = message->GetName();

  if (message_name == "EvalJavascriptReturn") {
    CefRefPtr<CefListValue> args = message->GetArgumentList();
    uint64_t task_id = 0;
    args->GetBinary(0)->GetData(&task_id, sizeof(uint64_t), 0);

    if (callbacks.on_javascript) {
      auto binary = args->GetBinary(1);

      auto v8_response = deserialize_v8_response(binary.get());
      callbacks.on_javascript(cef_interface_add_ref_browser(browser.get()),
                              task_id, v8_response);
    }

    return true;
  }

  return false;
}

// CefDisplayHandler methods:
void MyClient::OnTitleChange(CefRefPtr<CefBrowser> browser,
                             const CefString& title) {
  if (callbacks.on_title_change) {
    auto title_utf8 = title.ToString();
    callbacks.on_title_change(cef_interface_add_ref_browser(browser.get()),
                              title_utf8.c_str());
  }
}
void MyClient::OnLoadingProgressChange(CefRefPtr<CefBrowser> browser,
                                       double progress) {
  // auto ag = std::to_string(progress);
  // rust_debug(ag.c_str());
}

// CefLifeSpanHandler methods:
void MyClient::OnBeforeClose(CefRefPtr<CefBrowser> browser) {
  if (callbacks.on_before_close) {
    callbacks.on_before_close(cef_interface_add_ref_browser(browser.get()));
  }
}

void MyClient::OnAfterCreated(CefRefPtr<CefBrowser> browser) {
  if (callbacks.on_after_created) {
    callbacks.on_after_created(cef_interface_add_ref_browser(browser.get()));
  }
}

bool MyClient::DoClose(CefRefPtr<CefBrowser> browser) {
  rust_debug("DoClose");

  return false;
}

bool MyClient::OnBeforePopup(
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
    bool* no_javascript_access) {
  rust_debug("popup detected");

  frame->LoadURL(target_url);

  // block the popup
  return true;
}

// CefRenderHandler methods:
void MyClient::GetViewRect(CefRefPtr<CefBrowser> browser, CefRect& rect) {
  if (callbacks.get_view_rect) {
    auto new_rect =
        callbacks.get_view_rect(cef_interface_add_ref_browser(browser.get()));
    rect.x = new_rect.x;
    rect.y = new_rect.y;
    rect.width = new_rect.width;
    rect.height = new_rect.height;
  } else {
    rect.x = 0;
    rect.y = 0;
    rect.width = 180;
    rect.height = 100;
  }
}

void MyClient::OnPaint(CefRefPtr<CefBrowser> browser,
                       CefRenderHandler::PaintElementType type,
                       const CefRenderHandler::RectList& dirtyRects,
                       const void* pixels,
                       int width,
                       int height) {
  if (callbacks.on_paint) {
    callbacks.on_paint(cef_interface_add_ref_browser(browser.get()), pixels,
                       width, height);
  }
}

// CefLoadHandler methods:
void MyClient::OnLoadEnd(CefRefPtr<CefBrowser> browser,
                         CefRefPtr<CefFrame> frame,
                         int httpStatusCode) {
  if (frame->IsMain()) {
    if (callbacks.on_load_end) {
      callbacks.on_load_end(cef_interface_add_ref_browser(browser.get()));
    }
  }
}

// CefRequestHandler methods:
CefRefPtr<CefResourceRequestHandler> MyClient::GetResourceRequestHandler(
    CefRefPtr<CefBrowser> browser,
    CefRefPtr<CefFrame> frame,
    CefRefPtr<CefRequest> request,
    bool is_navigation,
    bool is_download,
    const CefString& request_initiator,
    bool& disable_default_handling) {
  auto referrer_url = request->GetReferrerURL();
  if (!referrer_url.c_str()) {
    std::string url = request->GetURL();
    auto main_url = frame->GetURL();

    if (main_url == "" && url.rfind("https://www.youtube.com/embed/", 0) == 0) {
      // use MyClient::OnBeforeResourceLoad
      return this;
    }
  }

  return nullptr;
}

bool MyClient::OnCertificateError(CefRefPtr<CefBrowser> browser,
                                  cef_errorcode_t cert_error,
                                  const CefString& request_url,
                                  CefRefPtr<CefSSLInfo> ssl_info,
                                  CefRefPtr<CefCallback> callback) {
  rust_warn("OnCertificateError");
  if (callbacks.on_certificate_error) {
    bool allow = callbacks.on_certificate_error(
        cef_interface_add_ref_browser(browser.get()));

    if (allow) {
      // Return true and call CefRequestCallback::Continue() either in this
      // method or at a later time to continue or cancel the request.
      callback->Continue();
      return true;
    }
  }

  // Return false to cancel the request immediately.
  return false;
}

// CefResourceRequestHandler methods:
CefResourceRequestHandler::ReturnValue MyClient::OnBeforeResourceLoad(
    CefRefPtr<CefBrowser> browser,
    CefRefPtr<CefFrame> frame,
    CefRefPtr<CefRequest> request,
    CefRefPtr<CefCallback> callback) {
  // fix for some embedded youtube videos giving "video unavailable"
  // something to do with referrer not being set from our data: url
  auto new_referrer_url = L"https://www.youtube.com/";
  request->SetReferrer(new_referrer_url,
                       CefRequest::ReferrerPolicy::REFERRER_POLICY_DEFAULT);

  return CefResourceRequestHandler::ReturnValue::RV_CONTINUE;
}

// CefJSDialogHandler methods:
bool MyClient::OnBeforeUnloadDialog(CefRefPtr<CefBrowser> browser,
                                    const CefString& message_text,
                                    bool is_reload,
                                    CefRefPtr<CefJSDialogCallback> callback) {
  CefString user_input;
  callback->Continue(true, user_input);
  // Return true if the application will use a custom dialog or if the callback
  // has been executed immediately.
  return true;
}

bool MyClient::OnJSDialog(CefRefPtr<CefBrowser> browser,
                          const CefString& origin_url,
                          CefJSDialogHandler::JSDialogType dialog_type,
                          const CefString& message_text,
                          const CefString& default_prompt_text,
                          CefRefPtr<CefJSDialogCallback> callback,
                          bool& suppress_message) {
  // Set |suppress_message| to true and return false to suppress the message
  suppress_message = true;
  return false;
}

// CefDialogHandler methods:
#if CEF_VERSION_MAJOR > 101
bool MyClient::OnFileDialog(CefRefPtr<CefBrowser> browser,
                            FileDialogMode mode,
                            const CefString& title,
                            const CefString& default_file_path,
                            const std::vector<CefString>& accept_filters,
                            CefRefPtr<CefFileDialogCallback> callback) {
  // To display a custom dialog return true and execute |callback| either inline
  // or at a later time.
  callback->Cancel();
  return true;
}
#else
bool MyClient::OnFileDialog(CefRefPtr<CefBrowser> browser,
                            FileDialogMode mode,
                            const CefString& title,
                            const CefString& default_file_path,
                            const std::vector<CefString>& accept_filters,
                            int selected_accept_filter,
                            CefRefPtr<CefFileDialogCallback> callback) {
  // To display a custom dialog return true and execute |callback| either inline
  // or at a later time.
  callback->Cancel();
  return true;
}
#endif

// CefDownloadHandler methods:
void MyClient::OnBeforeDownload(CefRefPtr<CefBrowser> browser,
                                CefRefPtr<CefDownloadItem> download_item,
                                const CefString& suggested_name,
                                CefRefPtr<CefBeforeDownloadCallback> callback) {
  // By default the download will be canceled.
  rust_debug("OnBeforeDownload");
}

void MyClient::OnDownloadUpdated(CefRefPtr<CefBrowser> browser,
                                 CefRefPtr<CefDownloadItem> download_item,
                                 CefRefPtr<CefDownloadItemCallback> callback) {
  rust_debug("OnDownloadUpdated");
  // Execute |callback| either asynchronously or in this method to cancel the
  // download if desired.
  callback->Cancel();
}

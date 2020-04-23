#include "client.hh"

MyClient::MyClient(Callbacks callbacks) {
  this->on_before_close_callback = callbacks.on_before_close_callback;
  this->on_paint_callback = callbacks.on_paint_callback;
  this->on_load_end_callback = callbacks.on_load_end_callback;
  this->on_after_created_callback = callbacks.on_after_created_callback;
  this->on_title_change_callback = callbacks.on_title_change_callback;
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

// CefDisplayHandler methods:
void MyClient::OnTitleChange(CefRefPtr<CefBrowser> browser,
                             const CefString& title) {
  if (on_title_change_callback) {
    auto title_utf8 = title.ToString();
    on_title_change_callback(cef_interface_add_ref_browser(browser.get()),
                             title_utf8.c_str());
  }
}

// CefLifeSpanHandler methods:
void MyClient::OnBeforeClose(CefRefPtr<CefBrowser> browser) {
  if (on_before_close_callback) {
    on_before_close_callback(cef_interface_add_ref_browser(browser.get()));
  }
}

void MyClient::OnAfterCreated(CefRefPtr<CefBrowser> browser) {
  if (on_after_created_callback) {
    on_after_created_callback(cef_interface_add_ref_browser(browser.get()));
  }
}

bool MyClient::DoClose(CefRefPtr<CefBrowser> browser) {
  rust_print("DoClose");

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
  rust_print("popup detected");

  frame->LoadURL(target_url);

  // block the popup
  return true;
}

// CefRenderHandler methods:
void MyClient::GetViewRect(CefRefPtr<CefBrowser> browser, CefRect& rect) {
  rect.x = 0;
  rect.y = 0;
  rect.width = 1920;
  rect.height = 1080;
}

void MyClient::OnPaint(CefRefPtr<CefBrowser> browser,
                       CefRenderHandler::PaintElementType type,
                       const CefRenderHandler::RectList& dirtyRects,
                       const void* pixels,
                       int width,
                       int height) {
  if (on_paint_callback) {
    on_paint_callback(cef_interface_add_ref_browser(browser.get()), pixels,
                      width, height);
  }
}

// CefLoadHandler methods:
void MyClient::OnLoadEnd(CefRefPtr<CefBrowser> browser,
                         CefRefPtr<CefFrame> frame,
                         int httpStatusCode) {
  if (frame->IsMain()) {
    if (on_load_end_callback) {
      on_load_end_callback(cef_interface_add_ref_browser(browser.get()));
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
  return this;
}

// CefResourceRequestHandler methods:
CefResourceRequestHandler::ReturnValue MyClient::OnBeforeResourceLoad(
    CefRefPtr<CefBrowser> browser,
    CefRefPtr<CefFrame> frame,
    CefRefPtr<CefRequest> request,
    CefRefPtr<CefRequestCallback> callback) {
  // fix for some embedded youtube videos giving "video unavailable"
  // something to do with referrer not being set from our data: url
  auto referrer_url = request->GetReferrerURL();
  if (!referrer_url.c_str()) {
    std::string url = request->GetURL();
    auto main_url = frame->GetURL();

    if (main_url == "" && url.rfind("https://www.youtube.com/embed/", 0) == 0) {
      auto new_referrer_url = L"https://www.youtube.com/";
      request->SetReferrer(new_referrer_url,
                           CefRequest::ReferrerPolicy::REFERRER_POLICY_DEFAULT);
    }
  }

  return CefResourceRequestHandler::ReturnValue::RV_CONTINUE;
}

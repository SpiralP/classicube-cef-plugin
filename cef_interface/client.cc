#include "client.hh"

MyClient::MyClient(Callbacks callbacks) {
  this->on_before_close_callback = callbacks.on_before_close_callback;
  this->on_paint_callback = callbacks.on_paint_callback;
  this->on_load_end_callback = callbacks.on_load_end_callback;
  this->on_after_created_callback = callbacks.on_after_created_callback;
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

// CefDisplayHandler methods:
void MyClient::OnTitleChange(CefRefPtr<CefBrowser> browser,
                             const CefString& title) {}

// CefLifeSpanHandler methods:
void MyClient::OnAfterCreated(CefRefPtr<CefBrowser> browser) {
  if (on_after_created_callback) {
    on_after_created_callback(cef_interface_add_ref_browser(browser.get()));
  }
}

bool MyClient::DoClose(CefRefPtr<CefBrowser> browser) {
  rust_print("DoClose");

  // Must be executed on the UI thread.
  CEF_REQUIRE_UI_THREAD();

  // force close?
  // Allow the close. For windowed browsers this will result in the OS close
  // event being sent.
  return false;
}

void MyClient::OnBeforeClose(CefRefPtr<CefBrowser> browser) {
  CEF_REQUIRE_UI_THREAD();

  if (on_before_close_callback) {
    on_before_close_callback(cef_interface_add_ref_browser(browser.get()));
  }
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

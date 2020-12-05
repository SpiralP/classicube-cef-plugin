#include "app.hh"
#include "serialize.hh"

// Minimal implementation of CefApp for the browser process.

MyApp::MyApp(Callbacks callbacks_) {
  this->callbacks = callbacks_;
  this->client = new MyClient(callbacks_);
}

// CefApp methods:
CefRefPtr<CefBrowserProcessHandler> MyApp::GetBrowserProcessHandler() {
  return this;
}
CefRefPtr<CefRenderProcessHandler> MyApp::GetRenderProcessHandler() {
  return this;
}

void MyApp::OnBeforeCommandLineProcessing(
    const CefString& process_type,
    CefRefPtr<CefCommandLine> command_line) {
  command_line->AppendSwitchWithValue("autoplay-policy",
                                      "no-user-gesture-required");
  command_line->AppendSwitch("disable-extensions");
  // command_line->AppendSwitch("disable-web-security");
  // command_line->AppendSwitch("ignore-certificate-errors");
  // command_line->AppendSwitch("disable-site-isolation-trials");

  // don't show up in media info on windows volume change popup
  std::string new_value("HardwareMediaKeyHandling");
  if (command_line->HasSwitch("disable-features")) {
    CefString old_value = command_line->GetSwitchValue("disable-features");
    new_value += ",";
    new_value += old_value;
  }
  command_line->AppendSwitchWithValue("disable-features", new_value);
}

void MyApp::OnRegisterCustomSchemes(CefRawPtr<CefSchemeRegistrar> registrar) {
  if (!registrar->AddCustomScheme("local", CEF_SCHEME_OPTION_STANDARD)) {
    rust_warn("AddCustomScheme local failed!");
  }
}

// CefBrowserProcessHandler methods:
void MyApp::OnContextInitialized() {
  if (callbacks.on_context_initialized) {
    callbacks.on_context_initialized(
        cef_interface_add_ref_client(this->client.get()));
  }
}

// CefRenderProcessHandler methods:
bool MyApp::OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                     CefRefPtr<CefFrame> frame,
                                     CefProcessId source_process,
                                     CefRefPtr<CefProcessMessage> message) {
  // this is called in the render sub-process

  auto message_name = message->GetName();

  if (message_name == "EvalJavascript") {
    CefRefPtr<CefListValue> args = message->GetArgumentList();

    uint64_t task_id = 0;
    args->GetBinary(0)->GetData(&task_id, sizeof(uint64_t), 0);
    auto script = args->GetString(1);
    auto script_url = args->GetString(2);
    auto start_line = args->GetInt(3);

    auto response_message = CefProcessMessage::Create("EvalJavascriptReturn");
    CefRefPtr<CefListValue> response_args = response_message->GetArgumentList();
    response_args->SetBinary(0, args->GetBinary(0));

    auto context = frame->GetV8Context();

    CefRefPtr<CefV8Exception> exception;
    CefRefPtr<CefV8Value> result;

    context->Enter();
    bool success =
        context->Eval(script, script_url, start_line, result, exception);
    context->Exit();

    FFIRustV8Response v8_response;
    if (success) {
      v8_response.success = true;
      v8_response.result = create_rust_v8_value(result.get());

    } else {
      rust_debug("js error");
      // TODO

      v8_response.success = false;
      v8_response.error = true;
    }

    auto serialized = serialize_v8_response(v8_response);
    response_args->SetBinary(1, serialized);

    frame->SendProcessMessage(source_process, response_message);

    return true;
  }

  return false;
}

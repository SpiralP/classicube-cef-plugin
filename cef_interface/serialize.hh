#pragma once

#include <include/cef_app.h>

#include "interface.hh"

FFIRustV8Value create_rust_v8_value(CefV8Value* v);

CefRefPtr<CefBinaryValue> serialize_v8_response(FFIRustV8Response v8_response);
FFIRustV8Response deserialize_v8_response(CefBinaryValue* binary);

#include "serialize.hh"

FFIRustV8Value create_rust_v8_value(CefV8Value* v) {
  FFIRustV8Value rust_value;
  rust_value.tag = FFIRustV8Value::Tag::Unknown;

  if (v->IsArray()) {
    rust_value.tag = FFIRustV8Value::Tag::Array;
  } else if (v->IsArrayBuffer()) {
    rust_value.tag = FFIRustV8Value::Tag::ArrayBuffer;
  } else if (v->IsBool()) {
    rust_value.tag = FFIRustV8Value::Tag::Bool;
    rust_value.bool_ = v->GetBoolValue();
  } else if (v->IsDate()) {
    rust_value.tag = FFIRustV8Value::Tag::Date;
  } else if (v->IsDouble()) {
    rust_value.tag = FFIRustV8Value::Tag::Double;
    rust_value.double_ = v->GetDoubleValue();
  } else if (v->IsFunction()) {
    rust_value.tag = FFIRustV8Value::Tag::Function;
  } else if (v->IsInt()) {
    rust_value.tag = FFIRustV8Value::Tag::Int;
    rust_value.int_ = v->GetIntValue();
  } else if (v->IsNull()) {
    rust_value.tag = FFIRustV8Value::Tag::Null;
  } else if (v->IsObject()) {
    rust_value.tag = FFIRustV8Value::Tag::Object;
  } else if (v->IsString()) {
    rust_value.tag = FFIRustV8Value::Tag::String;
    std::string s = v->GetStringValue().ToString();
    rust_value.string = cef_interface_new_ref_string(s.c_str(), s.length());
  } else if (v->IsUInt()) {
    rust_value.tag = FFIRustV8Value::Tag::UInt;
    rust_value.uint = v->GetUIntValue();
  } else if (v->IsUndefined()) {
    rust_value.tag = FFIRustV8Value::Tag::Undefined;
  }

  return rust_value;
}

template <typename T>
void write(std::ostringstream& s, T value) {
  s.write(reinterpret_cast<const char*>(&value), sizeof(T));
}

CefRefPtr<CefBinaryValue> serialize_v8_response(FFIRustV8Response response) {
  std::ostringstream s;

  // response.success
  write(s, response.success);

  if (response.success) {
    // response.result

    // result.tag
    write(s, response.result.tag);

    // result.<value>
    if (response.result.tag == FFIRustV8Value::Tag::Bool) {
      write(s, response.result.bool_);
    } else if (response.result.tag == FFIRustV8Value::Tag::Double) {
      write(s, response.result.double_);
    } else if (response.result.tag == FFIRustV8Value::Tag::Int) {
      write(s, response.result.int_);
    } else if (response.result.tag == FFIRustV8Value::Tag::String) {
      write(s, response.result.string.len);
      s.write(response.result.string.ptr, response.result.string.len);
    } else if (response.result.tag == FFIRustV8Value::Tag::UInt) {
      write(s, response.result.uint);
    }
  } else {
    // TODO
  }

  std::string str = s.str();
  auto binary = CefBinaryValue::Create(str.c_str(), str.length());

  return binary;
}

template <typename T>
void read(std::istringstream& s, T* value) {
  s.read(reinterpret_cast<char*>(value), sizeof(T));
}

FFIRustV8Response deserialize_v8_response(CefBinaryValue* binary) {
  size_t len = binary->GetSize();
  char* data = new char[len + 1]();
  binary->GetData(data, len, 0);

  // std::string copies from data
  std::istringstream s(std::string(data, len));
  delete[] data;

  FFIRustV8Response response;

  // response.success
  read(s, &response.success);

  if (response.success) {
    // response.result

    // result.tag
    read(s, &response.result.tag);

    // result.<value>
    if (response.result.tag == FFIRustV8Value::Tag::Bool) {
      read(s, &response.result.bool_);
    } else if (response.result.tag == FFIRustV8Value::Tag::Double) {
      read(s, &response.result.double_);
    } else if (response.result.tag == FFIRustV8Value::Tag::Int) {
      read(s, &response.result.int_);
    } else if (response.result.tag == FFIRustV8Value::Tag::String) {
      size_t string_len = 0;
      read(s, &string_len);

      char* tmp = new char[string_len + 1]();
      s.read(tmp, string_len);
      response.result.string = cef_interface_new_ref_string(tmp, string_len);
      delete[] tmp;

    } else if (response.result.tag == FFIRustV8Value::Tag::UInt) {
      read(s, &response.result.uint);
    }
  } else {
    response.error = true;
  }

  return response;
}

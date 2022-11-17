#include "serialize.hh"

FFIRustV8Value create_rust_v8_value(CefV8Value* v) {
  FFIRustV8Value rust_value;
  rust_value.tag = FFIRustV8ValueTag::Unknown;

  if (v->IsArray()) {
    rust_value.tag = FFIRustV8ValueTag::Array;
  } else if (v->IsArrayBuffer()) {
    rust_value.tag = FFIRustV8ValueTag::ArrayBuffer;
  } else if (v->IsBool()) {
    rust_value.tag = FFIRustV8ValueTag::Bool;
    rust_value.bool_ = v->GetBoolValue();
  } else if (v->IsDate()) {
    rust_value.tag = FFIRustV8ValueTag::Date;
  } else if (v->IsDouble()) {
    rust_value.tag = FFIRustV8ValueTag::Double;
    rust_value.double_ = v->GetDoubleValue();
  } else if (v->IsFunction()) {
    rust_value.tag = FFIRustV8ValueTag::Function;
  } else if (v->IsInt()) {
    rust_value.tag = FFIRustV8ValueTag::Int;
    rust_value.int_ = v->GetIntValue();
  } else if (v->IsNull()) {
    rust_value.tag = FFIRustV8ValueTag::Null;
  } else if (v->IsObject()) {
    rust_value.tag = FFIRustV8ValueTag::Object;
  } else if (v->IsString()) {
    rust_value.tag = FFIRustV8ValueTag::String;
    std::string s = v->GetStringValue().ToString();
    rust_value.string = cef_interface_new_ref_string(s.c_str(), s.length());
  } else if (v->IsUInt()) {
    rust_value.tag = FFIRustV8ValueTag::UInt;
    rust_value.uint = v->GetUIntValue();
  } else if (v->IsUndefined()) {
    rust_value.tag = FFIRustV8ValueTag::Undefined;
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
    if (response.result.tag == FFIRustV8ValueTag::Bool) {
      write(s, response.result.bool_);
    } else if (response.result.tag == FFIRustV8ValueTag::Double) {
      write(s, response.result.double_);
    } else if (response.result.tag == FFIRustV8ValueTag::Int) {
      write(s, response.result.int_);
    } else if (response.result.tag == FFIRustV8ValueTag::String) {
      write(s, response.result.string.len);
      s.write(response.result.string.ptr, response.result.string.len);
    } else if (response.result.tag == FFIRustV8ValueTag::UInt) {
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
    if (response.result.tag == FFIRustV8ValueTag::Bool) {
      read(s, &response.result.bool_);
    } else if (response.result.tag == FFIRustV8ValueTag::Double) {
      read(s, &response.result.double_);
    } else if (response.result.tag == FFIRustV8ValueTag::Int) {
      read(s, &response.result.int_);
    } else if (response.result.tag == FFIRustV8ValueTag::String) {
      size_t string_len = 0;
      read(s, &string_len);

      char* tmp = new char[string_len + 1]();
      s.read(tmp, string_len);
      response.result.string = cef_interface_new_ref_string(tmp, string_len);
      delete[] tmp;

    } else if (response.result.tag == FFIRustV8ValueTag::UInt) {
      read(s, &response.result.uint);
    }
  } else {
    response.error = true;
  }

  return response;
}

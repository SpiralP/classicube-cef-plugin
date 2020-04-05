use std::os::raw::c_int;

extern "C" {
  #[must_use]
  pub fn cef_init() -> c_int;

  #[must_use]
  pub fn cef_free() -> c_int;
}

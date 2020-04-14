use error_chain::error_chain;
use std::os::raw::c_int;

error_chain! {
    foreign_links {
        Fmt(::std::fmt::Error);
        ParseFloatError(::std::num::ParseFloatError);
        ParseIntError(::std::num::ParseIntError);
    }

    errors {
        CefError(return_value: c_int) {
            description("cef error")
            display("cef error {}", return_value)
        }
    }
}

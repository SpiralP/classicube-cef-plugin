pub use error_chain::bail;
use error_chain::error_chain;

error_chain! {
    foreign_links {
        Fmt(::std::fmt::Error);
        ParseFloatError(::std::num::ParseFloatError);
        ParseIntError(::std::num::ParseIntError);
        Url(url::ParseError);
    }

    errors {
        CefError(return_value: ::std::os::raw::c_int) {
            description("cef error")
            display("cef error {}", return_value)
        }
    }
}

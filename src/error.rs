pub use error_chain::bail;
use error_chain::error_chain;

error_chain! {
    foreign_links {
        Fmt(::std::fmt::Error);
        Io(::std::io::Error);
        ParseFloatError(::std::num::ParseFloatError);
        ParseIntError(::std::num::ParseIntError);
        ParseBoolError(::std::str::ParseBoolError);
        Url(url::ParseError);
        Tokio(tokio::task::JoinError);
        Bincode(bincode::Error);
        Base64(base64::DecodeError);
        Clap(clap::Error);
    }

    links {
        Invidious(invidious::Error, invidious::ErrorKind);
    }

    errors {
        CefError(return_value: ::std::os::raw::c_int) {
            description("cef error")
            display("cef error {}", return_value)
        }
    }
}

#![allow(unexpected_cfgs)]

use error_chain::error_chain;
pub use error_chain::{bail, ensure};

error_chain! {
    foreign_links {
        Fmt(::std::fmt::Error);
        Io(::std::io::Error);
        NulError(::std::ffi::NulError);
        ParseBoolError(::std::str::ParseBoolError);
        ParseFloatError(::std::num::ParseFloatError);
        ParseIntError(::std::num::ParseIntError);
        Utf8Error(::std::str::Utf8Error);

        Base64(base64::DecodeError);
        BincodeDecode(bincode::error::DecodeError);
        BincodeEncode(bincode::error::EncodeError);
        Clap(clap::Error);
        FuturesCanceled(futures::channel::oneshot::Canceled);
        Reqwest(reqwest::Error);
        SerdeJson(serde_json::Error);
        Tokio(tokio::task::JoinError);
        Url(url::ParseError);
    }

    errors {
        CefError(return_value: ::std::os::raw::c_int) {
            description("cef error")
            display("cef error {}", return_value)
        }
    }
}

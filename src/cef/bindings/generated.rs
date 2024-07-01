#![allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    deref_nullptr,
    dead_code,
    clippy::pub_underscore_fields,
    clippy::transmute_ptr_to_ptr,
    clippy::must_use_candidate
)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

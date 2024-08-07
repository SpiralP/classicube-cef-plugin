#![allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    deref_nullptr,
    dead_code,
    clippy::must_use_candidate,
    clippy::pub_underscore_fields,
    clippy::struct_field_names,
    clippy::transmute_ptr_to_ptr
)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

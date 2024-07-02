#[path = "generated.rs"]
mod generated;

pub use self::generated::{
    Callbacks, CefInitializePaths, FFIRustV8Response, FFIRustV8Value, FFIRustV8ValueTag, RustRect,
    RustRefApp, RustRefBrowser, RustRefClient, RustRefString, RustSchemeReturn,
};
use proc_todo::test_mock_fn;
use std::os::raw::c_int;

test_mock_fn!(cef_interface_add_ref_app, 1, RustRefApp);
test_mock_fn!(cef_interface_add_ref_browser, 1, RustRefBrowser);
test_mock_fn!(cef_interface_add_ref_client, 1, RustRefClient);
test_mock_fn!(cef_interface_browser_close, 1, c_int);
test_mock_fn!(cef_interface_browser_eval_javascript_on_frame, 4, c_int);
test_mock_fn!(cef_interface_browser_eval_javascript, 3, c_int);
test_mock_fn!(cef_interface_browser_execute_javascript_on_frame, 3, c_int);
test_mock_fn!(cef_interface_browser_execute_javascript, 2, c_int);
test_mock_fn!(cef_interface_browser_get_identifier, 1, c_int);
test_mock_fn!(cef_interface_browser_load_url, 2, c_int);
test_mock_fn!(cef_interface_browser_open_dev_tools, 1, c_int);
test_mock_fn!(cef_interface_browser_reload, 1, c_int);
test_mock_fn!(cef_interface_browser_send_click, 3, c_int);
test_mock_fn!(cef_interface_browser_send_text, 2, c_int);
test_mock_fn!(cef_interface_browser_set_audio_muted, 2, c_int);
test_mock_fn!(cef_interface_browser_was_resized, 1, c_int);
test_mock_fn!(cef_interface_create_app, 1, RustRefApp);
test_mock_fn!(cef_interface_create_browser, 5, c_int);
test_mock_fn!(cef_interface_delete_ref_string, 1, c_int);
test_mock_fn!(cef_interface_execute_process, 2, c_int);
test_mock_fn!(cef_interface_initialize, 2, c_int);
test_mock_fn!(cef_interface_new_ref_string, 2, RustRefString);
test_mock_fn!(cef_interface_release_ref_app, 1, c_int);
test_mock_fn!(cef_interface_release_ref_browser, 1, c_int);
test_mock_fn!(cef_interface_release_ref_client, 1, c_int);
test_mock_fn!(cef_interface_shutdown, 0, c_int);
test_mock_fn!(cef_interface_step, 0, c_int);

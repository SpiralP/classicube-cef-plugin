#![feature(core_intrinsics)]
mod cef;
mod helpers;

use classicube_sys::*;
use std::{os::raw::c_int, ptr};

unsafe extern "C" fn init() {
    color_backtrace::install_with_settings(
        color_backtrace::Settings::new().verbosity(color_backtrace::Verbosity::Full),
    );

    cef::initialize();
}

extern "C" fn free() {
    println!("Free");

    cef::shutdown();
}

unsafe extern "C" fn on_new_map_loaded() {
    println!("OnNewMapLoaded");
}

#[no_mangle]
pub static Plugin_ApiVersion: c_int = 1;

#[no_mangle]
pub static mut Plugin_Component: IGameComponent = IGameComponent {
    // Called when the game is being loaded.
    Init: Some(init),
    // Called when the component is being freed. (e.g. due to game being closed)
    Free: Some(free),
    // Called to reset the component's state. (e.g. reconnecting to server)
    Reset: None,
    // Called to update the component's state when the user begins loading a new map.
    OnNewMap: None,
    // Called to update the component's state when the user has finished loading a new map.
    OnNewMapLoaded: Some(on_new_map_loaded),
    // Next component in linked list of components.
    next: ptr::null_mut(),
};

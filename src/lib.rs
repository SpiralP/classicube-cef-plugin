#![feature(const_fn)]

mod async_manager;
mod cef;
mod chat;
mod entity_manager;
mod error;
mod helpers;
mod logger;
mod options;
mod player;
mod plugin;
mod search;

use self::plugin::Plugin;
use classicube_helpers::{time, time_silent};
use classicube_sys::*;
use log::debug;
use std::{os::raw::c_int, ptr};

extern "C" fn init() {
    color_backtrace::install_with_settings(
        color_backtrace::Settings::new()
            .verbosity(color_backtrace::Verbosity::Full)
            .message("CEF crashed!!"),
    );

    logger::initialize(true, false);

    time!("Plugin::initialize()", 5000, {
        Plugin::initialize();
    });
}

extern "C" fn free() {
    debug!("Free");

    time!("Plugin::shutdown()", 1000, {
        Plugin::shutdown();
    });
}

extern "C" fn reset() {
    time!("Plugin::reset()", 1000, {
        Plugin::reset();
    });
}

extern "C" fn on_new_map() {
    time!("Plugin::on_new_map()", 1000, {
        Plugin::on_new_map();
    });
}

extern "C" fn on_new_map_loaded() {
    time!("Plugin::on_new_map_loaded()", 1000, {
        Plugin::on_new_map_loaded();
    });
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
    Reset: Some(reset),
    // Called to update the component's state when the user begins loading a new map.
    OnNewMap: Some(on_new_map),
    // Called to update the component's state when the user has finished loading a new map.
    OnNewMapLoaded: Some(on_new_map_loaded),
    // Next component in linked list of components.
    next: ptr::null_mut(),
};

#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    clippy::unused_self
)]

mod api;
mod cef;
mod chat;
mod entity_manager;
mod error;
mod helpers;
mod logger;
mod options;
mod panic;
mod player;
mod plugin;

use std::{os::raw::c_int, ptr};

use classicube_helpers::{test_noop_fn, test_noop_static, time, time_silent};
use classicube_sys::IGameComponent;
use tracing::debug;

pub use self::cef::cef_interface_execute_process;
use self::plugin::Plugin;

extern "C" fn init() {
    panic::install_hook();

    logger::initialize(true, Some(module_path!()), false);

    tracing::debug_span!("init").in_scope(|| {
        debug!(
            "Init {}",
            concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"))
        );

        time!("Plugin::initialize()", 5000, {
            Plugin::initialize();
        });
    });
}

extern "C" fn free() {
    tracing::debug_span!("free").in_scope(|| {
        debug!("Free");

        time!("Plugin::shutdown()", 1000, {
            Plugin::shutdown();
        });
    });

    logger::free();
}

#[tracing::instrument]
extern "C" fn reset() {
    time!("Plugin::reset()", 1000, {
        Plugin::reset();
    });
}

#[tracing::instrument]
extern "C" fn on_new_map() {
    time!("Plugin::on_new_map()", 1000, {
        Plugin::on_new_map();
    });
}

#[tracing::instrument]
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

test_noop_static!(Entities);
test_noop_static!(Camera);

test_noop_fn!(Entity_SetModel);
test_noop_fn!(Options_Get);
test_noop_fn!(Options_Set);
test_noop_fn!(ScheduledTask_Add);
test_noop_fn!(Chat_Add);
test_noop_fn!(Chat_AddOf);
test_noop_fn!(Chat_Send);
test_noop_fn!(Gfx_CreateTexture);
test_noop_fn!(Gfx_DeleteTexture);

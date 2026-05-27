pub mod clients;
pub mod encoding;
pub mod global_control;
pub mod whispers;

use std::{
    cell::{Cell, RefCell},
    ptr, slice,
};

use classicube_helpers::async_manager;
use classicube_sys::{
    MsgType_MSG_TYPE_NORMAL, Net_Handler, OPCODE__OPCODE_MESSAGE, Protocol, Server,
    UNSAFE_GetString,
};
use futures::channel::oneshot;
use tracing::debug;

pub use self::global_control::CURRENT_MAP_THEME;
use super::SIMULATING;
use crate::plugin::is_plugin_active;

thread_local!(
    static SHOULD_BLOCK: Cell<bool> = const { Cell::new(false) };
);

thread_local!(
    static WAITING_FOR_MESSAGE: RefCell<Vec<oneshot::Sender<String>>> = RefCell::default();
);

thread_local!(
    static OLD_MESSAGE_HANDLER: RefCell<Net_Handler> = RefCell::default();
);

extern "C" fn message_handler(data: *mut u8) {
    // If the plugin has been shut down but our hook is still reachable via
    // another plugin's chain, skip our processing — handle_chat_message
    // would touch async_manager and WAITING_FOR_MESSAGE which shutdown()
    // tears down. Fall straight through to the saved next handler.
    if is_plugin_active() {
        use classicube_sys::MsgType;

        let bytes = unsafe { slice::from_raw_parts(data, 65) };
        let message_type = bytes[0] as MsgType;
        let text = unsafe { UNSAFE_GetString(&bytes[1..]) }.to_string();

        if message_type == MsgType_MSG_TYPE_NORMAL && handle_chat_message(&text) {
            return;
        }
    }

    OLD_MESSAGE_HANDLER.with(|cell| {
        if let Some(f) = *cell.borrow() {
            unsafe {
                f(data);
            }
        }
    });
}

fn install_message_handler() {
    // Idempotent: if our handler is already installed (e.g. reset() called
    // a second time without an intervening shutdown), don't re-save it as
    // the "old" handler — that would cause `message_handler` to recurse
    // into itself.
    let current = unsafe { Protocol.Handlers[OPCODE__OPCODE_MESSAGE as usize] };
    if is_our_handler(current) {
        return;
    }

    // We previously installed ourselves and another plugin has since stacked
    // its own hook on top. Re-pushing to the top would set
    //   slot = us, OLD_MESSAGE_HANDLER = other_plugin
    // while other_plugin's saved "old" still points at us — an infinite
    // recursion through our own handler on every chat message. Leave the
    // chain alone; we're still reachable via the existing chain.
    if OLD_MESSAGE_HANDLER.with(|cell| cell.borrow().is_some()) {
        return;
    }

    unsafe {
        Protocol.Handlers[OPCODE__OPCODE_MESSAGE as usize] = Some(message_handler);
    }

    OLD_MESSAGE_HANDLER.with(|cell| {
        let option = &mut *cell.borrow_mut();
        *option = current;
    });
}

fn uninstall_message_handler() {
    // If another plugin stacked on top of us, our message_handler is still
    // reachable via their chain — keep OLD_MESSAGE_HANDLER populated so the
    // fall-through call still works instead of dropping the message.
    let current = unsafe { Protocol.Handlers[OPCODE__OPCODE_MESSAGE as usize] };
    if is_our_handler(current) {
        let restored = OLD_MESSAGE_HANDLER.with(|cell| cell.borrow_mut().take());
        unsafe {
            Protocol.Handlers[OPCODE__OPCODE_MESSAGE as usize] = restored;
        }
    }
}

fn is_our_handler(handler: Net_Handler) -> bool {
    handler.is_some_and(|h| ptr::fn_addr_eq(h, message_handler as unsafe extern "C" fn(*mut u8)))
}

pub fn initialize() {
    debug!("initialize hidden_communication");

    if unsafe { Server.IsSinglePlayer } != 0 {
        return;
    }

    whispers::start_listening();
    global_control::start_listening();

    install_message_handler();
}

pub fn reset() {
    debug!("reset hidden_communication");

    if unsafe { Server.IsSinglePlayer } != 0 {
        return;
    }

    install_message_handler();
}

pub fn on_new_map() {
    if unsafe { Server.IsSinglePlayer } == 0 {
        clients::stop_query();
        global_control::on_new_map();
    }
}

pub fn on_new_map_loaded() {
    if unsafe { Server.IsSinglePlayer } == 0 {
        clients::query();
        global_control::on_new_map_loaded();
    }
}

pub fn shutdown() {
    debug!("shutdown hidden_communication");

    global_control::stop_listening();
    whispers::stop_listening();
    clients::shutdown();

    uninstall_message_handler();

    SHOULD_BLOCK.set(false);
    WAITING_FOR_MESSAGE.with(|cell| cell.borrow_mut().clear());
}

pub async fn wait_for_message() -> String {
    let (sender, receiver) = oneshot::channel();

    WAITING_FOR_MESSAGE.with(|cell| {
        let waiting = &mut cell.borrow_mut();
        waiting.push(sender);
    });

    // unwrap ok because we don't drop the sender before send()
    receiver.await.unwrap()
}

#[must_use]
pub fn handle_chat_message(message: &str) -> bool {
    // don't recurse from Chat::print()
    if SIMULATING.get() {
        return false;
    }

    // resolve the awaits here so that our very next step() will trigger that code section
    let waiting: Vec<_> = WAITING_FOR_MESSAGE.with(|cell| {
        let waiting = &mut cell.borrow_mut();
        waiting.drain(..).collect()
    });

    for sender in waiting {
        let _ignore_error = sender.send(message.to_string());
    }

    async_manager::step();

    // check SHOULD_BLOCK to see if any futures said to block
    let should_block = SHOULD_BLOCK.get();
    SHOULD_BLOCK.set(false);

    should_block
}

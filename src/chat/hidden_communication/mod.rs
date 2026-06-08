pub mod clients;
pub mod encoding;
pub mod global_control;
pub mod whispers;

use std::cell::{Cell, RefCell};

use classicube_helpers::{async_manager, chat::ProtocolMessageHook};
use classicube_sys::Server;
use futures::channel::oneshot;
use tracing::debug;

pub use self::global_control::CURRENT_MAP_THEME;
use super::SIMULATING;

thread_local!(
    static SHOULD_BLOCK: Cell<bool> = const { Cell::new(false) };
);

thread_local!(
    static WAITING_FOR_MESSAGE: RefCell<Vec<oneshot::Sender<String>>> = RefCell::default();
);

thread_local!(
    static HOOK: RefCell<Option<ProtocolMessageHook>> = const { RefCell::new(None) };
);

pub fn initialize() {
    debug!("initialize hidden_communication");

    if unsafe { Server.IsSinglePlayer } != 0 {
        return;
    }

    whispers::start_listening();
    global_control::start_listening();

    HOOK.with_borrow_mut(|hook| {
        *hook = ProtocolMessageHook::install(handle_chat_message);
    });
}

pub fn reset() {
    debug!("reset hidden_communication");

    // reinstall() is a no-op in singleplayer and HOOK is None there anyway.
    HOOK.with_borrow(|hook| {
        if let Some(hook) = hook {
            hook.reinstall();
        }
    });
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

    // Drop uninstalls (when on top) and always clears the callback, so a
    // trampoline still reachable via a foreign plugin's chain just forwards.
    HOOK.with_borrow_mut(|hook| *hook = None);

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

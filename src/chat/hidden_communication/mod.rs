pub mod clients;
pub mod encoding;
pub mod global_control;
pub mod whispers;

pub use self::{encoding::LightEntity, global_control::CURRENT_MAP_THEME};
use super::SIMULATING;
use crate::async_manager;
use classicube_helpers::CellGetSet;
use classicube_sys::{MsgType_MSG_TYPE_NORMAL, Server};
#[cfg(feature = "detour")]
use detour::static_detour;
use futures::channel::oneshot;
use log::debug;
use std::{
    cell::{Cell, RefCell},
    os::raw::c_int,
};

#[cfg(feature = "detour")]
static_detour! {
    static DETOUR: unsafe extern "C" fn(*const classicube_sys:: String, c_int);
}

thread_local!(
    static SHOULD_BLOCK: Cell<bool> = Cell::new(false);
);

thread_local!(
    static WAITING_FOR_MESSAGE: RefCell<Vec<oneshot::Sender<String>>> = Default::default();
);

#[cfg(feature = "detour")]
fn chat_add_hook(text: *const classicube_sys::String, message_type: c_int) {
    if message_type == MsgType_MSG_TYPE_NORMAL as c_int
        && handle_chat_message(unsafe { (*text).to_string() })
    {
        return;
    }

    unsafe { DETOUR.call(text, message_type) }
}

pub fn initialize() {
    debug!("initialize hidden_communication");

    if unsafe { Server.IsSinglePlayer } != 0 {
        return;
    }

    #[cfg(feature = "detour")]
    unsafe {
        use classicube_sys::Chat_AddOf;

        DETOUR.initialize(Chat_AddOf, chat_add_hook).unwrap();
        DETOUR.enable().unwrap();
    }

    #[cfg(not(feature = "detour"))]
    {
        use classicube_helpers::events::chat::{ChatReceivedEvent, ChatReceivedEventHandler};

        thread_local!(
            static CHAT_RECEIVED: ChatReceivedEventHandler = {
                let mut h = ChatReceivedEventHandler::new();

                h.on(
                    |ChatReceivedEvent {
                         message,
                         message_type,
                     }| {
                        if *message_type == MsgType_MSG_TYPE_NORMAL as c_int
                            && handle_chat_message(message.to_string())
                        {
                            return;
                        }
                    },
                );

                h
            };
        );

        CHAT_RECEIVED.with(|_| {});
    }

    whispers::start_listening();
    global_control::start_listening();
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

    #[cfg(feature = "detour")]
    unsafe {
        let _ignore_error = DETOUR.disable();
    }
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
fn handle_chat_message(message: String) -> bool {
    // don't recurse from Chat::send()
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

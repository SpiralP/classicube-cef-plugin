mod chat_command;

pub use self::chat_command::{command_callback, CefChatCommand};
use crate::async_manager::AsyncManager;
use classicube_helpers::{
    entities::ENTITY_SELF_ID,
    events::chat::{ChatReceivedEvent, ChatReceivedEventHandler},
    tab_list::TabList,
};
use classicube_sys::{Chat_Add, Entities, MsgType, MsgType_MSG_TYPE_NORMAL, OwnedString, Server};
use futures::{future::RemoteHandle, prelude::*};
use log::info;
use std::cell::{Cell, RefCell};

thread_local!(
    static LAST_CHAT: RefCell<Option<String>> = RefCell::new(None);
);

thread_local!(
    static SIMULATING: Cell<bool> = Cell::new(false);
);

thread_local!(
    static TAB_LIST: RefCell<Option<TabList>> = RefCell::new(None);
);

thread_local!(
    static FUTURE_HANDLE: Cell<Option<RemoteHandle<()>>> = Cell::new(None);
);

pub struct Chat {
    chat_command: CefChatCommand,
    chat_received: ChatReceivedEventHandler,
}

impl Chat {
    pub fn new() -> Self {
        Self {
            chat_command: CefChatCommand::new(),
            chat_received: ChatReceivedEventHandler::new(),
        }
    }

    pub fn initialize(&mut self) {
        self.chat_command.initialize();

        self.chat_received.on(
            |ChatReceivedEvent {
                 message,
                 message_type,
             }| {
                handle_chat_received(message.to_string(), *message_type);
            },
        );

        TAB_LIST.with(|cell| {
            let tab_list = &mut *cell.borrow_mut();
            *tab_list = Some(TabList::new());
        });
    }

    pub fn shutdown(&mut self) {
        self.chat_command.shutdown();
        TAB_LIST.with(|cell| {
            let tab_list = &mut *cell.borrow_mut();
            tab_list.take();
        });
    }

    pub fn print<S: Into<String>>(s: S) {
        let s = s.into();
        info!("{}", s);

        let owned_string = OwnedString::new(s);

        SIMULATING.with(|a| a.set(true));
        unsafe {
            Chat_Add(owned_string.as_cc_string());
        }
        SIMULATING.with(|a| a.set(false));
    }
}

fn handle_chat_received(message: String, message_type: MsgType) {
    if SIMULATING.with(|a| a.get()) {
        return;
    }
    if message_type != MsgType_MSG_TYPE_NORMAL {
        return;
    }

    if let Some((id, _name, message)) = find_player_from_message(message) {
        // let name: String = remove_color(name).trim().to_string();

        // don't remove colors because & might be part of url!
        // let message: String = remove_color(message).trim().to_string();

        let mut split = message
            .split(' ')
            .map(|a| a.to_string())
            .collect::<Vec<String>>();

        if split
            .get(0)
            .map(|first| remove_color(first).trim() == "cef")
            .unwrap_or(false)
        {
            // remove "cef"
            split.remove(0);

            FUTURE_HANDLE.with(|cell| {
                let (remote, remote_handle) = async move {
                    tokio::time::delay_for(std::time::Duration::from_millis(256)).await;

                    AsyncManager::run_on_main_thread(async move {
                        // TODO use the higher level Entity from helpers
                        let player = unsafe { &*Entities.List[id as usize] };

                        if let Err(e) = command_callback(player, split) {
                            if id == ENTITY_SELF_ID {
                                Chat::print(format!("cef command error: {}", e));
                            }
                        }
                    })
                    .await;
                }
                .remote_handle();

                cell.set(Some(remote_handle));

                AsyncManager::spawn(remote);
            });
        }
    }
}

fn find_player_from_message(mut full_msg: String) -> Option<(u8, String, String)> {
    if unsafe { Server.IsSinglePlayer } != 0 {
        // in singleplayer there is no tab list, even self id infos are null

        return Some((ENTITY_SELF_ID, String::new(), full_msg));
    }

    LAST_CHAT.with(|cell| {
        let mut last_chat = cell.borrow_mut();

        if !full_msg.starts_with("> &f") {
            *last_chat = Some(full_msg.clone());
        } else if let Some(chat_last) = &*last_chat {
            FUTURE_HANDLE.with(|cell| {
                cell.set(None);
            });

            // we're a continue message
            full_msg = full_msg.split_off(4); // skip "> &f"

            // most likely there's a space
            // the server trims the first line :(
            // TODO try both messages? with and without the space?
            full_msg = format!("{}{}", chat_last, full_msg);
            *last_chat = Some(full_msg.clone());
        }

        // &]SpiralP: &faaa
        // let full_msg = full_msg.into();

        // nickname_resolver_handle_message(full_msg.to_string());

        // find colon from the left
        if let Some(pos) = full_msg.find(": ") {
            // &]SpiralP
            let left = &full_msg[..pos]; // left without colon
                                         // &faaa
            let right = &full_msg[(pos + 2)..]; // right without colon

            // TODO title is [ ] before nick, team is < > before nick, also there are rank
            // symbols? &f┬ &f♂&6 Goodly: &fhi

            let full_nick = left.to_string();
            let said_text = right.to_string();

            // lookup entity id from nick_name by using TabList
            TAB_LIST.with(|cell| {
                let tab_list = &*cell.borrow();
                tab_list
                    .as_ref()
                    .unwrap()
                    .find_entry_by_nick_name(full_nick)
                    .map(|entry| (entry.get_id(), entry.get_real_name().unwrap(), said_text))
            })
        } else {
            None
        }
    })
}

fn remove_color<T: AsRef<str>>(text: T) -> String {
    let mut found_ampersand = false;

    text.as_ref()
        .chars()
        .filter(|&c| {
            if c == '&' {
                // we remove all amps but they're kept in chat if repeated
                found_ampersand = true;
                false
            } else if found_ampersand {
                found_ampersand = false;
                false
            } else {
                true
            }
        })
        .collect()
}

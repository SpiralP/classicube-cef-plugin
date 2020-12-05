mod chat_command;
pub mod commands;
pub mod helpers;
pub mod hidden_communication;

pub use self::chat_command::CefChatCommand;
use crate::{async_manager, chat::helpers::is_continuation_message, error::*};
use classicube_helpers::{
    entities::{Entities, ENTITY_SELF_ID},
    events::chat::{ChatReceivedEvent, ChatReceivedEventHandler},
    tab_list::{remove_color, TabList},
    CellGetSet,
};
use classicube_sys::{Chat_Send, MsgType, MsgType_MSG_TYPE_NORMAL, OwnedString, Server, Vec3};
use deunicode::deunicode;
use futures::{future::RemoteHandle, prelude::*};
use log::*;
use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

thread_local!(
    static LAST_CHAT: RefCell<Option<String>> = RefCell::new(None);
);

thread_local!(
    static SIMULATING: Cell<bool> = Cell::new(false);
);

thread_local!(
    static TAB_LIST: RefCell<Option<TabList>> = RefCell::new(None);
);

// TODO make this not public :p
thread_local!(
    pub static ENTITIES: RefCell<Option<Entities>> = RefCell::new(None);
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
        debug!("initialize chat");

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

        ENTITIES.with(|cell| {
            let tab_list = &mut *cell.borrow_mut();
            *tab_list = Some(Entities::new());
        });

        hidden_communication::initialize();

        commands::initialize();
    }

    pub fn on_new_map(&mut self) {
        hidden_communication::on_new_map();
    }

    pub fn on_new_map_loaded(&mut self) {
        debug!("on_new_map_loaded chat");

        hidden_communication::on_new_map_loaded();

        #[cfg(debug_assertions)]
        unsafe {
            use classicube_helpers::OptionWithInner;
            use classicube_sys::ENTITIES_SELF_ID;

            if Server.IsSinglePlayer == 0 {
                let me = ENTITIES
                    .with_inner(|entities| {
                        entities
                            .get(ENTITIES_SELF_ID as _)
                            .map(|me| me.get_display_name() == "SpiralP2")
                    })
                    .flatten()
                    .unwrap_or(false);

                if me {
                    async_manager::spawn_local_on_main_thread(async {
                        async_manager::sleep(Duration::from_millis(200)).await;
                        let s = classicube_sys::OwnedString::new("/tp SpiralP+");
                        classicube_sys::Chat_Send(s.as_cc_string(), 0);
                    });
                }
            } else {
                async_manager::spawn_local_on_main_thread(async {
                    async_manager::sleep(Duration::from_millis(2000)).await;

                    async fn run(args: &[&str]) {
                        let player_snapshot =
                            PlayerSnapshot::from_entity_id(classicube_sys::ENTITIES_SELF_ID as _)
                                .unwrap();
                        crate::chat::commands::run(
                            player_snapshot,
                            args.iter().map(ToString::to_string).collect(),
                            true,
                            true,
                        )
                        .await
                        .expect("don't worry about this error");
                    }

                    run(&["create", "-n", "ag", "-s", "youtu.be/keF7n1eVKzE?t=273"]).await;
                    run(&["here", "-n", "ag"]).await;
                    run(&["volume", "-n", "ag", "-p", "10.0"]).await;
                    // run(&["speed", "-n", "ag", "1.1"]).await;

                    // async_manager::sleep(Duration::from_millis(1000)).await;

                    // let browser = crate::entity_manager::EntityManager::with_all_entities(|entities| {
                    //     entities
                    //         .values()
                    //         .next()
                    //         .and_then(|e| e.browser.as_ref().cloned())
                    // });

                    // if let Some(browser) = browser {
                    //     async_manager::spawn_local_on_main_thread(async move {
                    //         debug!("eval");
                    //         debug!(
                    //             "{:#?}",
                    //             browser
                    //                 .eval_javascript("'the world is not anymore the way it used to be'")
                    //                 .await
                    //         );
                    //     });
                    // }
                });
            }
        }
    }

    pub fn shutdown(&mut self) {
        hidden_communication::shutdown();

        ENTITIES.with(|cell| {
            let entities = &mut *cell.borrow_mut();
            entities.take();
        });

        TAB_LIST.with(|cell| {
            let tab_list = &mut *cell.borrow_mut();
            tab_list.take();
        });

        self.chat_command.shutdown();
    }

    pub fn reset(&mut self) {
        hidden_communication::reset();
    }

    pub fn print<S: Into<String>>(s: S) {
        let s = s.into();
        info!("{}", s);

        #[cfg(not(test))]
        {
            use classicube_sys::Chat_Add;

            let mut s = deunicode(&s);

            if s.len() > 255 {
                s.truncate(255);
            }

            SIMULATING.set(true);

            let owned_string = OwnedString::new(s);

            unsafe {
                Chat_Add(owned_string.as_cc_string());
            }

            SIMULATING.set(false);
        }
    }

    pub fn send<S: Into<String>>(s: S) {
        let s = s.into();
        info!("{}", s);
        let s = deunicode(&s);

        let owned_string = OwnedString::new(s);

        unsafe {
            Chat_Send(owned_string.as_cc_string(), 0);
        }
    }
}

#[test]
fn test_unicode() {
    let input = "Ｌｕｉｇｉ，　ｂｒｏｔｈｅｒ．．．[ヒップホップ MIX]";
    println!("{:?}", deunicode(input));

    assert_eq!(deunicode(input), "Luigi, brother...[hitupuhotupu MIX]");
}

fn handle_chat_received(message: String, message_type: MsgType) {
    if SIMULATING.get() {
        return;
    }
    if message_type != MsgType_MSG_TYPE_NORMAL {
        return;
    }

    // TODO if it wasn't a > message, fire the command of the last
    if let Some((mut id, real_name, message)) = find_player_from_message(message.clone()) {
        // debug!("{:?} {:?} {:?}", id, real_name, message);
        // let name: String = remove_color(name).trim().to_string();

        // don't remove colors because & might be part of url!
        // let message: String = remove_color(message).trim().to_string();

        let mut split = message
            .split(' ')
            .map(|a| a.to_string())
            .collect::<Vec<String>>();

        // if you put a leading space " cef"
        // you get ["&f", "cef"]

        if split
            .get(0)
            .map(|first| remove_color(first) == "cef")
            .unwrap_or(false)
        {
            // remove "cef"
            split.remove(0);

            let mut opt = PlayerSnapshot::from_entity_id(id);

            if opt.is_none() {
                // try searching all entitys' display-name
                warn!(
                    "player_snapshot lookup failed for id {:?}, trying direct entity DisplayName \
                     lookup",
                    id
                );

                let (id2, opt2) = ENTITIES
                    .with(|cell| {
                        let entities = &*cell.borrow();
                        let entities = entities.as_ref().unwrap();
                        for (new_id, e) in entities.get_all() {
                            if real_name == remove_color(e.get_display_name()) {
                                return Ok::<_, Error>((
                                    *new_id,
                                    PlayerSnapshot::from_entity_id(*new_id),
                                ));
                            }
                        }

                        Ok::<_, Error>((id, None))
                    })
                    .unwrap();

                id = id2;
                opt = opt2;
            }

            if let Some(player_snapshot) = opt {
                FUTURE_HANDLE.with(|cell| {
                    let (remote, remote_handle) = async move {
                        if unsafe { Server.IsSinglePlayer } == 0 {
                            async_manager::sleep(Duration::from_millis(256)).await;
                        }

                        let is_self = id == ENTITY_SELF_ID;

                        if let Err(e) = commands::run(player_snapshot, split, is_self, false).await
                        {
                            if is_self {
                                warn!("chat command error: {:#?}", e);
                                Chat::print(format!(
                                    "{}cef command error: {}{}",
                                    classicube_helpers::color::RED,
                                    classicube_helpers::color::WHITE,
                                    e
                                ));
                            }
                        }
                    }
                    .remote_handle();

                    cell.set(Some(remote_handle));

                    async_manager::spawn_local_on_main_thread(remote);
                });
            } else {
                warn!("player_snapshot all lookups failed for id {:?}", id);
            }
        }
    } else if (message.contains(": ") || message.contains("> "))
        && !message.starts_with("&5Discord: &f[")
    {
        warn!("couldn't match player for {:?}", message);
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    pub id: u8,
    pub eye_position: Vec3,
    pub Position: Vec3,
    pub Pitch: f32,
    pub Yaw: f32,
    pub RotX: f32,
    pub RotY: f32,
    pub RotZ: f32,
}

impl PlayerSnapshot {
    pub fn from_entity_id(id: u8) -> Option<Self> {
        ENTITIES.with(|cell| {
            let entities = &*cell.borrow();
            let entities = entities.as_ref().unwrap();
            entities.get(id).map(|entity| {
                let position = entity.get_position();
                let eye_position = entity.get_eye_position();
                let head = entity.get_head();
                let rot = entity.get_rot();
                Self {
                    id,
                    Position: position,
                    eye_position,
                    Pitch: head[0],
                    Yaw: head[1],
                    RotX: rot[0],
                    RotY: rot[1],
                    RotZ: rot[2],
                }
            })
        })
    }
}

fn find_player_from_message(mut full_msg: String) -> Option<(u8, String, String)> {
    if unsafe { Server.IsSinglePlayer } != 0 {
        // in singleplayer there is no tab list, even self id infos are null

        return Some((ENTITY_SELF_ID, String::new(), full_msg));
    }

    LAST_CHAT.with(|cell| {
        let mut last_chat = cell.borrow_mut();

        if let Some(continuation) = is_continuation_message(&full_msg) {
            if let Some(chat_last) = &*last_chat {
                // we're a continue message

                // kill old thread
                FUTURE_HANDLE.with(|cell| {
                    cell.set(None);
                });

                full_msg = continuation.to_string();

                // most likely there's a space
                // the server trims the first line :(
                full_msg = format!("{} {}", chat_last, full_msg);
                *last_chat = Some(full_msg.clone());
            }
        } else {
            // normal message start
            *last_chat = Some(full_msg.clone());
        }

        // &]SpiralP: &faaa
        // let full_msg = full_msg.into();

        // nickname_resolver_handle_message(full_msg.to_string());

        // find colon from the left
        let opt = full_msg
            .find(": ")
            .and_then(|pos| if pos > 4 { Some(pos) } else { None })
            .or_else(|| full_msg.find("> "))
            .and_then(|pos| if pos > 4 { Some(pos) } else { None });

        if let Some(pos) = opt {
            // > &fasdfasdf

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
                    .find_entry_by_nick_name(&full_nick)
                    .map(|entry| (entry.get_id(), entry.get_real_name().unwrap(), said_text))
            })
        } else {
            None
        }
    })
}

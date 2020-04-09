mod cef_paint;
mod chat_command;
mod render_model;

use self::{
    cef_paint::cef_paint_callback, chat_command::c_chat_command_callback,
    render_model::local_player_render_model_hook,
};
use crate::{interface::*, owned_entity::OwnedEntity, owned_model::*};
use classicube_helpers::{detour::*, tick::*};
use classicube_sys::*;
use std::{
    cell::RefCell,
    os::raw::{c_double, c_float},
    pin::Pin,
};

// Some means we are initialized
thread_local!(
    pub static CEF: RefCell<Option<Cef>> = RefCell::new(None);
);

fn print<S: Into<Vec<u8>>>(s: S) {
    let owned_string = OwnedString::new(s);
    unsafe {
        Chat_Add(owned_string.as_cc_string());
    }
}

pub fn initialize() {
    print("cef initialize");

    CEF.with(|option| {
        debug_assert!(option.borrow().is_none());

        *option.borrow_mut() = Some(Cef::new());
    });

    CEF.with(|option| {
        if let Some(cef) = &mut *option.borrow_mut() {
            cef.initialize();
        }
    });
}

pub fn shutdown() {
    print("cef shutdown");

    CEF.with(|option| {
        let mut cef = option.borrow_mut().take().unwrap();
        cef.shutdown();
    });
}

pub struct Cef {
    pub model: Option<Pin<Box<OwnedModel>>>,
    pub entity: Option<Pin<Box<OwnedEntity>>>,

    pub local_player_render_model_detour:
        GenericDetour<unsafe extern "C" fn(*mut Entity, c_double, c_float)>,
    tick_handler: TickEventHandler,
    initialized: bool,
    _chat_command: OwnedChatCommand,
}

impl Cef {
    pub fn new() -> Self {
        let local_player_render_model_detour = unsafe {
            let me = &*Entities.List[ENTITIES_SELF_ID as usize];
            let v_table = &*me.VTABLE;
            let target = v_table.RenderModel.unwrap();
            GenericDetour::new(
                target,
                local_player_render_model_hook
                    as unsafe extern "C" fn(*mut Entity, c_double, c_float),
            )
            .unwrap()
        };

        let _chat_command =
            OwnedChatCommand::new("Cef", c_chat_command_callback, false, vec!["hello??"]);

        Self {
            model: None,
            entity: None,
            local_player_render_model_detour,
            tick_handler: TickEventHandler::new(),
            initialized: false,
            _chat_command,
        }
    }

    pub fn initialize(&mut self) {
        unsafe {
            self.model = Some(OwnedModel::register("cef", "cef"));
            self.entity = Some(OwnedEntity::register());

            self.local_player_render_model_detour.enable().unwrap();
        }

        unsafe {
            assert_eq!(cef_init(Some(cef_paint_callback)), 0);
        }

        self.tick_handler.on(|_task| {
            //
            unsafe {
                assert_eq!(cef_step(), 0);
            }
        });

        self.initialized = true;
    }

    pub fn shutdown(&mut self) {
        if self.initialized {
            self.model.take();

            unsafe {
                assert_eq!(cef_free(), 0);
            }
            self.initialized = false;
        }
    }
}

impl Drop for Cef {
    fn drop(&mut self) {
        self.shutdown();
    }
}

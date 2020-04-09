mod cef_paint;
mod chat_command;
mod render_model;

use self::{
    cef_paint::cef_paint_callback, chat_command::c_chat_command_callback,
    render_model::local_player_render_model_hook,
};
use crate::{bindings::*, helpers::*, owned_entity::OwnedEntity, owned_model::*};
use classicube_helpers::{detour::*, tick::*};
use classicube_sys::{Entities, Entity, OwnedChatCommand, ENTITIES_SELF_ID};
use std::{
    cell::RefCell,
    ffi::CString,
    os::raw::{c_double, c_float},
    pin::Pin,
};

// Some means we are initialized
thread_local!(
    pub static CEF: RefCell<Option<Cef>> = RefCell::new(None);
);

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
    chat_command: Pin<Box<OwnedChatCommand>>,
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

        let chat_command =
            OwnedChatCommand::new("Cef", c_chat_command_callback, false, vec!["cef"]);

        Self {
            model: None,
            entity: None,
            local_player_render_model_detour,
            tick_handler: TickEventHandler::new(),
            initialized: false,
            chat_command,
        }
    }

    pub fn load(&mut self, url: String) {
        let c_str = CString::new(url).unwrap();

        unsafe {
            assert_eq!(cef_load(c_str.as_ptr()), 0);
        }
    }

    pub fn initialize(&mut self) {
        self.chat_command.as_mut().register();

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

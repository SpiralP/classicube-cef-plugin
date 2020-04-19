use crate::{async_manager::AsyncManager, cef, chat::Chat, entity_manager::EntityManager, players};
use classicube_helpers::with_inner::WithInner;
use classicube_sys::{Server, String_AppendConst};
use log::debug;
use std::{cell::RefCell, ffi::CString};

thread_local!(
    static PLUGIN: RefCell<Option<Plugin>> = RefCell::new(None);
);

pub fn initialize() {
    PLUGIN.with(|cell| {
        assert!(cell.borrow().is_none());

        *cell.borrow_mut() = Some(Plugin::new());
    });
}

pub fn on_first_context_created() {
    PLUGIN
        .with_inner_mut(|plugin| {
            debug!("plugin initialize");
            plugin.initialize();
        })
        .unwrap();
}

pub fn shutdown() {
    PLUGIN.with_inner_mut(|plugin| {
        debug!("plugin shutdown");
        plugin.shutdown();
    });

    PLUGIN.with(|cell| {
        cell.borrow_mut().take().unwrap();
    });
}

pub struct Plugin {
    async_manager: AsyncManager,
    chat: Chat,
    entity_manager: EntityManager,
}

pub const APP_NAME: &str = concat!("cef", env!("CARGO_PKG_VERSION"));

impl Plugin {
    /// Called once on our plugin's `Init`
    pub fn new() -> Self {
        Chat::print(format!("Cef v{} initializing", env!("CARGO_PKG_VERSION")));

        let append_app_name = CString::new(format!(" +{}", APP_NAME)).unwrap();
        let c_str = append_app_name.as_ptr();
        unsafe {
            String_AppendConst(&mut Server.AppName, c_str);
        }

        Self {
            async_manager: AsyncManager::new(),
            chat: Chat::new(),
            entity_manager: EntityManager::new(),
        }
    }

    /// Called once on our plugin's `OnNewMapLoaded`
    pub fn initialize(&mut self) {
        debug!("initialize async_manager");
        self.async_manager.initialize();
        debug!("initialize chat");
        self.chat.initialize();
        debug!("initialize entity_manager");
        self.entity_manager.initialize();

        debug!("initialize cef");
        cef::initialize();
    }

    /// Called once on our plugin's `Free`
    pub fn shutdown(&mut self) {
        players::shutdown();
        self.entity_manager.shutdown();
        self.chat.shutdown();

        cef::shutdown();

        self.async_manager.shutdown();
        debug!("shutdown OK");
    }
}

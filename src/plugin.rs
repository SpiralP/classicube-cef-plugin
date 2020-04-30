use crate::{async_manager::AsyncManager, cef::Cef, chat::Chat, entity_manager::EntityManager};
use classicube_helpers::OptionWithInner;
use classicube_sys::{Server, String_AppendConst};
use log::debug;
use std::{cell::RefCell, ffi::CString};

thread_local!(
    static PLUGIN: RefCell<Option<Plugin>> = RefCell::new(None);
);

pub struct Plugin {
    async_manager: AsyncManager,
    chat: Chat,
    entity_manager: EntityManager,
    context_initialized: bool,
}

pub const APP_NAME: &str = concat!("cef", env!("CARGO_PKG_VERSION"));

impl Plugin {
    /// Called once on our plugin's `Init`
    pub fn initialize() {
        debug!("plugin initialize");

        PLUGIN.with(|cell| {
            assert!(cell.borrow().is_none());

            Chat::print(format!("Cef v{} initializing", env!("CARGO_PKG_VERSION")));

            let append_app_name = CString::new(format!(" +{}", APP_NAME)).unwrap();
            let c_str = append_app_name.as_ptr();
            unsafe {
                String_AppendConst(&mut Server.AppName, c_str);
            }

            let mut async_manager = AsyncManager::new();
            let mut chat = Chat::new();

            async_manager.initialize();
            chat.initialize();

            AsyncManager::spawn_local_on_main_thread(async {
                Cef::initialize().await;
            });

            let plugin = Self {
                async_manager,
                chat,
                entity_manager: EntityManager::new(),
                context_initialized: false,
            };
            *cell.borrow_mut() = Some(plugin);
        });
    }

    /// Called every time when our plugin's `OnNewMapLoaded` is called
    ///
    /// Rendering context is set up by now.
    pub fn on_new_map_loaded() {
        debug!("plugin on_new_map_loaded");

        PLUGIN
            .with_inner_mut(|plugin| {
                if !plugin.context_initialized {
                    plugin.entity_manager.initialize();

                    plugin.context_initialized = true;
                }

                plugin.entity_manager.on_new_map_loaded();

                plugin.chat.on_new_map_loaded();
            })
            .unwrap();
    }

    /// Called once on our plugin's `Free`
    pub fn shutdown() {
        debug!("plugin shutdown");

        PLUGIN.with(|cell| {
            let plugin = &mut *cell.borrow_mut();
            let mut plugin = plugin.take().unwrap();

            plugin.entity_manager.shutdown();
            plugin.chat.shutdown();

            AsyncManager::spawn_local_on_main_thread(async {
                Cef::shutdown().await;
            });

            // this will run all remaining tasks to completion
            plugin.async_manager.shutdown();
        });
    }
}

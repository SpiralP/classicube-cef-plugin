mod cef_paint;
mod chat;
mod chat_command;
mod entity;
mod interface;
mod model;
mod render_model;

use self::{
    cef_paint::{cef_paint_callback, CEF_CAN_DRAW},
    chat::{handle_chat_received, print},
    chat_command::c_chat_command_callback,
    entity::CefEntity,
    interface::*,
    model::CefModel,
    render_model::local_player_render_model_hook,
};
use crate::helpers::*;
use async_dispatcher::{Dispatcher, DispatcherHandle, LocalDispatcherHandle};
use classicube_helpers::{
    detour::GenericDetour,
    events::{
        chat::{ChatReceivedEvent, ChatReceivedEventHandler},
        gfx::{ContextLostEventHandler, ContextRecreatedEventHandler},
    },
    tick::*,
};
use classicube_sys::{
    Entities, Entity, OwnedChatCommand, OwnedGfxVertexBuffer, VertexFormat__VERTEX_FORMAT_P3FC4B,
    VertexFormat__VERTEX_FORMAT_P3FT2FC4B, ENTITIES_SELF_ID,
};
use lazy_static::lazy_static;
use std::{
    cell::RefCell,
    collections::HashMap,
    future::Future,
    os::raw::{c_double, c_float, c_int},
    pin::Pin,
    rc::Rc,
    sync::{atomic::Ordering, Mutex},
    thread,
    time::Duration,
};

// Some means we are initialized
thread_local!(
    pub static CEF: RefCell<Option<Cef>> = RefCell::new(None);
);

thread_local!(
    static ASYNC_DISPATCHER: RefCell<Option<Dispatcher>> = RefCell::new(None);
);

thread_local!(
    static ASYNC_DISPATCHER_LOCAL_HANDLE: RefCell<Option<LocalDispatcherHandle>> =
        RefCell::new(None);
);

lazy_static! {
    static ref ASYNC_DISPATCHER_HANDLE: Mutex<Option<DispatcherHandle>> = Mutex::new(None);
}

lazy_static! {
    static ref TOKIO_RUNTIME: Mutex<Option<tokio::runtime::Runtime>> = Mutex::new(None);
}

// identifier, browser
thread_local!(
    static BROWSERS: RefCell<HashMap<c_int, RustRefBrowser>> = RefCell::new(HashMap::new());
);

pub fn initialize() {
    print("cef initialize");

    CEF.with(|cell| {
        assert!(cell.borrow().is_none());

        *cell.borrow_mut() = Some(Cef::new());
    });

    CEF.with_inner_mut(|cef| {
        cef.initialize();
    })
    .unwrap();
}

pub fn shutdown() {
    print("cef shutdown");

    CEF.with_inner_mut(|cef| {
        cef.shutdown();
    });

    CEF.with(|cell| {
        cell.borrow_mut().take().unwrap();
    });
}

pub struct Cef {
    pub model: Option<Pin<Box<CefModel>>>,
    pub entity: Option<Pin<Box<CefEntity>>>,

    pub local_player_render_model_detour:
        GenericDetour<unsafe extern "C" fn(*mut Entity, c_double, c_float)>,

    tick_handler: TickEventHandler,
    chat_command: Pin<Box<OwnedChatCommand>>,
    chat_received: ChatReceivedEventHandler,

    context_lost_handler: ContextLostEventHandler,
    context_recreated_handler: ContextRecreatedEventHandler,

    app: Option<RustRefApp>,
    client: Option<RustRefClient>,
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
            chat_command,
            context_lost_handler: ContextLostEventHandler::new(),
            context_recreated_handler: ContextRecreatedEventHandler::new(),
            chat_received: ChatReceivedEventHandler::new(),
            app: None,
            client: None,
        }
    }

    pub fn context_recreated(&mut self) {
        // create texture, vertex buffers, enable detour

        QUAD_VB.with(|cell| {
            *cell.borrow_mut() = Some(OwnedGfxVertexBuffer::create(
                VertexFormat__VERTEX_FORMAT_P3FC4B,
                4,
            ));
        });

        TEX_VB.with(|cell| {
            *cell.borrow_mut() = Some(OwnedGfxVertexBuffer::create(
                VertexFormat__VERTEX_FORMAT_P3FT2FC4B,
                4,
            ));
        });

        // Start calling our CefEntity's draw
        unsafe {
            println!("enable RenderModel detour");
            self.local_player_render_model_detour.enable().unwrap();
        }

        CEF_CAN_DRAW.store(true, Ordering::SeqCst);
    }

    pub fn context_lost(&mut self) {
        CEF_CAN_DRAW.store(false, Ordering::SeqCst);

        // disable detour so we don't call our ModelRender
        if self.local_player_render_model_detour.is_enabled() {
            println!("disable RenderModel detour");
            unsafe {
                self.local_player_render_model_detour.disable().unwrap();
            }
        } else {
            println!("RenderModel detour already disabled?");
        }

        // delete vertex buffers
        QUAD_VB.with(|cell| {
            cell.borrow_mut().take();
        });

        TEX_VB.with(|cell| {
            cell.borrow_mut().take();
        });
    }

    /// Called once on our plugin's `init`
    pub fn initialize(&mut self) {
        self.chat_command.as_mut().register();

        self.chat_received.on(
            |ChatReceivedEvent {
                 message,
                 message_type,
             }| {
                handle_chat_received(message.to_string(), *message_type);
            },
        );

        self.context_lost_handler.on(|_| {
            println!("ContextLost {:?}", std::thread::current().id());

            CEF.with_inner_mut(|cef| {
                cef.context_lost();
            })
            .unwrap();
        });

        self.context_recreated_handler.on(|_| {
            println!("ContextRecreated {:?}", std::thread::current().id());

            CEF.with_inner_mut(|cef| {
                cef.context_recreated();
            })
            .unwrap();
        });

        self.model = Some(CefModel::register("cef", "cef"));
        self.entity = Some(CefEntity::register());

        self.tick_handler.on(|_task| {
            Cef::step();
        });

        let async_dispatcher = Dispatcher::new();
        *ASYNC_DISPATCHER_HANDLE.lock().unwrap() = Some(async_dispatcher.get_handle());
        ASYNC_DISPATCHER_LOCAL_HANDLE.with(|cell| {
            *cell.borrow_mut() = Some(async_dispatcher.get_handle_local());
        });
        ASYNC_DISPATCHER.with(|cell| {
            *cell.borrow_mut() = Some(async_dispatcher);
        });

        let rt = tokio::runtime::Builder::new()
            .threaded_scheduler()
            .enable_all()
            .build()
            .unwrap();

        *TOKIO_RUNTIME.lock().unwrap() = Some(rt);

        // self.tokio_runtime.as_mut().unwrap().spawn(async {
        //     // :(
        //     tokio::time::delay_for(Duration::from_millis(2000)).await;

        //     loop {
        //         tokio::time::delay_for(Duration::from_millis(100)).await;

        //         Self::run_on_main_thread(async {
        //             let me = unsafe { &*Entities.List[ENTITIES_SELF_ID as usize] };
        //             let player_pos = Vec3 {
        //                 X: 64.0 - 4.0,
        //                 Y: 48.0,
        //                 Z: 64.0,
        //             };

        //             let percent = (player_pos - me.Position).length_squared() * 0.4;
        //             let percent = (100.0 - percent).max(0.0).min(100.0);

        //             let code = format!(
        //                 r#"if (window.player && window.player.setVolume) {{
        //                     window.player.setVolume({});
        //                 }}"#,
        //                 percent
        //             );
        //             let c_str = CString::new(code).unwrap();
        //             unsafe {
        //                 assert_eq!(crate::bindings::cef_run_script(c_str.as_ptr()), 0);
        //             }
        //         })
        //         .await;
        //     }
        // });

        // finally initialize cef via our App

        extern "C" fn on_context_initialized_callback(client: RustRefClient) {
            // on the main thread

            println!(
                "on_context_initialized_callback {:?} {:?}",
                std::thread::current().id(),
                client
            );

            // need to defer here because app.initialize() calls context_initialized
            // right away, and self is still borrowed there
            Cef::defer_on_main_thread(async {
                CEF.with_inner_mut(|cef| cef.on_context_initialized(client))
                    .unwrap();
            });
        }

        extern "C" fn on_after_created_callback(browser: RustRefBrowser) {
            // on the main thread

            println!(
                "on_after_created_callback {:?} {:?}",
                std::thread::current().id(),
                browser
            );

            let id = browser.get_identifier();
            BROWSERS.with(|cell| cell.borrow_mut().insert(id, browser));

            TOKIO_RUNTIME
                .with_inner(|rt| {
                    rt.spawn(async move {
                        tokio::time::delay_for(Duration::from_secs(4)).await;

                        Cef::spawn_on_main_thread(async move {
                            BROWSERS.with(|cell| {
                                if let Some(browser) = cell.borrow_mut().get_mut(&id) {
                                    browser
                                        .load_url(
                                            "https://www.youtube.com/embed/gQngg8iQipk?autoplay=1"
                                                .to_string(),
                                        )
                                        .unwrap();
                                }
                            });
                        });

                        tokio::time::delay_for(Duration::from_secs(4)).await;
                    });
                })
                .unwrap();
        }

        extern "C" fn on_before_close_callback(browser: RustRefBrowser) {
            println!(
                "on_before_close_callback {:?} {:?}",
                std::thread::current().id(),
                browser
            );

            let id = browser.get_identifier();
            BROWSERS.with(|cell| {
                cell.borrow_mut()
                    .remove(&id)
                    .expect("browser already removed from browsers")
            });
        }

        let ref_app = RustRefApp::create(
            Some(on_context_initialized_callback),
            Some(on_after_created_callback),
            Some(on_before_close_callback),
            Some(cef_paint_callback),
        );
        ref_app.initialize().unwrap();

        self.app = Some(ref_app);
    }

    fn on_context_initialized(&mut self, client: RustRefClient) {
        client
            .create_browser("https://www.classicube.net/".to_string())
            .unwrap();

        self.client = Some(client);
    }

    /// Called once on our plugin's `free` or on Drop (crashed)
    pub fn shutdown(&mut self) {
        {
            if self.entity.is_some() {
                println!("shutdown entity");
                self.entity.take();
            } else {
                println!("entity already shutdown?");
            }
        }

        {
            if self.model.is_some() {
                println!("shutdown model");
                self.model.take();
            } else {
                println!("model already shutdown?");
            }
        }

        {
            println!("shutdown context");
            self.context_lost();
        }

        {
            if !BROWSERS.with(|cell| cell.borrow().is_empty()) {
                println!("shutdown cef browsers");

                // get first browser in map, calling close on the browser and returning its id
                while let Some((id, browser)) = BROWSERS.with(|cell| {
                    let browsers = &*cell.borrow();

                    if let Some((&id, browser)) = browsers.iter().next() {
                        Some((id, browser.clone()))
                    } else {
                        None
                    }
                }) {
                    println!("shutdown browser {} {:?}", id, browser);
                    browser.close().unwrap();

                    // keep looping until our id doesn't exist in the map anymore
                    while BROWSERS.with(|cell| cell.borrow().contains_key(&id)) {
                        println!("waiting for browser {}", id);
                        Cef::step();

                        thread::sleep(Duration::from_millis(64));
                    }
                }
                println!("shut down all browsers");
            } else {
                println!("cef browsers already shutdown?");
            }
        }

        {
            if self.client.is_some() {
                println!("shutdown cef client");
                self.client.take();
            } else {
                println!("cef client already shutdown?");
            }
        }

        {
            let mut option = TOKIO_RUNTIME.lock().unwrap();
            if option.is_some() {
                println!("shutdown tokio");
                if let Some(rt) = option.take() {
                    rt.shutdown_timeout(Duration::from_millis(100));
                }
            } else {
                println!("tokio already shutdown?");
            }
        }

        {
            if ASYNC_DISPATCHER.with_inner(|_| ()).is_some() {
                println!("shutdown async_dispatcher");

                ASYNC_DISPATCHER_HANDLE.lock().unwrap().take();
                ASYNC_DISPATCHER_LOCAL_HANDLE.with(|cell| cell.borrow_mut().take());
                ASYNC_DISPATCHER.with(|cell| cell.borrow_mut().take());
            } else {
                println!("async_dispatcher already shutdown?");
            }
        }

        {
            if self.app.is_some() {
                println!("shutdown cef app");
                self.app.take();
            } else {
                println!("cef app already shutdown?");
            }
        }

        println!("shutdown OK");
    }

    pub fn step() {
        // process futures
        ASYNC_DISPATCHER.with_inner_mut(|async_dispatcher| {
            async_dispatcher.run_until_stalled();
        });

        CefInterface::step().unwrap();
    }

    // pub fn load(&mut self, url: String) {
    //     let c_str = CString::new(url).unwrap();

    //     unsafe {
    //         assert_eq!(cef_load(c_str.as_ptr()), 0);
    //     }
    // }

    // pub fn run_script(&mut self, code: String) {
    //     let c_str = CString::new(code).unwrap();
    //     unsafe {
    //         assert_eq!(cef_run_script(c_str.as_ptr()), 0);
    //     }
    // }

    #[allow(dead_code)]
    pub fn spawn_on_main_thread<F>(f: F)
    where
        F: Future<Output = ()> + 'static + Send,
    {
        let mut handle = {
            let mut handle = ASYNC_DISPATCHER_HANDLE.lock().unwrap();
            handle.as_mut().expect("handle.as_mut()").clone()
        };

        handle.spawn(f);
    }

    #[allow(dead_code)]
    pub async fn run_on_main_thread<F, O>(f: F) -> O
    where
        F: Future<Output = O> + 'static + Send,
        O: 'static + Send + std::fmt::Debug,
    {
        let mut handle = {
            let mut handle = ASYNC_DISPATCHER_HANDLE.lock().unwrap();
            handle.as_mut().expect("handle.as_mut()").clone()
        };

        handle.dispatch(f).await
    }

    #[allow(dead_code)]
    pub fn defer_on_main_thread<F>(f: F)
    where
        F: Future<Output = ()> + 'static,
    {
        let mut handle = ASYNC_DISPATCHER_LOCAL_HANDLE
            .with_inner(|handle| handle.clone())
            .expect("ASYNC_DISPATCHER_LOCAL_HANDLE is None");

        handle.spawn(f);
    }
}

impl Drop for Cef {
    fn drop(&mut self) {
        println!("DROP SHUTDOWN");
        self.shutdown();
    }
}

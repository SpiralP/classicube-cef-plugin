mod async_manager;
mod chat;
mod entity_manager;
mod interface;

use self::{
    async_manager::AsyncManager,
    chat::Chat,
    entity_manager::{cef_paint_callback, CefEntityManager},
    interface::{RustRefApp, RustRefBrowser, RustRefClient},
};
use crate::helpers::WithInner;
use std::{cell::RefCell, collections::HashMap, os::raw::c_int, thread, time::Duration};

// Some means we are initialized
thread_local!(
    pub static CEF: RefCell<Option<Cef>> = RefCell::new(None);
);

// identifier, browser
thread_local!(
    static BROWSERS: RefCell<HashMap<c_int, RustRefBrowser>> = RefCell::new(HashMap::new());
);

pub fn initialize() {
    Chat::print("cef initialize");

    CEF.with(|cell| {
        assert!(cell.borrow().is_none());

        *cell.borrow_mut() = Some(Cef::new());
    });
}

pub fn on_first_context_created() {
    CEF.with_inner_mut(|cef| {
        cef.initialize();
    })
    .unwrap();
}

pub fn shutdown() {
    Chat::print("cef shutdown");

    CEF.with_inner_mut(|cef| {
        cef.shutdown();
    });

    CEF.with(|cell| {
        cell.borrow_mut().take().unwrap();
    });
}

pub struct Cef {
    app: Option<RustRefApp>,
    client: Option<RustRefClient>,

    async_manager: AsyncManager,
    chat: Chat,
    entity_manager: CefEntityManager,
}

impl Cef {
    pub fn new() -> Self {
        Self {
            app: None,
            client: None,

            async_manager: AsyncManager::new(),
            chat: Chat::new(),
            entity_manager: CefEntityManager::new(),
        }
    }

    /// Called once on our plugin's `init`
    pub fn initialize(&mut self) {
        println!("initialize async_manager");
        self.async_manager.initialize();
        println!("initialize chat");
        self.chat.initialize();
        println!("initialize entity_manager");
        self.entity_manager.initialize();

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
            AsyncManager::defer_on_main_thread(async move {
                CEF.with_inner_mut(|cef| cef.on_context_initialized(client))
                    .unwrap();
            });
        }

        extern "C" fn on_before_browser_close(browser: RustRefBrowser) {
            let id = browser.get_identifier();

            println!(
                "on_before_browser_close {} {:?} {:?}",
                id,
                std::thread::current().id(),
                browser
            );

            BROWSERS.with(|cell| {
                cell.borrow_mut()
                    .remove(&id)
                    .expect("browser already removed from browsers")
            });
            CefEntityManager::on_browser_close(browser);
        }

        let ref_app = RustRefApp::create(
            Some(on_context_initialized_callback),
            Some(on_before_browser_close),
            Some(cef_paint_callback),
        );
        ref_app.initialize().unwrap();

        self.app = Some(ref_app);
    }

    fn on_context_initialized(&mut self, client: RustRefClient) {
        self.client = Some(client);
    }

    pub fn create_browser(&self, url: String) -> RustRefBrowser {
        let browser = self.client.as_ref().unwrap().create_browser(url);

        let id = browser.get_identifier();
        println!("create_browser {}", id);

        BROWSERS.with(|cell| cell.borrow_mut().insert(id, browser.clone()));
        CefEntityManager::create_entity(browser.clone());

        browser
    }

    /// Called once on our plugin's `free` or on Drop (crashed)
    pub fn shutdown(&mut self) {
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

                        // process cef's event loop
                        AsyncManager::step();

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
            if self.app.is_some() {
                println!("shutdown cef app");
                if let Some(app) = self.app.take() {
                    app.shutdown().unwrap();
                }
            } else {
                println!("cef app already shutdown?");
            }
        }

        self.entity_manager.shutdown();
        self.chat.shutdown();
        self.async_manager.shutdown();

        println!("shutdown OK");
    }
}

impl Drop for Cef {
    fn drop(&mut self) {
        println!("DROP SHUTDOWN");
        self.shutdown();
    }
}

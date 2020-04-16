mod async_manager;
mod chat;
mod entity_manager;
mod interface;

pub use self::interface::RustRefBrowser;
use self::{
    async_manager::AsyncManager,
    chat::Chat,
    entity_manager::{cef_paint_callback, CefEntityManager},
    interface::{RustRefApp, RustRefClient},
};
use crate::players;
use classicube_helpers::with_inner::WithInner;
use classicube_sys::{Server, String_AppendConst};
use log::debug;
use std::{
    cell::RefCell, collections::HashMap, ffi::CString, os::raw::c_int, thread, time::Duration,
};

thread_local!(
    pub static CEF: RefCell<Option<Cef>> = RefCell::new(None);
);

// identifier, browser
thread_local!(
    static BROWSERS: RefCell<HashMap<c_int, RustRefBrowser>> = RefCell::new(HashMap::new());
);

pub fn initialize() {
    {
        let append_app_name = CString::new(format!(" +cef{}", env!("CARGO_PKG_VERSION"))).unwrap();

        let c_str = append_app_name.as_ptr();

        unsafe {
            String_AppendConst(&mut Server.AppName, c_str);
        }
    }

    Chat::print(format!("Cef v{} initializing", env!("CARGO_PKG_VERSION")));

    CEF.with(|cell| {
        assert!(cell.borrow().is_none());

        *cell.borrow_mut() = Some(Cef::new());
    });
}

pub fn on_first_context_created() {
    CEF.with_inner_mut(|cef| {
        debug!("cef initialize");
        cef.initialize();
    })
    .unwrap();
}

pub fn shutdown() {
    CEF.with_inner_mut(|cef| {
        debug!("cef shutdown");
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
        debug!("initialize async_manager");
        self.async_manager.initialize();
        debug!("initialize chat");
        self.chat.initialize();
        debug!("initialize entity_manager");
        self.entity_manager.initialize();

        // finally initialize cef via our App

        extern "C" fn on_context_initialized_callback(client: RustRefClient) {
            // on the main thread

            debug!(
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

            debug!(
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

        extern "C" fn on_browser_page_loaded(browser: RustRefBrowser) {
            players::on_browser_page_loaded(browser);
        }

        let ref_app = RustRefApp::create(
            Some(on_context_initialized_callback),
            Some(on_before_browser_close),
            Some(cef_paint_callback),
            Some(on_browser_page_loaded),
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
        debug!("create_browser {}", id);

        BROWSERS.with(|cell| cell.borrow_mut().insert(id, browser.clone()));
        CefEntityManager::create_entity(browser.clone());

        browser
    }

    /// Called once on our plugin's `free` or on Drop (crashed)
    pub fn shutdown(&mut self) {
        players::shutdown();

        {
            if !BROWSERS.with(|cell| cell.borrow().is_empty()) {
                debug!("shutdown all cef browsers");

                BROWSERS.with(|cell| {
                    let browsers = &*cell.borrow();

                    for (id, browser) in browsers {
                        debug!("shutdown browser {} {:?}", id, browser);
                        browser.close().unwrap();
                    }
                });

                // keep looping until our id doesn't exist in the map anymore
                while !BROWSERS.with(|cell| cell.borrow().is_empty()) {
                    debug!("waiting for browsers...");

                    // process cef's event loop
                    AsyncManager::step();

                    thread::sleep(Duration::from_millis(64));
                }
                debug!("shut down all browsers");
            } else {
                debug!("cef browsers already shutdown?");
            }
        }

        {
            if self.client.is_some() {
                debug!("shutdown cef client");
                self.client.take();
            } else {
                debug!("cef client already shutdown?");
            }
        }

        {
            if self.app.is_some() {
                debug!("shutdown cef app");
                if let Some(app) = self.app.take() {
                    app.shutdown().unwrap();
                }
            } else {
                debug!("cef app already shutdown?");
            }
        }

        self.entity_manager.shutdown();
        self.chat.shutdown();
        self.async_manager.shutdown();

        debug!("shutdown OK");
    }
}

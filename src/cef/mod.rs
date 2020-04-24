mod bindings;
mod browser;

pub use self::bindings::{Callbacks, RustRefApp, RustRefBrowser, RustRefClient};
use self::browser::BROWSERS;
use crate::{async_manager::AsyncManager, entity_manager::cef_paint_callback};
use classicube_helpers::{shared::FutureShared, CellGetSet, OptionWithInner};
use futures::stream::{FuturesUnordered, StreamExt};
use log::debug;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    os::raw::c_int,
    time::Duration,
};
use tokio::sync::broadcast;

// we've set cef to render at 60 fps
// (1/60)*1000 = 16.6666666667
const CEF_RATE: Duration = Duration::from_millis(16);

#[derive(Clone)]
pub enum CefEvent {
    ContextInitialized(RustRefClient),
    BrowserCreated(RustRefBrowser),
    BrowserPageLoaded(RustRefBrowser),
    BrowserTitleChange(RustRefBrowser, String),
    BrowserClosed(RustRefBrowser),
}

thread_local!(
    static CEF: FutureShared<Option<Cef>> = FutureShared::new(None);
);

thread_local!(
    static EVENT_QUEUE: RefCell<
        Option<(broadcast::Sender<CefEvent>, broadcast::Receiver<CefEvent>)>,
    > = RefCell::new(Some(broadcast::channel(32)));
);

thread_local!(
    static IS_INITIALIZED: Cell<bool> = Cell::new(false);
);

extern "C" fn on_context_initialized_callback(client: RustRefClient) {
    debug!("on_context_initialized_callback {:?}", client);

    EVENT_QUEUE
        .with_inner_mut(move |(sender, _)| {
            let _ignore_error = sender.send(CefEvent::ContextInitialized(client));
        })
        .unwrap();
}

pub struct Cef {
    pub app: RustRefApp,
    pub client: RustRefClient,

    create_browser_mutex: FutureShared<()>,
}

impl Cef {
    pub async fn initialize() {
        debug!("initialize cef");

        let app = RustRefApp::create(Callbacks {
            on_context_initialized_callback: Some(on_context_initialized_callback),
            on_after_created_callback: Some(browser::on_after_created),
            on_before_close_callback: Some(browser::on_before_close),
            on_load_end_callback: Some(browser::on_page_loaded),
            on_title_change_callback: Some(browser::on_title_change),
            on_paint_callback: Some(cef_paint_callback),
        });

        let mut event_receiver = Self::create_event_listener();

        app.initialize().unwrap();

        let client = loop {
            if let CefEvent::ContextInitialized(client) = event_receiver.recv().await.unwrap() {
                break client;
            }
        };

        IS_INITIALIZED.set(true);

        AsyncManager::spawn_local_on_main_thread(async move {
            while crate::time_silent!("Cef::try_step()", 100, { Cef::try_step() }) {
                AsyncManager::sleep(CEF_RATE).await;
            }
        });

        let cef = Self {
            app,
            client,
            create_browser_mutex: FutureShared::new(()),
        };

        let mut mutex = CEF.with(|mutex| mutex.clone());
        let mut global_cef = mutex.lock().await;
        *global_cef = Some(cef);
    }

    pub async fn shutdown() {
        let app = {
            let mut mutex = CEF.with(|mutex| mutex.clone());
            let mut global_cef = mutex.lock().await;
            let cef = global_cef.take().unwrap();

            debug!("shutting down all browsers");
            crate::time!("Cef::close_all_browsers()", 1000, {
                Self::close_all_browsers().await;
            });

            cef.app
        };

        // must clear this before .shutdown() because it holds refs
        // to cef objects
        EVENT_QUEUE.with(|cell| {
            let event_queue = &mut *cell.borrow_mut();
            event_queue.take().unwrap();
        });

        crate::time!("cef app.shutdown()", 1000, {
            app.shutdown().unwrap();
        });
        IS_INITIALIZED.set(false);
    }

    fn try_step() -> bool {
        if IS_INITIALIZED.get() {
            RustRefApp::step().unwrap();
            true
        } else {
            false
        }
    }

    pub fn create_event_listener() -> broadcast::Receiver<CefEvent> {
        EVENT_QUEUE
            .with_inner(|(sender, _receiver)| sender.subscribe())
            .unwrap()
    }

    pub async fn create_browser(url: String) -> RustRefBrowser {
        let (mut create_browser_mutex, client, mut event_receiver) = {
            let mut mutex = CEF.with(|mutex| mutex.clone());
            let maybe_cef = mutex.lock().await;
            let cef = maybe_cef.as_ref().unwrap();

            let create_browser_mutex = cef.create_browser_mutex.clone();
            let client = cef.client.clone();
            let event_receiver = Self::create_event_listener();

            (create_browser_mutex, client, event_receiver)
        };

        // Since we can't distinguish which browser was created if multiple
        // create at the same time, we only allow 1 to be in the "creating"
        // state at a time.
        let _ = create_browser_mutex.lock().await;

        client.create_browser(url).unwrap();

        loop {
            if let CefEvent::BrowserCreated(browser) = event_receiver.recv().await.unwrap() {
                break browser;
            }
        }
    }

    pub async fn close_browser(browser: &RustRefBrowser) {
        let mut event_receiver = Self::create_event_listener();

        let id = browser.get_identifier();

        browser.close().unwrap();

        loop {
            if let CefEvent::BrowserClosed(browser) = event_receiver.recv().await.unwrap() {
                if browser.get_identifier() == id {
                    break;
                }
            }
        }
    }

    pub async fn close_all_browsers() {
        // must clone here or we will recurse into `close` and borrow multiple times
        let browsers: HashMap<c_int, RustRefBrowser> = BROWSERS.with(|cell| {
            let browsers = &mut *cell.borrow_mut();
            browsers.drain().collect()
        });

        let mut ids: FuturesUnordered<_> = browsers
            .iter()
            .map(|(id, browser)| async move {
                debug!("closing browser {}", id);
                Self::close_browser(browser).await;
                id
            })
            .collect();

        while let Some(id) = ids.next().await {
            debug!("browser {} closed", id);
        }
    }
}

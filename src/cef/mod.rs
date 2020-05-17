mod bindings;
mod browser;
mod javascript;

use self::browser::{BROWSERS, BROWSER_SIZES};
pub use self::{
    bindings::{Callbacks, RustRefApp, RustRefBrowser, RustRefClient},
    javascript::RustV8Value,
};
use crate::{
    async_manager::AsyncManager,
    entity_manager::{cef_paint_callback, TEXTURE_HEIGHT, TEXTURE_WIDTH},
    error::*,
};
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

pub const CEF_DEFAULT_WIDTH: c_int = 1920;
pub const CEF_DEFAULT_HEIGHT: c_int = 1080;

// we've set cef to render at 60 fps
// (1/60)*1000 = 16.6666666667
const CEF_RATE: Duration = Duration::from_millis(16);

#[derive(Debug, Clone)]
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
            on_context_initialized: Some(on_context_initialized_callback),
            on_after_created: Some(browser::on_after_created),
            on_before_close: Some(browser::on_before_close),
            on_load_end: Some(browser::on_page_loaded),
            on_title_change: Some(browser::on_title_change),
            on_paint: Some(cef_paint_callback),
            get_view_rect: Some(browser::get_view_rect),
            on_javascript: Some(javascript::on_javascript_callback),
            on_certificate_error: Some(browser::on_certificate_error_callback),
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

        Self::warm_up();
    }

    fn warm_up() {
        // load a blank browser so that the next load is quicker
        AsyncManager::spawn_local_on_main_thread(async {
            let browser = Self::create_browser("data:text/html,", 30).await.unwrap();
            Self::close_browser(&browser).await.unwrap();
        });
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

    pub async fn create_browser<T: Into<Vec<u8>>>(url: T, fps: u16) -> Result<RustRefBrowser> {
        let mut create_browser_mutex = {
            let mut mutex = CEF.with(|mutex| mutex.clone());
            let maybe_cef = mutex.lock().await;
            let cef = maybe_cef.as_ref().unwrap();

            cef.create_browser_mutex.clone()
        };

        // Since we can't distinguish which browser was created if multiple
        // create at the same time, we only allow 1 to be in the "creating"
        // state at a time.
        let mutex = create_browser_mutex.lock().await;

        let (client, mut event_receiver) = {
            let mut mutex = CEF.with(|mutex| mutex.clone());
            let maybe_cef = mutex.lock().await;
            let cef = maybe_cef.as_ref().unwrap();

            let client = cef.client.clone();
            let event_receiver = Self::create_event_listener();

            (client, event_receiver)
        };

        client.create_browser(url, fps as _)?;

        let browser = loop {
            if let CefEvent::BrowserCreated(browser) = event_receiver.recv().await.unwrap() {
                break browser;
            }
        };

        let browser_id = browser.get_identifier();

        debug!("Cef::create_browser => {}", browser_id);

        drop(mutex);

        Ok(browser)
    }

    pub async fn close_browser(browser: &RustRefBrowser) -> Result<()> {
        let mut event_receiver = Self::create_event_listener();

        let id = browser.get_identifier();

        browser.close()?;

        loop {
            if let CefEvent::BrowserClosed(browser) = event_receiver.recv().await.unwrap() {
                if browser.get_identifier() == id {
                    break;
                }
            }
        }

        Ok(())
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
                Self::close_browser(browser).await.unwrap();
                id
            })
            .collect();

        while let Some(id) = ids.next().await {
            debug!("browser {} closed", id);
        }
    }

    fn set_browser_size(browser_id: c_int, width: usize, height: usize) -> Result<()> {
        // 0 size crashes
        if width < 1 || height < 1 || width > TEXTURE_WIDTH || height > TEXTURE_HEIGHT {
            bail!("size not within {}x{}", TEXTURE_WIDTH, TEXTURE_HEIGHT);
        }

        BROWSER_SIZES.with(move |cell| {
            let sizes = &mut *cell.borrow_mut();

            sizes.insert(browser_id, (width as _, height as _));
        });

        Ok(())
    }

    pub fn resize_browser(browser: &RustRefBrowser, width: usize, height: usize) -> Result<()> {
        let browser_id = browser.get_identifier();

        Self::set_browser_size(browser_id, width, height)?;

        browser.was_resized()?;
        Ok(())
    }

    pub fn get_browser_size(browser: &RustRefBrowser) -> (c_int, c_int) {
        let browser_id = browser.get_identifier();
        BROWSER_SIZES.with(move |cell| {
            let sizes = &*cell.borrow();

            sizes
                .get(&browser_id)
                .cloned()
                .unwrap_or((CEF_DEFAULT_WIDTH, CEF_DEFAULT_HEIGHT))
        })
    }
}

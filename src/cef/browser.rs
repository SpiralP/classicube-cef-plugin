use super::bindings::{RustRefBrowser, RustRefClient};
use crate::entity_manager::EntityManager;
use futures::{
    channel::{mpsc, oneshot},
    prelude::*,
    stream::FuturesUnordered,
};
use log::debug;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    os::raw::c_int,
};

// identifier, browser
thread_local!(
    static BROWSERS: RefCell<HashMap<c_int, RustRefBrowser>> = RefCell::new(HashMap::new());
);

// Since we can't distinguish which browser was created if multiple
// create at the same time, we only allow 1 to be in the "creating"
// state at a time.
type BrowserCreatedFuture = oneshot::Sender<RustRefBrowser>;
thread_local!(
    static CREATED_FUTURE: RefCell<(
        mpsc::Sender<BrowserCreatedFuture>,
        mpsc::Receiver<BrowserCreatedFuture>,
    )> = RefCell::new(mpsc::channel(1));
);

pub extern "C" fn on_after_created(browser: RustRefBrowser) {
    debug!("on_after_created");

    CREATED_FUTURE.with(move |cell| {
        let (_sender, receiver) = &mut *cell.borrow_mut();
        let sender = receiver.try_next().unwrap().unwrap();
        sender.send(browser).unwrap();
    });
}

pub async fn create(client: RustRefClient, url: String) -> RustRefBrowser {
    let (browser_sender, receiver) = oneshot::channel();
    let mut sender_sender = CREATED_FUTURE.with(move |cell| {
        let (sender, _receiver) = &mut *cell.borrow_mut();
        sender.clone()
    });

    sender_sender.send(browser_sender).await.unwrap();

    client.create_browser(url).unwrap();

    let browser = receiver.await.unwrap();

    BROWSERS.with(|cell| {
        let browser = browser.clone();
        let id = browser.get_identifier();

        let browsers = &mut *cell.borrow_mut();
        browsers.insert(id, browser);
    });

    browser
}

// OnBeforeClose

thread_local!(
    static CLOSED_FUTURE: RefCell<HashMap<c_int, oneshot::Sender<()>>> =
        RefCell::new(HashMap::new());
);

pub extern "C" fn on_before_close(browser: RustRefBrowser) {
    let id = browser.get_identifier();

    debug!("on_before_close {} {:?}", id, browser);

    CLOSED_FUTURE.with(move |cell| {
        let map = &mut *cell.borrow_mut();
        let future = map.remove(&id).unwrap();
        future.send(()).unwrap();
    });
}

pub async fn close(browser: &RustRefBrowser) {
    let id = browser.get_identifier();

    let (sender, receiver) = oneshot::channel();
    CLOSED_FUTURE.with(move |cell| {
        let map = &mut *cell.borrow_mut();
        map.insert(id, sender);
    });

    browser.close().unwrap();

    receiver.await.unwrap();

    BROWSERS.with(|cell| {
        let browsers = &mut *cell.borrow_mut();
        browsers.remove(&id);
    });
}

// OnPageLoaded

// TODO only 1 listener can be waiting :(
thread_local!(
    static PAGE_LOADED_FUTURE: RefCell<HashMap<c_int, oneshot::Sender<()>>> =
        RefCell::new(HashMap::new());
);

thread_local!(
    static IS_LOADED: RefCell<HashSet<c_int>> = RefCell::new(HashSet::new());
);

pub extern "C" fn on_page_loaded(browser: RustRefBrowser) {
    let id = browser.get_identifier();
    debug!("on_page_loaded {:?}", browser);

    IS_LOADED.with(|cell| {
        let set = &mut *cell.borrow_mut();
        set.insert(id);
    });

    PAGE_LOADED_FUTURE.with(|cell| {
        let map = &mut *cell.borrow_mut();
        if let Some(future) = map.remove(&id) {
            // resolve anyone if they subscribed
            future.send(()).unwrap();
        }
    });
}

pub async fn wait_for_page_load(browser: &RustRefBrowser) {
    let id = browser.get_identifier();

    let already_loaded = IS_LOADED.with(|cell| {
        let set = &mut *cell.borrow_mut();
        set.contains(&id)
    });

    if !already_loaded {
        // subscribe a future

        let (sender, receiver) = oneshot::channel();
        PAGE_LOADED_FUTURE.with(move |cell| {
            let map = &mut *cell.borrow_mut();
            map.insert(id, sender);
        });

        receiver.await.unwrap()
    }
}

pub async fn close_all() {
    // must clone here or we will recurse into `close` and borrow multiple times
    let cloned_browsers: HashMap<c_int, RustRefBrowser> = BROWSERS.with(|cell| {
        let browsers = &*cell.borrow();
        browsers.clone()
    });

    let mut ag: FuturesUnordered<_> = cloned_browsers
        .iter()
        .map(|(id, browser)| {
            debug!("closing browser {}", id);
            close(browser)
        })
        .collect();

    while let Some(_) = ag.next().await {}

    debug!("finished shutting down all browsers");
}

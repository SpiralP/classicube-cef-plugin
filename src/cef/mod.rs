mod bindings;
mod browser;

pub use self::bindings::{Callbacks, RustRefApp, RustRefBrowser, RustRefClient};
use crate::{async_manager::AsyncManager, entity_manager::cef_paint_callback};
use async_std::future;
use futures::channel::oneshot;
use log::{debug, warn};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    time::{Duration, Instant},
};

// we've set cef to render at 60 fps
// (1/60)*1000 = 16.6666666667
const CEF_RATE: Duration = Duration::from_millis(16);

thread_local!(
    pub static CEF: RefCell<Option<Cef>> = RefCell::new(None);
);

pub fn initialize() {
    let cef = AsyncManager::block_on_local(Cef::initialize());

    CEF.with(|cell| {
        let global_cef = &mut *cell.borrow_mut();

        *global_cef = Some(cef);
    });
}

pub fn shutdown() {
    CEF.with(|cell| {
        let cef = &mut *cell.borrow_mut();

        cef.take().unwrap().shutdown();
    });
}

thread_local!(
    static CONTEXT_INITIALIZED_FUTURE: RefCell<Option<oneshot::Sender<RustRefClient>>> =
        RefCell::new(None);
);

extern "C" fn on_context_initialized_callback(client: RustRefClient) {
    debug!("on_context_initialized_callback {:?}", client);

    CONTEXT_INITIALIZED_FUTURE.with(move |cell| {
        let future = &mut *cell.borrow_mut();
        let future = future.take().unwrap();
        future.send(client).unwrap();
    });
}

thread_local!(
    static IS_INITIALIZED: Cell<bool> = Cell::new(false);
);

pub struct Cef {
    pub app: RustRefApp,
    pub client: RustRefClient,
}

impl Cef {
    pub async fn initialize() -> Self {
        let app = RustRefApp::create(Callbacks {
            on_context_initialized_callback: Some(on_context_initialized_callback),
            on_after_created_callback: Some(browser::on_after_created),
            on_before_close_callback: Some(browser::on_before_close),
            on_load_end_callback: Some(browser::on_page_loaded),
            on_paint_callback: Some(cef_paint_callback),
        });

        let (sender, receiver) = oneshot::channel();
        CONTEXT_INITIALIZED_FUTURE.with(move |cell| {
            let future = &mut *cell.borrow_mut();
            assert!(future.is_none());
            *future = Some(sender);
        });

        app.initialize().unwrap();

        let client = receiver.await.unwrap();

        IS_INITIALIZED.with(|cell| cell.set(true));

        AsyncManager::spawn_local_on_main_thread(async move {
            while {
                let before = Instant::now();
                let res = Cef::try_step();
                let after = Instant::now();
                if after - before > Duration::from_millis(100) {
                    warn!("cef step {:?}", after - before);
                }
                res
            } {
                let _ = future::timeout(CEF_RATE, future::pending::<()>()).await;
            }
        });

        Self { app, client }
    }

    pub fn shutdown(&self) {
        let app = self.app.clone();

        AsyncManager::spawn_local_on_main_thread(async move {
            debug!("shutting down all browsers");
            Self::close_all_browsers().await;

            debug!("shutdown cef");
            app.shutdown().unwrap();
            IS_INITIALIZED.with(|cell| cell.set(false));
        });
    }

    fn try_step() -> bool {
        if IS_INITIALIZED.with(|cell| cell.get()) {
            RustRefApp::step().unwrap();
            true
        } else {
            false
        }
    }

    pub fn create_browser(&self, url: String) -> impl Future<Output = RustRefBrowser> {
        let client = self.client.clone();
        browser::create(client, url)
    }

    pub async fn close_browser(browser: &RustRefBrowser) {
        browser::close(browser).await
    }

    #[allow(dead_code)]
    pub async fn wait_for_browser_page_load(browser: &RustRefBrowser) {
        browser::wait_for_page_load(browser).await
    }

    pub async fn close_all_browsers() {
        browser::close_all().await
    }
}

// #[test]
// fn test_cef() {
//     use crate::cef::AsyncManager;

//     crate::logger::initialize(true, false);

//     unsafe {
//         extern "C" fn ag(_: *mut classicube_sys::ScheduledTask) {}
//         classicube_sys::Server.Tick = Some(ag);
//     }

//     let mut am = AsyncManager::new();
//     am.initialize();

//     let is_shutdown = std::rc::Rc::new(std::cell::Cell::new(false));
//     let is_initialized = std::rc::Rc::new(std::cell::Cell::new(false));

//     {
//         let is_shutdown = is_shutdown.clone();
//         let is_initialized = is_initialized.clone();
//         AsyncManager::spawn_local_on_main_thread(async move {
//             let app = {
//                 debug!("create");
//                 let (app, client) = create_app().await;
//                 is_initialized.set(true);

//                 // {
//                 //     let is_shutdown = is_shutdown.clone();
//                 //     AsyncManager::spawn_local_on_main_thread(async move {
//                 //         while !is_shutdown.get() {
//                 //             debug!(".");
//                 //             RustRefApp::step().unwrap();
//                 //             // tokio::task::yield_now().await;
//                 //             async_std::task::yield_now().await;
//                 //         }
//                 //     });
//                 // }

//                 debug!("create_browser 1");
//                 let browser_1 = create_browser(&client, "https://icanhazip.com/".to_string());
//                 debug!("create_browser 2");
//                 let browser_2 = create_browser(&client, "https://icanhazip.com/".to_string());

//                 let (browser_1, browser_2) = futures::future::join(browser_1, browser_2).await;

//                 let never = futures::future::pending::<()>();
//                 let dur = std::time::Duration::from_secs(2);
//                 assert!(async_std::future::timeout(dur, never).await.is_err());

//                 debug!("browsers close");
//                 futures::future::join_all(
//                     [browser_1, browser_2]
//                         .iter()
//                         .map(|browser| close_browser(browser)),
//                 )
//                 .await;
//                 // close_browser(&browser_1).await;
//                 // close_browser(&browser_2).await;

//                 app
//             };

//             debug!("shutdown");
//             app.shutdown().unwrap();
//             is_shutdown.set(true);
//         });
//     }

//     while !is_shutdown.get() {
//         let before = std::time::Instant::now();
//         AsyncManager::step();
//         if is_initialized.get() && !is_shutdown.get() {
//             RustRefApp::step().unwrap();
//         }
//         debug!("step {:?}", std::time::Instant::now() - before);
//         std::thread::sleep(std::time::Duration::from_millis(100));
//     }
// }

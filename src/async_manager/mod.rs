use async_dispatcher::{Dispatcher, DispatcherHandle, LocalDispatcherHandle};
use classicube_helpers::{tick::TickEventHandler, with_inner::WithInner};
use lazy_static::lazy_static;
use log::debug;
use std::{cell::RefCell, future::Future, sync::Mutex, time::Duration};
use tokio::task::JoinHandle;

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

pub struct AsyncManager {
    tick_handler: TickEventHandler,
}

impl AsyncManager {
    pub fn new() -> Self {
        Self {
            tick_handler: TickEventHandler::new(),
        }
    }

    pub fn initialize(&mut self) {
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

        self.tick_handler.on(|_task| {
            Self::step();
        });
    }

    pub fn shutdown(&mut self) {
        {
            let mut option = TOKIO_RUNTIME.lock().unwrap();
            if option.is_some() {
                debug!("shutdown tokio");
                if let Some(rt) = option.take() {
                    rt.shutdown_timeout(Duration::from_millis(100));
                }
            } else {
                debug!("tokio already shutdown?");
            }
        }

        {
            if ASYNC_DISPATCHER.with_inner(|_| ()).is_some() {
                debug!("shutdown async_dispatcher");

                ASYNC_DISPATCHER_HANDLE.lock().unwrap().take();
                ASYNC_DISPATCHER_LOCAL_HANDLE.with(|cell| cell.borrow_mut().take());
                ASYNC_DISPATCHER.with(|cell| cell.borrow_mut().take());
            } else {
                debug!("async_dispatcher already shutdown?");
            }
        }
    }

    pub fn step() {
        // process futures
        ASYNC_DISPATCHER.with_inner_mut(|async_dispatcher| {
            async_dispatcher.run_until_stalled();
        });
    }

    #[allow(dead_code)]
    pub fn block_on<F>(future: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        futures::executor::block_on(future)
    }

    #[allow(dead_code)]
    pub fn block_on_local<F>(future: F) -> F::Output
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        futures::executor::block_on(future)
    }

    #[allow(dead_code)]
    pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        TOKIO_RUNTIME.with_inner(|rt| rt.spawn(future)).unwrap()
    }

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
    pub fn spawn_local_on_main_thread<F>(f: F)
    where
        F: Future<Output = ()> + 'static,
    {
        let mut handle = ASYNC_DISPATCHER_LOCAL_HANDLE
            .with_inner(|handle| handle.clone())
            .expect("ASYNC_DISPATCHER_LOCAL_HANDLE is None");

        handle.spawn(f);
    }
}

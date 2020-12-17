use async_dispatcher::{Dispatcher, DispatcherHandle, LocalDispatcherHandle};
use classicube_helpers::{tick::TickEventHandler, OptionWithInner};
use futures::{future::Either, prelude::*};
use futures_timer::Delay;
use lazy_static::lazy_static;
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::Mutex,
    task::{Context, Poll},
    time::Duration,
};
use tokio::task::{JoinError, JoinHandle};
use tracing::debug;

thread_local!(
    static ASYNC_DISPATCHER: RefCell<Option<Dispatcher>> = Default::default();
);

thread_local!(
    static ASYNC_DISPATCHER_LOCAL_HANDLE: RefCell<Option<LocalDispatcherHandle>> =
        Default::default();
);

lazy_static! {
    static ref ASYNC_DISPATCHER_HANDLE: Mutex<Option<DispatcherHandle>> = Default::default();
}

lazy_static! {
    static ref TOKIO_RUNTIME: Mutex<Option<tokio::runtime::Runtime>> = Default::default();
}

thread_local!(
    static TICK_HANDLER: RefCell<Option<TickEventHandler>> = Default::default();
);

pub fn initialize() {
    debug!("initialize async_manager");

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

    TICK_HANDLER.with(|cell| {
        let mut tick_handler = TickEventHandler::new();
        tick_handler.on(|_task| {
            step();
        });

        *cell.borrow_mut() = Some(tick_handler);
    });
}

pub fn shutdown() {
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

    {
        if TICK_HANDLER.with_inner(|_| ()).is_some() {
            debug!("shutdown tick_handler");

            TICK_HANDLER.with(|cell| cell.borrow_mut().take());
        } else {
            debug!("tick_handler already shutdown?");
        }
    }
}

thread_local!(
    static YIELDED_WAKERS: RefCell<Vec<Rc<Cell<bool>>>> = Default::default();
);

pub fn step() {
    YIELDED_WAKERS.with(move |cell| {
        let vec = &mut *cell.borrow_mut();
        for waker in vec.drain(..) {
            waker.set(true);
        }
    });

    // process futures
    ASYNC_DISPATCHER
        .with_inner_mut(|async_dispatcher| {
            async_dispatcher.run_until_stalled();
        })
        .unwrap();
}

pub async fn sleep(duration: Duration) {
    let _ = Delay::new(duration).await;
}

pub async fn yield_now() {
    struct YieldNow {
        waker: Rc<Cell<bool>>,
    }

    impl Future for YieldNow {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            cx.waker().wake_by_ref();
            if self.waker.get() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }

    let waker = Rc::new(Cell::new(false));
    {
        let waker = waker.clone();
        YIELDED_WAKERS.with(move |cell| {
            let vec = &mut *cell.borrow_mut();
            vec.push(waker);
        });
    }
    YieldNow { waker }.await
}

pub async fn timeout<T, F>(duration: Duration, f: F) -> Option<T>
where
    F: Future<Output = T> + Send,
{
    let delay = Delay::new(duration);

    match future::select(delay, f.boxed()).await {
        Either::Left((_, _f)) => None,
        Either::Right((r, _delay)) => Some(r),
    }
}

#[allow(dead_code)]
pub async fn timeout_local<T, F>(duration: Duration, f: F) -> Option<T>
where
    F: Future<Output = T>,
{
    let delay = Delay::new(duration);

    match future::select(delay, f.boxed_local()).await {
        Either::Left((_, _f)) => None,
        Either::Right((r, _delay)) => Some(r),
    }
}

/// Block thread until future is resolved.
///
/// This will continue to call the same executor so cef_step() will still be called!
pub fn block_on_local<F>(f: F)
where
    F: Future<Output = ()> + 'static,
{
    use futures::prelude::*;

    let shared = f.shared();

    {
        let shared = shared.clone();
        spawn_local_on_main_thread(async move {
            shared.await;
        });
    }

    loop {
        match shared.peek() {
            Some(_) => {
                return;
            }

            None => {
                step();
            }
        }

        // don't burn anything
        std::thread::sleep(Duration::from_millis(16));
    }
}

pub fn spawn<F>(f: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    TOKIO_RUNTIME.with_inner(|rt| rt.spawn(f)).unwrap()
}

#[allow(dead_code)]
pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<Result<R, JoinError>>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    spawn(async { tokio::task::spawn_blocking(f).await })
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

pub fn spawn_local_on_main_thread<F>(f: F)
where
    F: Future<Output = ()> + 'static,
{
    let mut handle = ASYNC_DISPATCHER_LOCAL_HANDLE
        .with_inner(|handle| handle.clone())
        .expect("ASYNC_DISPATCHER_LOCAL_HANDLE is None");

    handle.spawn(f);
}

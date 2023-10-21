use super::bindings::{FFIRustV8Response, RustRefBrowser};
use futures::channel::oneshot;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    os::raw::c_double,
};
use tracing::warn;

thread_local!(
    static TASK_ID: Cell<u64> = Cell::new(0);
);

thread_local!(
    static WAITING_TASKS: RefCell<HashMap<u64, oneshot::Sender<FFIRustV8Response>>> =
        RefCell::default();
);

#[derive(Debug)]
pub enum RustV8Value {
    Unknown,
    Array,
    ArrayBuffer,
    Bool(bool),
    Date,
    Double(c_double),
    Function,
    Int(i32),
    Null,
    Object,
    String(String),
    UInt(u32),
    Undefined,
}

#[tracing::instrument(fields(_browser, response))]
pub extern "C" fn on_javascript_callback(
    _browser: RustRefBrowser,
    task_id: u64,
    response: FFIRustV8Response,
) {
    // runs on main thread

    let maybe_task = WAITING_TASKS.with(|cell| {
        let waiting_tasks = &mut *cell.borrow_mut();
        waiting_tasks.remove(&task_id)
    });

    if let Some(task) = maybe_task {
        if task.send(response).is_err() {
            warn!("error sending to waiting task {}", task_id);
        }
    } else {
        warn!("no waiting task for id {}", task_id);
    }
}

pub fn create_task() -> (oneshot::Receiver<FFIRustV8Response>, u64) {
    let (sender, receiver) = oneshot::channel();

    let task_id = TASK_ID.get();
    TASK_ID.set(task_id + 1);

    WAITING_TASKS.with(|cell| {
        let waiting_tasks = &mut *cell.borrow_mut();
        waiting_tasks.insert(task_id, sender);
    });

    (receiver, task_id)
}

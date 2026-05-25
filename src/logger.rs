use std::{fs::File, io::BufWriter, sync::Once};

use tracing_flame::FlameLayer;
use tracing_subscriber::{
    filter::EnvFilter,
    fmt::{Layer, time::SystemTime},
    prelude::*,
};

enum Guard {
    #[allow(dead_code)]
    Appender(tracing_appender::non_blocking::WorkerGuard),
    #[allow(dead_code)]
    Flame(tracing_flame::FlushGuard<BufWriter<File>>),
}

// Held for the lifetime of the process. The tracing subscriber is
// installed exactly once (via `Once`); on plugin reload we must keep
// the appender's worker thread alive so log lines after the second
// `Init` still reach `cef.log`.
static mut GUARDS: Option<Vec<Guard>> = None;

pub fn initialize(debug: bool, module_filter: Option<&str>, flame: bool) {
    static ONCE: Once = Once::new();
    ONCE.call_once(move || {
        {
            // erase files so they're only of this session
            let f = File::create("cef-binary.log").unwrap();
            f.set_len(0).unwrap();

            let f = File::create("cef.log").unwrap();
            f.set_len(0).unwrap();
        }

        let level = if debug { "debug" } else { "info" };

        let mut filter = EnvFilter::from_default_env();
        if let Some(module) = module_filter {
            filter = filter.add_directive(format!("{module}={level}").parse().unwrap());
        } else {
            filter = filter.add_directive(level.parse().unwrap());
        }

        let mut guards = Vec::with_capacity(2);

        let (file_writer, guard) =
            tracing_appender::non_blocking(tracing_appender::rolling::never(".", "cef.log"));
        guards.push(Guard::Appender(guard));

        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_ansi(true)
            .without_time()
            .finish()
            .with(
                Layer::default()
                    .with_writer(file_writer)
                    .with_target(false)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_ansi(false)
                    .with_timer(SystemTime),
            );

        let result = if flame {
            let (flame_layer, guard) = FlameLayer::with_file("./flame.log").unwrap();
            guards.push(Guard::Flame(guard));

            subscriber.with(flame_layer).try_init()
        } else {
            subscriber.try_init()
        };
        if let Err(e) = result {
            eprintln!("failed to init tracing subscriber: {e}");
        }

        unsafe {
            GUARDS = Some(guards);
        }
    });
}

/// Flush and drop the appender guards. Only call this from the panic hook
/// just before `process::abort()` — the abort would otherwise skip the
/// `WorkerGuard` destructor and the file writer would lose buffered log
/// lines. **Don't** call this from plugin `Free`: a subsequent `Init`
/// won't re-arm the appender (the `Once` doesn't re-run), and we'd lose
/// file logging for the rest of the process.
pub fn flush_for_abort() {
    unsafe {
        GUARDS = None;
    }
}

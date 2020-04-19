use simplelog::*;
use std::{fs::File, sync::Once};

pub fn initialize(debug: bool) {
    static START: Once = Once::new();

    START.call_once(move || {
        let level = if debug {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        };

        let my_crate_name = env!("CARGO_PKG_NAME").replace("-", "_");

        let mut loggers: Vec<Box<dyn SharedLogger>> = Vec::with_capacity(2);
        loggers.push(WriteLogger::new(
            level,
            ConfigBuilder::new()
                .add_filter_allow(my_crate_name.clone())
                .build(),
            File::create("cef.log").unwrap(),
        ));

        if let Some(term_logger) = TermLogger::new(
            level,
            ConfigBuilder::new()
                .add_filter_allow(my_crate_name)
                .set_target_level(LevelFilter::Trace)
                .set_thread_level(LevelFilter::Trace)
                .build(),
            TerminalMode::Mixed,
        ) {
            loggers.push(term_logger);
        }

        CombinedLogger::init(loggers).unwrap();
    });
}

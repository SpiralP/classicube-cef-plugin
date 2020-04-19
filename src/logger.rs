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

        CombinedLogger::init(vec![
            TermLogger::new(level, Config::default(), TerminalMode::Mixed).unwrap(),
            WriteLogger::new(level, Config::default(), File::create("cef.log").unwrap()),
        ])
        .unwrap();
    });
}

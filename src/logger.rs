use std::sync::Once;

#[inline]
pub fn initialize(debug: bool, other_crates: bool) {
    static START: Once = Once::new();

    START.call_once(move || {
        let my_crate_name = &env!("CARGO_PKG_NAME").replace("-", "_");
        env_logger::Builder::from_default_env()
            .format_timestamp(None)
            .format_module_path(false)
            .filter(
                if other_crates {
                    None
                } else {
                    Some(my_crate_name)
                },
                if debug {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                },
            )
            .init();
    });
}

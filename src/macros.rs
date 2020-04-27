#[macro_export]
macro_rules! time {
    ($title:tt, $block:block) => {{
        let before = ::std::time::Instant::now();
        let res = $block;
        let after = ::std::time::Instant::now();
        let diff = after - before;
        debug!("{} ({:?})", $title, diff);
        res
    }};

    ($title:expr, $high_millis:tt, $block:block) => {{
        let before = ::std::time::Instant::now();
        let res = $block;
        let after = ::std::time::Instant::now();
        let diff = after - before;
        if diff > ::std::time::Duration::from_millis($high_millis) {
            ::log::warn!("{} ({:?})", $title, diff);
        } else {
            ::log::debug!("{} ({:?})", $title, diff);
        }
        res
    }};
}

#[macro_export]
macro_rules! time_silent {
    ($title:expr, $high_millis:tt, $block:block) => {{
        let before = ::std::time::Instant::now();
        let res = $block;
        let after = ::std::time::Instant::now();
        let diff = after - before;
        if diff > ::std::time::Duration::from_millis($high_millis) {
            ::log::warn!("{} ({:?})", $title, diff);
        }
        res
    }};
}

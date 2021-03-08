use backtrace::Backtrace;
use classicube_sys::{DateTime, DateTime_CurrentLocal, Window_ShowDialog};
use std::{ffi::CString, fs, io::Write, mem, panic, panic::PanicInfo, process, thread};

pub fn install_hook() {
    panic::set_hook(Box::new(panic_hook));
}

fn panic_hook(info: &PanicInfo<'_>) {
    unsafe {
        drop(crate::logger::GUARDS.take());
    }

    let (popup_message, stderr_message, verbose_message) = {
        // The current implementation always returns `Some`.
        let panic_location = info.location().unwrap();

        let panic_message = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            },
        };

        let thread = thread::current();
        let thread_name = thread.name().unwrap_or("<unnamed>");
        let bt = Backtrace::new();

        let date = unsafe {
            let mut now: DateTime = mem::zeroed();
            DateTime_CurrentLocal(&mut now);
            format!(
                "{:02}/{:02}/{:04} {:02}:{:02}:{:02}",
                now.day, now.month, now.year, now.hour, now.minute, now.second
            )
        };

        (
            format!(
                "CEF crashed: '{}', {}\nMore details were written to 'cef-crashes.log'\nPlease \
                 report this file to a developer!",
                panic_message, panic_location
            ),
            format!(
                "thread '{}' panicked at '{}', {}\n{:?}",
                thread_name, panic_message, panic_location, bt
            ),
            format!(
                "----------------------------------------\nCEF version {} crashed at {}\nthread \
                 '{}' panicked at '{}', {}\n-- backtrace --\n{:#?}",
                env!("CARGO_PKG_VERSION"),
                date,
                thread_name,
                panic_message,
                panic_location,
                bt
            ),
        )
    };

    eprintln!("{}", stderr_message);

    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("cef-crashes.log")
    {
        drop(writeln!(file, "{}", verbose_message));
        drop(file.flush());
        drop(file);
    }

    unsafe {
        let title = CString::new("CEF crashed!").unwrap();
        let msg = CString::new(popup_message).unwrap();
        Window_ShowDialog(title.as_ptr(), msg.as_ptr());
    }

    process::abort();
}

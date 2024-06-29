#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use classicube_cef_plugin::cef::bindings::cef_interface_execute_process;
use std::{env, ffi::CString, os::raw::c_int, process};
use tracing::{debug, warn};
use tracing_subscriber::EnvFilter;

fn main() {
    #[cfg(all(target_os = "windows", debug_assertions))]
    {
        use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};

        // if we were called from a console, attach to it to make stdout work
        unsafe {
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("cef=debug".parse().unwrap()))
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_ansi(true)
        .without_time()
        .init();

    debug!("Init cef_exe");

    // exit process when parent pid dies
    //
    // on linux:
    // Chromium will refuse to run if it detects more than 1 thread on init (THREAD_SANITIZER in zygote_main_linux.cc),
    // so this needs to only be run on Windows. (I've only seen this issue on Windows.)
    // I see references to PR_SET_PDEATHSIG in linux chromium, so it is probably fine there.
    //
    // on mac, i can't seem to reproduce the child process staying after killall -9 ClassiCube
    #[cfg(target_os = "windows")]
    let stop_parent_watcher = {
        use std::{
            sync::{Arc, Mutex},
            thread,
            time::Duration,
        };
        use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};

        const PROCESS_CHECK_INTERVAL: Duration = Duration::from_secs(2);

        let should_die = Arc::new(Mutex::new(false));
        let thread_handle = {
            let should_die = should_die.clone();
            thread::spawn(move || {
                let mut system = System::new_with_specifics(
                    RefreshKind::new().with_processes(ProcessRefreshKind::new()),
                );
                let my_pid = Pid::from_u32(process::id());

                let parent_pid = system.process(my_pid).and_then(|process| process.parent());
                debug!(?my_pid, ?parent_pid);

                if let Some(parent_pid) = parent_pid {
                    debug!("watching for parent {:?} to die", parent_pid);

                    loop {
                        if *should_die.lock().unwrap() {
                            debug!("dying");
                            return;
                        }

                        if !system.refresh_process_specifics(parent_pid, ProcessRefreshKind::new())
                        {
                            warn!("parent {:?} died; exiting", parent_pid);
                            thread::sleep(PROCESS_CHECK_INTERVAL);
                            process::exit(1);
                        }

                        thread::sleep(PROCESS_CHECK_INTERVAL);
                    }
                }
            })
        };

        move || {
            *should_die.lock().unwrap() = true;
            thread_handle.join().unwrap();
        }
    };

    let arg_v = env::args()
        .map(|s| CString::new(s).unwrap())
        .collect::<Vec<_>>();
    let arg_c = arg_v.len() as c_int;

    let arg_v = arg_v.iter().map(|arg| arg.as_ptr()).collect::<Vec<_>>();

    let ret = unsafe { cef_interface_execute_process(arg_c, arg_v.as_ptr()) };
    warn!(?ret, "cef_interface_execute_process");

    #[cfg(target_os = "windows")]
    stop_parent_watcher();

    process::exit(ret);
}

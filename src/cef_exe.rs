#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::{env, ffi::CString, os::raw::c_int, process};

use classicube_cef_plugin::cef_interface_execute_process;
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

    let debug = true;
    let other_crates = false;
    let my_crate_name = module_path!();

    let level = if debug { "debug" } else { "info" };

    let mut filter = EnvFilter::from_default_env();
    if other_crates {
        filter = filter.add_directive(level.parse().unwrap());
    } else {
        filter = filter.add_directive(format!("{my_crate_name}={level}").parse().unwrap());
    }

    tracing_subscriber::fmt()
        .with_env_filter(filter)
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
    // on mac, i can't seem to reproduce the child process staying after `killall -9 ClassiCube`
    #[cfg(target_os = "windows")]
    {
        use std::thread;

        use windows::{
            core::Error,
            Win32::{
                Foundation::{CloseHandle, FALSE, HANDLE},
                System::{
                    Diagnostics::ToolHelp::{
                        CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
                        TH32CS_SNAPPROCESS,
                    },
                    Threading::{
                        GetCurrentProcessId, OpenProcess, WaitForSingleObject, INFINITE,
                        PROCESS_SYNCHRONIZE,
                    },
                },
            },
        };

        thread::spawn(move || {
            unsafe fn get_parent_handle() -> Result<(HANDLE, u32), Error> {
                let current_process_id = GetCurrentProcessId();

                let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
                let mut process_entry = PROCESSENTRY32 {
                    dwSize: core::mem::size_of::<PROCESSENTRY32>() as _,
                    ..Default::default()
                };

                Process32First(snapshot, &mut process_entry)?;
                loop {
                    if process_entry.th32ProcessID == current_process_id {
                        break;
                    }

                    Process32Next(snapshot, &mut process_entry)?;
                }
                CloseHandle(snapshot)?;

                Ok((
                    OpenProcess(
                        PROCESS_SYNCHRONIZE,
                        FALSE,
                        process_entry.th32ParentProcessID,
                    )?,
                    process_entry.th32ParentProcessID,
                ))
            }

            match unsafe { get_parent_handle() } {
                Ok((parent_handle, parent_pid)) => {
                    debug!(?parent_handle, parent_pid, "watching for parent to die");
                    let result = unsafe { WaitForSingleObject(parent_handle, INFINITE) };
                    warn!(?result, ?parent_handle, parent_pid, "parent died; exiting");
                    process::exit(1);
                }
                Err(e) => {
                    warn!(?e, "get_parent_handle");
                }
            }
        });
    }

    let arg_v = env::args()
        .map(|s| CString::new(s).unwrap())
        .collect::<Vec<_>>();
    let arg_c = arg_v.len() as c_int;

    let arg_v = arg_v.iter().map(|arg| arg.as_ptr()).collect::<Vec<_>>();

    let ret = unsafe { cef_interface_execute_process(arg_c, arg_v.as_ptr()) };
    warn!(?ret, "cef_interface_execute_process");

    process::exit(ret);
}

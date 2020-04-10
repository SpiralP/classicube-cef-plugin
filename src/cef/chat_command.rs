use super::{chat::print, CEF};
use classicube_sys::{Entities, ENTITIES_SELF_ID};
use std::{os::raw::c_int, slice};

pub extern "C" fn c_chat_command_callback(args: *const classicube_sys::String, args_count: c_int) {
    let args = unsafe { slice::from_raw_parts(args, args_count as _) };
    let args: Vec<String> = args.iter().map(|cc_string| cc_string.to_string()).collect();

    command_callback(args);
}

pub fn command_callback(args: Vec<String>) {
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    CEF.with(|cell| {
        if let Some(cef) = cell.borrow_mut().as_mut() {
            match args.as_slice() {
                ["here"] => {
                    if let Some(entity) = cef.entity.as_mut() {
                        let entity = entity.as_mut().project();
                        let entity = entity.entity;
                        unsafe {
                            let me = &*Entities.List[ENTITIES_SELF_ID as usize];
                            entity.Position.set(
                                me.Position.X - 4.0,
                                me.Position.Y + 4.0,
                                me.Position.Z,
                            );
                            print(format!("moved screen to {:?}", me.Position));
                        }
                    }
                }

                ["load", url] => {
                    cef.load((*url).to_string());
                }

                ["play", id] => {
                    cef.run_script(format!("player.loadVideoById(\"{}\");", id));
                }

                ["volume", percent] => {
                    // 0 to 100
                    cef.run_script(format!("player.setVolume({});", percent));
                }

                ["test"] => {
                    std::thread::spawn(move || {
                        use std::ffi::CString;

                        let code = format!("player.loadVideoById(\"{}\");", "gQngg8iQipk");
                        let c_str = CString::new(code).unwrap();
                        unsafe {
                            assert_eq!(crate::bindings::cef_run_script(c_str.as_ptr()), 0);
                        }
                    });
                }

                _ => {}
            }
        } else {
            print("Cef not initialized!");
        }
    });
}

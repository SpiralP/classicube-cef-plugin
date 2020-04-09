use super::CEF;
use crate::helpers::print;
use classicube_sys::{Entities, ENTITIES_SELF_ID};
use std::{os::raw::c_int, slice};

pub extern "C" fn c_chat_command_callback(args: *const classicube_sys::String, args_count: c_int) {
    let args = unsafe { slice::from_raw_parts(args, args_count as _) };
    let args: Vec<String> = args.iter().map(|cc_string| cc_string.to_string()).collect();

    command_callback(args);
}

fn command_callback(args: Vec<String>) {
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    match args.as_slice() {
        ["bap", arg] => {
            print((*arg).to_string());
        }

        ["meow"] => {
            print("yes");
        }

        ["here"] => {
            CEF.with(|cell| {
                if let Some(cef) = cell.borrow_mut().as_mut() {
                    if let Some(entity) = cef.entity.as_mut() {
                        let entity = entity.as_mut().project();
                        let entity = entity.entity;
                        unsafe {
                            let me = &*Entities.List[ENTITIES_SELF_ID as usize];
                            entity
                                .Position
                                .set(me.Position.X, me.Position.Y, me.Position.Z);
                            print(format!("moved screen to {:?}", me.Position));
                        }
                    }
                }
            });
        }

        _ => {}
    }

    print("meow");
}

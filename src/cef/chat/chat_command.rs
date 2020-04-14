use super::Chat;
use crate::{
    cef::{
        entity_manager::{CefEntityManager, ENTITIES},
        CEF,
    },
    helpers::WithInner,
};
use classicube_sys::{Entities, OwnedChatCommand, Vec3, ENTITIES_SELF_ID, MATH_DEG2RAD};
use std::{os::raw::c_int, pin::Pin, slice};

type Error = dyn std::error::Error;

extern "C" fn c_chat_command_callback(args: *const classicube_sys::String, args_count: c_int) {
    let args = unsafe { slice::from_raw_parts(args, args_count as _) };
    let args: Vec<String> = args.iter().map(|cc_string| cc_string.to_string()).collect();

    if let Err(e) = command_callback(args) {
        Chat::print(format!("cef command error: {}", e));
    }
}

pub fn command_callback(args: Vec<String>) -> Result<(), Box<Error>> {
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
    let me = unsafe { &*Entities.List[ENTITIES_SELF_ID as usize] };

    match args.as_slice() {
        ["create"] => {
            CEF.with_inner_mut(|cef| {
                cef.create_browser("https://www.classicube.net/".to_string());
            })
            .unwrap();

            Ok(())
        }

        ["create", url] => {
            CEF.with_inner_mut(|cef| {
                cef.create_browser((*url).to_string());
            })
            .unwrap();

            Ok(())
        }

        ["here", id] => {
            let id: c_int = id.parse()?;

            ENTITIES.with(|entities| {
                let entities = &mut *entities.borrow_mut();

                if let Some((_browser, entity)) = entities.get_mut(&id) {
                    let dir = Vec3::get_dir_vector(
                        me.Yaw * MATH_DEG2RAD as f32,
                        me.Pitch * MATH_DEG2RAD as f32,
                    );

                    entity.entity.Position.set(
                        me.Position.X + dir.X,
                        me.Position.Y + dir.Y,
                        me.Position.Z + dir.Z,
                    );

                    entity.entity.RotY = me.RotY;

                    Chat::print(format!(
                        "moved browser {} to {:?}",
                        id, entity.entity.Position
                    ));
                } else {
                    Chat::print(format!("no browser id {}", id));
                }

                Ok(())
            })
        }

        _ => ENTITIES.with(|entities| {
            let entities = &mut *entities.borrow_mut();

            if let Some((id, browser, entity)) =
                CefEntityManager::get_closest_mut(me.Position, entities)
            {
                let mut entity = entity.project();

                match args.as_slice() {
                    ["here"] => {
                        let dir = Vec3::get_dir_vector(
                            me.Yaw * MATH_DEG2RAD as f32,
                            me.Pitch * MATH_DEG2RAD as f32,
                        );

                        entity.entity.Position.set(
                            me.Position.X + dir.X,
                            me.Position.Y + dir.Y,
                            me.Position.Z + dir.Z,
                        );

                        entity.entity.RotY = me.RotY;

                        Chat::print(format!(
                            "moved browser {} to {:?}",
                            id, entity.entity.Position
                        ));
                    }

                    ["at", x, y, z] => {
                        let x = x.parse()?;
                        let y = y.parse()?;
                        let z = z.parse()?;

                        entity.entity.Position.set(x, y, z);

                        Chat::print(format!(
                            "moved browser {} to {:?}",
                            id, entity.entity.Position
                        ));
                    }

                    ["scale", scale] => {
                        let scale = scale.parse()?;

                        entity.set_scale(scale);

                        Chat::print(format!(
                            "scaled browser {} to {:?}",
                            id, entity.entity.ModelScale
                        ));
                    }

                    ["load", url] => {
                        browser.load_url((*url).to_string()).unwrap();
                    }

                    _ => {
                        Chat::print("NO");
                    }
                }
            } else {
                Chat::print("No browser to control!");
            }

            Ok(())
        }),
    }

    //

    //         // ["load", url] => {
    //         //     cef.load((*url).to_string());
    //         // }

    //         // ["play", id] => {
    //         //     cef.run_script(format!("player.loadVideoById(\"{}\");", id));
    //         // }

    //         // ["volume", percent] => {
    //         //     // 0 to 100
    //         //     cef.run_script(format!("player.setVolume({});", percent));
    //         // }

    //         // ["test"] => {
    //         //     std::thread::spawn(move || {
    //         //         use std::ffi::CString;

    //         //         let code = format!("player.loadVideoById(\"{}\");", "gQngg8iQipk");
    //         //         let c_str = CString::new(code).unwrap();
    //         //         unsafe {
    //         //             assert_eq!(crate::bindings::cef_run_script(c_str.as_ptr()), 0);
    //         //         }
    //         //     });
    //         // }
    //         _ => {}
    //     }
    // });

    // if result.is_none() {
    //     print("Cef not initialized!");
    // }
}

pub struct CefChatCommand {
    chat_command: Pin<Box<OwnedChatCommand>>,
}

impl CefChatCommand {
    pub fn new() -> Self {
        Self {
            chat_command: OwnedChatCommand::new("Cef", c_chat_command_callback, false, vec!["cef"]),
        }
    }

    pub fn initialize(&mut self) {
        self.chat_command.as_mut().register();
    }

    pub fn shutdown(&mut self) {
        //
    }
}

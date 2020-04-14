#![allow(clippy::single_match)]

use super::Chat;
use crate::{
    cef::{
        entity_manager::{CefEntity, CefEntityManager, ENTITIES},
        interface::RustRefBrowser,
        CEF,
    },
    error::*,
    helpers::WithInner,
};
use classicube_sys::{Entities, OwnedChatCommand, Vec3, ENTITIES_SELF_ID, MATH_DEG2RAD};
use error_chain::bail;
use std::{os::raw::c_int, slice};

extern "C" fn c_chat_command_callback(args: *const classicube_sys::String, args_count: c_int) {
    let args = unsafe { slice::from_raw_parts(args, args_count as _) };
    let args: Vec<String> = args.iter().map(|cc_string| cc_string.to_string()).collect();

    if let Err(e) = command_callback(args) {
        Chat::print(format!("cef command error: {}", e));
    }
}

fn with_closest<F, T>(f: F) -> Result<T>
where
    F: FnOnce(c_int, &mut RustRefBrowser, &mut CefEntity) -> Result<T>,
{
    let me = unsafe { &*Entities.List[ENTITIES_SELF_ID as usize] };

    ENTITIES.with(|entities| {
        let entities = &mut *entities.borrow_mut();

        if let Some((id, browser, entity)) =
            CefEntityManager::get_closest_mut(me.Position, entities)
        {
            f(id, browser, entity)
        } else {
            bail!("No browser to control!");
        }
    })
}

fn with_browser<F, T>(id: c_int, f: F) -> Result<T>
where
    F: FnOnce(c_int, &mut RustRefBrowser, &mut CefEntity) -> Result<T>,
{
    ENTITIES.with(|entities| {
        let entities = &mut *entities.borrow_mut();

        if let Some((browser, entity)) = entities.get_mut(&id) {
            f(id, browser, entity)
        } else {
            bail!("No browser id {}!", id);
        }
    })
}

pub fn command_callback(args: Vec<String>) -> Result<()> {
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
    let args: &[&str] = &args;
    let me = unsafe { &*Entities.List[ENTITIES_SELF_ID as usize] };

    // static commands not targetted at a specific entity
    match args {
        ["create"] => {
            CEF.with_inner_mut(|cef| {
                cef.create_browser("https://www.classicube.net/".to_string());
            })
            .chain_err(|| "CEF not initialized")?;
        }

        ["create", url] => {
            CEF.with_inner_mut(|cef| {
                cef.create_browser((*url).to_string());
            })
            .chain_err(|| "CEF not initialized")?;
        }

        _ => {}
    }

    // commands that target a certain entity/browser
    match args {
        ["here", id] => {
            let id: c_int = id.parse()?;

            with_browser(id, |id, _browser, entity| {
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

                Ok(())
            })?;
        }

        _ => {}
    }

    // commands that target a the closest entity/browser
    match args {
        ["here"] => with_closest(|id, _browser, entity| {
            let dir =
                Vec3::get_dir_vector(me.Yaw * MATH_DEG2RAD as f32, me.Pitch * MATH_DEG2RAD as f32);

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

            Ok(())
        })?,

        ["at", x, y, z] => with_closest(|id, _browser, entity| {
            let x = x.parse()?;
            let y = y.parse()?;
            let z = z.parse()?;

            entity.entity.Position.set(x, y, z);

            Chat::print(format!(
                "moved browser {} to {:?}",
                id, entity.entity.Position
            ));

            Ok(())
        })?,

        ["scale", scale] => with_closest(|id, _browser, entity| {
            let scale = scale.parse()?;

            entity.set_scale(scale);

            Chat::print(format!(
                "scaled browser {} to {:?}",
                id, entity.entity.ModelScale
            ));

            Ok(())
        })?,

        ["load", url] => {
            let closest_browser = with_closest(|_id, browser, _entity| Ok(browser.clone()))?;
            closest_browser.load_url((*url).to_string())?;
        }

        ["close"] => {
            let closest_browser = with_closest(|_id, browser, _entity| Ok(browser.clone()))?;
            closest_browser.close()?;
        }

        _ => {}
    }

    // ["play", id] => {
    //     cef.run_script(format!("player.loadVideoById(\"{}\");", id));
    // }

    // ["volume", percent] => {
    //     // 0 to 100
    //     cef.run_script(format!("player.setVolume({});", percent));
    // }

    Ok(())
}

pub struct CefChatCommand {
    chat_command: OwnedChatCommand,
}

impl CefChatCommand {
    pub fn new() -> Self {
        Self {
            chat_command: OwnedChatCommand::new("Cef", c_chat_command_callback, false, vec!["cef"]),
        }
    }

    pub fn initialize(&mut self) {
        self.chat_command.register();
    }

    pub fn shutdown(&mut self) {}
}

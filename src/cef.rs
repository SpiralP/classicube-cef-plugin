use crate::{helpers::*, interface::*, owned_entity::OwnedEntity, owned_model::*};
use classicube_helpers::{detour::*, tick::*};
use classicube_sys::*;
use std::{
    cell::RefCell,
    os::raw::{c_double, c_float, c_int},
    pin::Pin,
    ptr,
};

// Some means we are initialized
thread_local!(
    pub static CEF: RefCell<Option<Cef>> = RefCell::new(None);
);

fn print<S: Into<Vec<u8>>>(s: S) {
    let owned_string = OwnedString::new(s);
    unsafe {
        Chat_Add(owned_string.as_cc_string());
    }
}

pub fn initialize() {
    print("cef initialize");

    CEF.with(|option| {
        debug_assert!(option.borrow().is_none());

        *option.borrow_mut() = Some(Cef::new());
    });

    CEF.with(|option| {
        if let Some(cef) = &mut *option.borrow_mut() {
            cef.initialize();
        }
    });
}

pub fn shutdown() {
    print("cef shutdown");

    CEF.with(|option| {
        let mut cef = option.borrow_mut().take().unwrap();
        cef.shutdown();
    });
}

pub struct Cef {
    pub model: Option<Pin<Box<OwnedModel>>>,
    pub entity: Option<Pin<Box<OwnedEntity>>>,

    pub LocalPlayer_RenderModel_Detour:
        GenericDetour<unsafe extern "C" fn(*mut Entity, c_double, c_float)>,
    tick_handler: TickEventHandler,
    initialized: bool,
}

impl Cef {
    pub fn new() -> Self {
        let LocalPlayer_RenderModel_Detour = unsafe {
            let me = &*Entities.List[ENTITIES_SELF_ID as usize];
            let v_table = &*me.VTABLE;
            let target = v_table.RenderModel.unwrap();
            GenericDetour::new(
                target,
                Self::LocalPlayer_RenderModel_Hook
                    as unsafe extern "C" fn(*mut Entity, c_double, c_float),
            )
            .unwrap()
        };

        Self {
            model: None,
            entity: None,
            LocalPlayer_RenderModel_Detour,
            tick_handler: TickEventHandler::new(),
            initialized: false,
        }
    }

    extern "C" fn LocalPlayer_RenderModel_Hook(entity: *mut Entity, delta: c_double, t: c_float) {
        CEF.with(|option| {
            if let Some(cef) = &mut *option.borrow_mut() {
                unsafe {
                    cef.LocalPlayer_RenderModel_Detour.call(entity, delta, t);

                    let entity = cef.entity.as_mut().unwrap();
                    let entity = entity.as_mut().project();
                    let entity = entity.entity;

                    // Entities.List[i]->VTABLE->RenderModel(Entities.List[i], delta, t);
                    let v_table = &*entity.VTABLE;
                    let RenderModel = v_table.RenderModel.unwrap();
                    RenderModel(entity, delta, t);
                }
            }
        });
    }

    pub fn initialize(&mut self) {
        unsafe {
            self.model = Some(OwnedModel::register("cef", "cef"));
            self.entity = Some(OwnedEntity::register());

            self.LocalPlayer_RenderModel_Detour.enable().unwrap();
        }

        unsafe {
            assert_eq!(cef_init(Some(Self::on_paint_callback)), 0);
        }

        self.tick_handler.on(|_task| {
            //
            unsafe {
                assert_eq!(cef_step(), 0);
            }
        });

        self.initialized = true;
    }

    pub fn shutdown(&mut self) {
        if self.initialized {
            self.model.take();

            unsafe {
                assert_eq!(cef_free(), 0);
            }
            self.initialized = false;
        }
    }

    fn paint(
        &mut self,
        new_pixels: *const ::std::os::raw::c_void,
        new_width: ::std::os::raw::c_int,
        new_height: ::std::os::raw::c_int,
    ) {
        if let Some(model) = self.model.as_mut() {
            let mut part = Bitmap {
                Scan0: new_pixels as *mut _,
                Width: new_width as i32,
                Height: new_height as i32,
            };

            unsafe {
                println!("cef paint");
                let texture = model.texture.as_ref().unwrap();
                Gfx_UpdateTexturePart(texture.resource_id, 0, 0, &mut part, 0);
            }
        }
    }

    extern "C" fn on_paint_callback(
        new_pixels: *const ::std::os::raw::c_void,
        new_width: ::std::os::raw::c_int,
        new_height: ::std::os::raw::c_int,
    ) {
        CEF.with(|option| {
            if let Some(cef) = &mut *option.borrow_mut() {
                cef.paint(new_pixels, new_width, new_height);
            }
        });
    }
}

impl Drop for Cef {
    fn drop(&mut self) {
        self.shutdown();
    }
}

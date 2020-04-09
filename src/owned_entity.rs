use crate::helpers::*;
use classicube_sys::*;
use pin_project::{pin_project, project};
use std::{ffi::CString, mem, pin::Pin};

#[pin_project]
pub struct OwnedEntity {
    pub entity: Entity,

    #[pin]
    v_table: EntityVTABLE,
}

impl OwnedEntity {
    pub unsafe fn register() -> Pin<Box<Self>> {
        let entity = mem::zeroed();

        let v_table = EntityVTABLE {
            Tick: Some(Self::Tick),
            Despawn: Some(Self::Despawn),
            SetLocation: Some(Self::SetLocation),
            GetCol: Some(Self::GetCol),
            RenderModel: Some(Self::RenderModel),
            RenderName: Some(Self::RenderName),
        };

        let mut this = Box::pin(Self { entity, v_table });

        // this.as_mut().project().register_gfx_texture();
        // this.as_mut().project().register_texture();
        // this.as_mut().project().register_model();
        this.as_mut().project().register_entity();

        this
    }

    unsafe extern "C" fn Tick(entity: *mut Entity, delta: f64) {
        println!("Tick");
    }

    unsafe extern "C" fn Despawn(entity: *mut Entity) {
        println!("Despawn");
    }

    unsafe extern "C" fn SetLocation(
        entity: *mut Entity,
        update: *mut LocationUpdate,
        interpolate: cc_bool,
    ) {
        println!("SetLocation");
    }

    unsafe extern "C" fn GetCol(entity: *mut Entity) -> PackedCol {
        println!("GetCol");

        PACKEDCOL_WHITE
    }

    unsafe extern "C" fn RenderModel(entity: *mut Entity, deltaTime: f64, t: f32) {
        let entity = &mut *entity;

        println!("RenderModel");

        entity.Position.set(0.0, 40.0, 0.0);
        entity.RotX = 180.0;

        Model_Render(entity.Model, entity);
    }

    unsafe extern "C" fn RenderName(entity: *mut Entity) {
        println!("RenderName");
    }
}

#[project]
impl OwnedEntity {
    #[project]
    unsafe fn register_entity(&mut self) {
        #[project]
        let OwnedEntity {
            entity, v_table, ..
        } = self;

        Entity_Init(entity);

        let model_name = OwnedString::new("cef");
        Entity_SetModel(*entity, model_name.as_cc_string());

        entity.VTABLE = v_table.as_mut().get_unchecked_mut();
        entity.Velocity.set(0.0, 0.0, 0.0);

        // entity.DisplayNameRaw =
    }
}

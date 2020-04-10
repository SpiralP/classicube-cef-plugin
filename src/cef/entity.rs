use classicube_sys::*;
use pin_project::{pin_project, project};
use std::{mem, pin::Pin};

#[pin_project]
pub struct CefEntity {
    pub entity: Entity,

    #[pin]
    v_table: EntityVTABLE,
}

impl CefEntity {
    pub unsafe fn register() -> Pin<Box<Self>> {
        let entity = mem::zeroed();

        let v_table = EntityVTABLE {
            Tick: Some(Self::tick),
            Despawn: Some(Self::despawn),
            SetLocation: Some(Self::set_location),
            GetCol: Some(Self::get_col),
            RenderModel: Some(Self::render_model),
            RenderName: Some(Self::render_name),
        };

        let mut this = Box::pin(Self { entity, v_table });

        // this.as_mut().project().register_gfx_texture();
        // this.as_mut().project().register_texture();
        // this.as_mut().project().register_model();
        this.as_mut().project().register_entity();

        this
    }

    unsafe extern "C" fn tick(_entity: *mut Entity, _delta: f64) {
        println!("Tick");
    }

    unsafe extern "C" fn despawn(_entity: *mut Entity) {
        println!("Despawn");
    }

    unsafe extern "C" fn set_location(
        _entity: *mut Entity,
        _update: *mut LocationUpdate,
        _interpolate: cc_bool,
    ) {
        println!("SetLocation");
    }

    unsafe extern "C" fn get_col(_entity: *mut Entity) -> PackedCol {
        // println!("GetCol");

        PACKEDCOL_WHITE
    }

    unsafe extern "C" fn render_model(entity: *mut Entity, _delta_time: f64, _t: f32) {
        let entity = &mut *entity;

        // println!("RenderModel");

        Model_Render(entity.Model, entity);
    }

    unsafe extern "C" fn render_name(_entity: *mut Entity) {
        println!("RenderName");
    }
}

#[project]
impl CefEntity {
    #[project]
    unsafe fn register_entity(&mut self) {
        #[project]
        let CefEntity {
            entity, v_table, ..
        } = self;

        Entity_Init(entity);

        let model_name = OwnedString::new("cef");
        Entity_SetModel(*entity, model_name.as_cc_string());

        entity.VTABLE = v_table.as_mut().get_unchecked_mut();
        entity.Velocity.set(0.0, 0.0, 0.0);

        entity.Position.set(64.0 - 4.0, 48.0, 64.0);

        entity.RotX = 180.0;

        // entity.DisplayNameRaw =
    }
}

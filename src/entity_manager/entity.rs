use super::{TEXTURE_HEIGHT, TEXTURE_WIDTH};
use crate::{
    cef::RustRefBrowser,
    entity_manager::{MODEL_HEIGHT, MODEL_WIDTH},
    players::Player,
};
use classicube_sys::{
    cc_bool, cc_int16, Bitmap, Entity, EntityVTABLE, Entity_Init, Entity_SetModel,
    Gfx_UpdateTexturePart, LocationUpdate, Model_Render, OwnedGfxTexture, OwnedString, PackedCol,
    Texture, TextureRec, PACKEDCOL_WHITE,
};
use std::{mem, pin::Pin};

pub struct CefEntity {
    pub id: usize,

    pub entity: Pin<Box<Entity>>,
    pub browser: Option<RustRefBrowser>,
    pub player: Player,

    v_table: Pin<Box<EntityVTABLE>>,
    texture: OwnedGfxTexture,

    browser_attached_callbacks: Vec<Box<dyn FnOnce(RustRefBrowser)>>,
}

impl CefEntity {
    pub fn register(id: usize, player: Player) -> Self {
        let entity = Box::pin(unsafe { mem::zeroed() });

        let v_table = Box::pin(EntityVTABLE {
            Tick: Some(Self::tick),
            Despawn: Some(Self::despawn),
            SetLocation: Some(Self::set_location),
            GetCol: Some(Self::get_col),
            RenderModel: Some(Self::c_render_model),
            RenderName: Some(Self::render_name),
        });

        let mut pixels: Vec<u8> = vec![255; 4 * TEXTURE_WIDTH * TEXTURE_HEIGHT];

        let mut bmp = Bitmap {
            Scan0: pixels.as_mut_ptr(),
            Width: TEXTURE_WIDTH as i32,
            Height: TEXTURE_HEIGHT as i32,
        };

        let texture = OwnedGfxTexture::create(&mut bmp, true, false);

        let mut this = Self {
            id,
            entity,
            v_table,
            texture,
            browser: None,
            player,
            browser_attached_callbacks: Vec::new(),
        };

        unsafe {
            this.register_entity();
        }

        this.set_scale(0.25);

        this
    }

    unsafe extern "C" fn tick(_entity: *mut Entity, _delta: f64) {}

    unsafe extern "C" fn despawn(_entity: *mut Entity) {}

    unsafe extern "C" fn set_location(
        _entity: *mut Entity,
        _update: *mut LocationUpdate,
        _interpolate: cc_bool,
    ) {
    }

    unsafe extern "C" fn get_col(_entity: *mut Entity) -> PackedCol {
        PACKEDCOL_WHITE
    }

    unsafe extern "C" fn c_render_model(_entity: *mut Entity, _delta_time: f64, _t: f32) {
        // we use the render_model function below directly instead
    }

    unsafe extern "C" fn render_name(_entity: *mut Entity) {}

    unsafe fn register_entity(&mut self) {
        let CefEntity {
            entity,
            v_table,
            texture,
            ..
        } = self;

        Entity_Init(entity);

        let model_name = OwnedString::new("cef");
        Entity_SetModel(
            entity.as_mut().get_unchecked_mut(),
            model_name.as_cc_string(),
        );

        entity.VTABLE = v_table.as_mut().get_unchecked_mut();
        entity.Velocity.set(0.0, 0.0, 0.0);
        entity.RotZ = 180.0;
        entity.TextureId = texture.resource_id;

        entity.Position.set(0.0, 0.0, 0.0);

        // hack so that Model can see browser resolution sizes
        // that are updated in update_texture
        entity.NameTex = Texture {
            ID: entity.TextureId,
            X: -(MODEL_WIDTH as cc_int16 / 2),
            Y: -(MODEL_HEIGHT as cc_int16),
            Width: MODEL_WIDTH as _,
            Height: MODEL_HEIGHT as _,
            uv: TextureRec {
                U1: 0.0,
                V1: 0.0,
                U2: 1.0,
                V2: 1.0,
            },
        };
    }

    pub fn update_texture(&mut self, mut part: Bitmap) {
        // update uv's
        self.entity.NameTex.uv.U2 = part.Width as f32 / TEXTURE_WIDTH as f32;
        self.entity.NameTex.uv.V2 = part.Height as f32 / TEXTURE_HEIGHT as f32;

        unsafe {
            Gfx_UpdateTexturePart(self.texture.resource_id, 0, 0, &mut part, 0);
        }
    }

    pub fn render_model(&mut self) {
        if self.get_scale() != 0.0 {
            let entity = self.entity.as_mut();
            unsafe {
                Model_Render(entity.Model, entity.get_unchecked_mut());
            }
        }
    }

    pub fn set_scale(&mut self, scale: f32) {
        let CefEntity { entity, .. } = self;

        // TODO make 1.0 be 1 block wide
        entity.ModelScale.set(scale, scale, 1.0);
    }

    pub fn get_scale(&self) -> f32 {
        let CefEntity { entity, .. } = self;
        entity.ModelScale.X
    }

    pub fn on_browser_attached<F: 'static>(&mut self, f: F)
    where
        F: FnOnce(RustRefBrowser),
    {
        self.browser_attached_callbacks.push(Box::new(f));
    }

    pub fn attach_browser(&mut self, browser: RustRefBrowser) {
        self.browser = Some(browser.clone());

        for callback in self.browser_attached_callbacks.drain(..) {
            callback(browser.clone());
        }
    }
}

use super::{helpers::Texture_RenderShaded, TEXTURE_HEIGHT, TEXTURE_WIDTH};
use classicube_sys::{
    Bitmap, Entity, Gfx_SetAlphaTest, Gfx_SetTexturing, Model, ModelTex, ModelVertex, Model_Init,
    Model_Register, OwnedGfxTexture, PackedCol, PackedCol_Make, SKIN_TYPE_SKIN_64x64,
    MODEL_BOX_VERTICES,
};
use std::{ffi::CString, mem, pin::Pin};

const WHITE: PackedCol = PackedCol_Make(255, 255, 255, 255);

pub struct CefModel {
    name: Pin<Box<CString>>,
    default_texture_name: Pin<Box<CString>>,
    model: Pin<Box<Model>>,
    vertices: Pin<Box<[ModelVertex; MODEL_BOX_VERTICES as usize]>>,
    default_model_tex: Pin<Box<ModelTex>>,

    default_texture: Option<OwnedGfxTexture>,
}

impl CefModel {
    pub fn register() -> Self {
        let name = "cef";
        let default_texture_name = format!("{}_texture", name);

        let model = Box::pin(unsafe { mem::zeroed() });
        let name = Box::pin(CString::new(name).unwrap());
        let default_texture_name = Box::pin(CString::new(default_texture_name).unwrap());
        let vertices = Box::pin(unsafe { mem::zeroed() });
        let default_model_tex = Box::pin(unsafe { mem::zeroed() });

        let mut this = Self {
            model,
            name,
            default_texture_name,
            vertices,
            default_model_tex,
            default_texture: None,
        };

        unsafe {
            this.register_gfx_texture();
            this.register_texture();
            this.register_model();
        }

        this
    }
}

impl CefModel {
    unsafe fn register_gfx_texture(&mut self) {
        // must be a vec or else we try to fit huge array onto stack and crash!
        let mut pixels: Vec<u32> =
            vec![0xFFFF_FFFF; TEXTURE_WIDTH as usize * TEXTURE_HEIGHT as usize];

        let mut bmp = Bitmap {
            scan0: pixels.as_mut_ptr(),
            width: i32::from(TEXTURE_WIDTH),
            height: i32::from(TEXTURE_HEIGHT),
        };

        let default_texture = OwnedGfxTexture::create(&mut bmp, true, false);

        self.default_texture = Some(default_texture);
    }

    unsafe fn register_texture(&mut self) {
        let CefModel {
            default_model_tex,
            default_texture_name,
            ..
        } = self;

        default_model_tex.name = default_texture_name.as_ptr();
        default_model_tex.skinType = SKIN_TYPE_SKIN_64x64 as _;
        default_model_tex.texID = self.default_texture.as_mut().unwrap().resource_id;

        // we don't need to register our texture!
        // Model_RegisterTexture(default_model_tex.as_mut().get_unchecked_mut());
    }

    extern "C" fn draw(entity: *mut Entity) {
        let entity = unsafe { &mut *entity };

        unsafe {
            Gfx_SetAlphaTest(1);
            // Gfx_SetAlphaBlending(1);
            // Gfx_SetAlphaArgBlend(1);
            Gfx_SetTexturing(1);

            Texture_RenderShaded(&mut entity.NameTex, WHITE);
        }
    }

    unsafe fn register_model(&mut self) {
        let CefModel {
            default_model_tex,
            model,
            vertices,
            name,
            ..
        } = self;

        model.name = name.as_ptr();
        model.vertices = vertices.as_mut_ptr();
        model.defaultTex = default_model_tex.as_mut().get_unchecked_mut();

        extern "C" fn make_parts() {}
        model.MakeParts = Some(make_parts);

        model.Draw = Some(Self::draw);

        extern "C" fn get_name_y(_entity: *mut Entity) -> f32 {
            0.0
        }
        model.GetNameY = Some(get_name_y);

        extern "C" fn get_eye_y(_entity: *mut Entity) -> f32 {
            0.0
        }
        model.GetEyeY = Some(get_eye_y);

        extern "C" fn get_collision_size(_entity: *mut Entity) {}
        model.GetCollisionSize = Some(get_collision_size);

        extern "C" fn get_picking_bounds(_entity: *mut Entity) {}
        model.GetPickingBounds = Some(get_picking_bounds);

        Model_Init(model.as_mut().get_unchecked_mut());

        model.bobbing = 0;

        Model_Register(model.as_mut().get_unchecked_mut());
    }
}

use std::{
    ffi::{CStr, CString},
    mem, ptr,
};

use classicube_sys::{
    Entity, Gfx_SetAlphaTest, Gfx_SetTexturing, MODEL_BOX_VERTICES, Model, Model_Init,
    Model_Register, ModelTex, ModelVertex, PackedCol, PackedCol_Make, SKIN_TYPE_SKIN_64x64,
};

use super::helpers::Texture_RenderShaded;

const WHITE: PackedCol = PackedCol_Make(255, 255, 255, 255);

pub struct CefModel {
    name: Box<CStr>,
    default_texture_name: Box<CStr>,
    model: Box<Model>,
    vertices: Box<[ModelVertex; MODEL_BOX_VERTICES as usize]>,
    default_model_tex: Box<ModelTex>,
}

impl CefModel {
    pub fn register() -> Self {
        let name = "cef";
        let default_texture_name = format!("{name}_texture");

        let model = Box::new(unsafe { mem::zeroed() });
        let name = CString::new(name).unwrap().into_boxed_c_str();
        let default_texture_name = CString::new(default_texture_name)
            .unwrap()
            .into_boxed_c_str();
        let vertices = Box::new(unsafe { mem::zeroed() });
        let default_model_tex = Box::new(unsafe { mem::zeroed() });

        let mut this = Self {
            name,
            default_texture_name,
            model,
            vertices,
            default_model_tex,
        };

        this.register_texture();
        this.register_model();

        this
    }
}

impl CefModel {
    fn register_texture(&mut self) {
        let CefModel {
            default_model_tex,
            default_texture_name,
            ..
        } = self;

        default_model_tex.name = default_texture_name.as_ptr();
        default_model_tex.skinType = SKIN_TYPE_SKIN_64x64 as _;
        // No backing GPU texture: our `draw` renders each entity's own
        // `NameTex` via Texture_RenderShaded and never calls
        // Model_ApplyTexture, so `defaultTex->texID` is never sampled.
        // Allocating one here would leak a D3D9 resource (and keep the
        // device alive) because `MODEL` is intentionally never dropped -
        // ClassiCube has no Model_Unregister - so the texture could never
        // be freed before Gfx_Free. A null texID binds no texture, which is
        // safe.
        default_model_tex.texID = ptr::null_mut();

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

    fn register_model(&mut self) {
        let CefModel {
            default_model_tex,
            model,
            vertices,
            name,
            ..
        } = self;

        model.name = name.as_ptr();
        model.vertices = vertices.as_mut_ptr();
        model.defaultTex = default_model_tex.as_mut();

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

        unsafe {
            Model_Init(model.as_mut());
        }

        model.bobbing = 0;

        unsafe {
            Model_Register(model.as_mut());
        }
    }
}

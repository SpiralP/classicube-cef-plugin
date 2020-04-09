use crate::{cef::CEF, helpers::*};
use classicube_sys::*;
use pin_project::{pin_project, project};
use std::{ffi::CString, mem, pin::Pin};

const WIDTH: usize = 512;
const HEIGHT: usize = 512;

static mut PIXELS: [u8; 4 * WIDTH * HEIGHT] = [255; 4 * WIDTH * HEIGHT];

static mut BMP: Bitmap = Bitmap {
    Scan0: unsafe { &mut PIXELS as *mut _ as *mut u8 },
    Width: WIDTH as i32,
    Height: HEIGHT as i32,
};

#[pin_project]
pub struct OwnedModel {
    #[pin]
    name: CString,
    #[pin]
    texture_name: CString,
    #[pin]
    model: Model,
    #[pin]
    vertices: [ModelVertex; MODEL_BOX_VERTICES as usize],
    #[pin]
    model_tex: ModelTex,

    pub texture: Option<OwnedGfxTexture>,
}

impl OwnedModel {
    pub unsafe fn register<S: Into<Vec<u8>>, S2: Into<Vec<u8>>>(
        name: S,
        texture_name: S2,
    ) -> Pin<Box<Self>> {
        let model = mem::zeroed();
        let name = CString::new(name).unwrap();
        let texture_name = CString::new(texture_name).unwrap();
        let vertices = mem::zeroed();
        let model_tex = mem::zeroed();

        let mut this = Box::pin(Self {
            model,
            name,
            texture_name,
            vertices,
            model_tex,
            texture: None,
        });

        this.as_mut().project().register_gfx_texture();
        this.as_mut().project().register_texture();
        this.as_mut().project().register_model();

        this
    }
}

#[project]
impl OwnedModel {
    #[project]
    unsafe fn register_gfx_texture(&mut self) {
        let texture = OwnedGfxTexture::create(&mut BMP, true, false);

        *self.texture = Some(texture);
    }

    #[project]
    unsafe fn register_texture(&mut self) {
        #[project]
        let OwnedModel {
            model_tex,
            texture_name,
            ..
        } = self;

        model_tex.name = texture_name.as_ptr();
        // self.model_tex.skinType = SKIN_TYPE_SKIN_64x32 as _;
        model_tex.texID = self.texture.as_mut().expect("self.texture").resource_id;
        // self.model_tex.next =  *mut ModelTex;

        Model_RegisterTexture(model_tex.as_mut().get_unchecked_mut());
    }

    #[project]
    fn draw_model(&mut self, _entity: *mut Entity) {
        let texture = self.texture.as_ref().unwrap();

        println!("draw");

        let mut tex = Texture {
            ID: texture.resource_id,
            X: 0,
            Y: 0,
            Width: 16,
            Height: 9,
            uv: TextureRec {
                U1: 0.0,
                V1: 0.0,
                U2: 1.0,
                V2: 1.0,
            },
        };

        unsafe {
            // Gfx_Draw2DFlat(0, 0, 64, 64, col);
            Texture_Render(&mut tex);
        }
    }

    unsafe extern "C" fn draw(entity: *mut Entity) {
        CEF.with(|option| {
            if let Some(cef) = &mut *option.borrow_mut() {
                let model = cef.model.as_mut().unwrap();
                let mut model = model.as_mut().project();
                model.draw_model(entity);
            }
        });
    }

    #[project]
    unsafe fn register_model(&mut self) {
        #[project]
        let OwnedModel {
            model_tex,
            model,
            vertices,
            name,
            ..
        } = self;

        model.name = name.as_ptr();
        model.vertices = vertices.as_mut_ptr();
        model.defaultTex = model_tex.as_mut().get_unchecked_mut();

        extern "C" fn make_parts() {
            //
        }
        model.MakeParts = Some(make_parts);

        model.Draw = Some(Self::draw);

        extern "C" fn get_name_y(_entity: *mut Entity) -> f32 {
            //
            0.0
        }
        model.GetNameY = Some(get_name_y);

        extern "C" fn get_eye_y(_entity: *mut Entity) -> f32 {
            //
            0.0
        }
        model.GetEyeY = Some(get_eye_y);

        extern "C" fn get_collision_size(_entity: *mut Entity) {
            //
        }
        model.GetCollisionSize = Some(get_collision_size);

        extern "C" fn get_picking_bounds(_entity: *mut Entity) {
            //
        }
        model.GetPickingBounds = Some(get_picking_bounds);

        Model_Init(model.as_mut().get_unchecked_mut());

        model.bobbing = 0;

        println!("Model_Register {:#?}", model);
        Model_Register(model.as_mut().get_unchecked_mut());
    }
}

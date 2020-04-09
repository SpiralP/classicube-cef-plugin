#![allow(non_snake_case)]

use classicube_sys::*;
use std::{
    cell::RefCell,
    os::raw::{c_int, c_void},
};

pub fn print<S: Into<Vec<u8>>>(s: S) {
    let owned_string = OwnedString::new(s);
    unsafe {
        Chat_Add(owned_string.as_cc_string());
    }
}

pub unsafe fn Entity_Init(e: &mut Entity) {
    let model = OwnedString::new("humanoid");
    e.ModelScale.set(1.0, 1.0, 1.0);
    e.uScale = 1.0;
    e.vScale = 1.0;
    e.StepSize = 0.5;
    e.SkinNameRaw[0] = 0;
    e.DisplayNameRaw[0] = 0;
    Entity_SetModel(e, model.as_cc_string());
}

// Gfx_quadVb = Gfx_CreateDynamicVb(VERTEX_FORMAT_P3FC4B, 4);
// Gfx_texVb  = Gfx_CreateDynamicVb(VERTEX_FORMAT_P3FT2FC4B, 4);

thread_local!(
    pub static QUAD_VB: RefCell<Option<OwnedGfxVertexBuffer>> = RefCell::new(None);
);

thread_local!(
    pub static TEX_VB: RefCell<Option<OwnedGfxVertexBuffer>> = RefCell::new(None);
);

pub unsafe fn Gfx_Draw2DFlat(x: c_int, y: c_int, width: c_int, height: c_int, col: PackedCol) {
    let mut verts = [
        VertexP3fC4b {
            X: x as _,
            Y: y as _,
            Z: 0 as _,
            Col: col as _,
        },
        VertexP3fC4b {
            X: (x + width) as _,
            Y: y as _,
            Z: 0 as _,
            Col: col as _,
        },
        VertexP3fC4b {
            X: (x + width) as _,
            Y: (y + height) as _,
            Z: 0 as _,
            Col: col as _,
        },
        VertexP3fC4b {
            X: x as _,
            Y: (y + height) as _,
            Z: 0 as _,
            Col: col as _,
        },
    ];

    Gfx_SetVertexFormat(VertexFormat__VERTEX_FORMAT_P3FC4B);
    QUAD_VB.with(|quad_vb| {
        let quad_vb = quad_vb.borrow_mut();
        let quad_vb = quad_vb.as_ref().unwrap();
        Gfx_UpdateDynamicVb_IndexedTris(quad_vb.resource_id, verts.as_mut_ptr() as _, 4);
    });
}

pub unsafe fn Gfx_UpdateDynamicVb_IndexedTris(
    vb: GfxResourceID,
    vertices: *mut c_void,
    vCount: c_int,
) {
    Gfx_SetDynamicVbData(vb, vertices, vCount);
    Gfx_DrawVb_IndexedTris(vCount);
}

pub unsafe fn Gfx_Draw2DTexture(tex: &mut Texture, col: PackedCol) {
    let mut vertices = Gfx_Make2DQuad(tex, col);
    Gfx_SetVertexFormat(VertexFormat__VERTEX_FORMAT_P3FT2FC4B);
    QUAD_VB.with(|tex_vb| {
        let tex_vb = tex_vb.borrow_mut();
        let tex_vb = tex_vb.as_ref().unwrap();
        Gfx_UpdateDynamicVb_IndexedTris(tex_vb.resource_id, vertices.as_mut_ptr() as _, 4);
    });
}

pub unsafe fn Gfx_Make2DQuad(tex: &mut Texture, col: PackedCol) -> [VertexP3fT2fC4b; 4] {
    let mut x1: f32 = tex.X as _;
    let mut x2: f32 = (tex.X as f32 + tex.Width as f32) as _;
    let mut y1: f32 = tex.Y as _;
    let mut y2: f32 = (tex.Y as f32 + tex.Height as f32) as _;
    // VertexP3fT2fC4b* v = *vertices;

    // #ifdef CC_BUILD_D3D9
    // NOTE: see "https://msdn.microsoft.com/en-us/library/windows/desktop/bb219690(v=vs.85).aspx",
    // i.e. the msdn article called "Directly Mapping Texels to Pixels (Direct3D 9)" for why we have to do this.
    x1 -= 0.5;
    x2 -= 0.5;
    y1 -= 0.5;
    y2 -= 0.5;
    // #endif

    [
        VertexP3fT2fC4b {
            X: x1,
            Y: y1,
            Z: 0 as _,
            Col: col,
            U: tex.uv.U1,
            V: tex.uv.V1,
        },
        VertexP3fT2fC4b {
            X: x2,
            Y: y1,
            Z: 0 as _,
            Col: col,
            U: tex.uv.U2,
            V: tex.uv.V1,
        },
        VertexP3fT2fC4b {
            X: x2,
            Y: y2,
            Z: 0 as _,
            Col: col,
            U: tex.uv.U2,
            V: tex.uv.V2,
        },
        VertexP3fT2fC4b {
            X: x1,
            Y: y2,
            Z: 0 as _,
            Col: col,
            U: tex.uv.U1,
            V: tex.uv.V2,
        },
    ]
}

pub unsafe fn Texture_Render(tex: &mut Texture) {
    let white = PACKEDCOL_WHITE;
    Gfx_BindTexture(tex.ID);
    Gfx_Draw2DTexture(tex, white);
}

pub unsafe fn Texture_RenderShaded(tex: &mut Texture, shadeCol: PackedCol) {
    Gfx_BindTexture(tex.ID);
    Gfx_Draw2DTexture(tex, shadeCol);
}

pub struct OwnedGfxTexture {
    pub resource_id: GfxResourceID,
}

impl OwnedGfxTexture {
    pub fn create(bmp: &mut Bitmap, managed_pool: bool, mipmaps: bool) -> Self {
        let resource_id = unsafe {
            Gfx_CreateTexture(
                bmp,
                if managed_pool { 1 } else { 0 },
                if mipmaps { 1 } else { 0 },
            )
        };
        println!("Gfx_CreateTexture {:#?}", resource_id);

        assert!(!resource_id.is_null());

        Self { resource_id }
    }
}

impl Drop for OwnedGfxTexture {
    fn drop(&mut self) {
        println!("Gfx_DeleteTexture {:#?}", self.resource_id);
        unsafe {
            Gfx_DeleteTexture(&mut self.resource_id);
        }
    }
}

pub struct OwnedGfxVertexBuffer {
    pub resource_id: GfxResourceID,
}

impl OwnedGfxVertexBuffer {
    pub fn create(fmt: VertexFormat, max_vertices: ::std::os::raw::c_int) -> Self {
        let resource_id = unsafe { Gfx_CreateDynamicVb(fmt, max_vertices) };
        println!("Gfx_CreateVertexBuffer {:#?}", resource_id);

        assert!(!resource_id.is_null());

        Self { resource_id }
    }
}

impl Drop for OwnedGfxVertexBuffer {
    fn drop(&mut self) {
        println!("Gfx_DeleteVb {:#?}", self.resource_id);
        unsafe {
            Gfx_DeleteVb(&mut self.resource_id);
        }
    }
}

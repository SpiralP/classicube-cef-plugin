#![allow(non_snake_case)]

use classicube_sys::*;
use std::cell::RefCell;

// Gfx_quadVb = Gfx_CreateDynamicVb(VERTEX_FORMAT_P3FC4B, 4);
// Gfx_texVb  = Gfx_CreateDynamicVb(VERTEX_FORMAT_P3FT2FC4B, 4);

thread_local!(
    pub static QUAD_VB: RefCell<Option<OwnedGfxVertexBuffer>> = RefCell::new(None);
);

thread_local!(
    pub static TEX_VB: RefCell<Option<OwnedGfxVertexBuffer>> = RefCell::new(None);
);

// pub unsafe fn Gfx_Draw2DFlat(x: c_int, y: c_int, width: c_int, height: c_int, col: PackedCol) {
//     let mut verts = [
//         VertexP3fC4b {
//             X: x as _,
//             Y: y as _,
//             Z: 0 as _,
//             Col: col as _,
//         },
//         VertexP3fC4b {
//             X: (x + width) as _,
//             Y: y as _,
//             Z: 0 as _,
//             Col: col as _,
//         },
//         VertexP3fC4b {
//             X: (x + width) as _,
//             Y: (y + height) as _,
//             Z: 0 as _,
//             Col: col as _,
//         },
//         VertexP3fC4b {
//             X: x as _,
//             Y: (y + height) as _,
//             Z: 0 as _,
//             Col: col as _,
//         },
//     ];

//     Gfx_SetVertexFormat(VertexFormat__VERTEX_FORMAT_P3FC4B);
//     QUAD_VB.with(|quad_vb| {
//         let quad_vb = quad_vb.borrow_mut();
//         let quad_vb = quad_vb.as_ref().unwrap();
//         Gfx_UpdateDynamicVb_IndexedTris(quad_vb.resource_id, verts.as_mut_ptr() as _, 4);
//     });
// }

// pub unsafe fn Texture_Render(tex: &mut Texture) {
//     let white = PACKEDCOL_WHITE;
//     Gfx_BindTexture(tex.ID);
//     Gfx_Draw2DTexture(tex, white);
// }

pub unsafe fn Gfx_Draw2DTexture(tex: &mut Texture, col: PackedCol) {
    let mut vertices = Gfx_Make2DQuad(tex, col);

    Gfx_SetVertexFormat(VertexFormat__VERTEX_FORMAT_P3FT2FC4B);
    TEX_VB.with(|tex_vb| {
        let tex_vb = tex_vb.borrow_mut();
        let tex_vb = tex_vb.as_ref().unwrap();
        Gfx_UpdateDynamicVb_IndexedTris(tex_vb.resource_id, vertices.as_mut_ptr() as _, 4);
    });
}

pub unsafe fn Texture_RenderShaded(tex: &mut Texture, shadeCol: PackedCol) {
    Gfx_BindTexture(tex.ID);
    Gfx_Draw2DTexture(tex, shadeCol);
}

use std::{cell::Cell, thread::LocalKey};

pub trait ThreadLocalGetSet<T> {
    fn get(&'static self) -> T;
    fn set(&'static self, value: T);
}

impl<T> ThreadLocalGetSet<T> for LocalKey<Cell<T>>
where
    T: Copy,
{
    fn get(&'static self) -> T {
        self.with(|cell| cell.get())
    }

    fn set(&'static self, value: T) {
        self.with(|cell| cell.set(value))
    }
}

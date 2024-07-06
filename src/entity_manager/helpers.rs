#![allow(non_snake_case)]

use std::cell::RefCell;

use classicube_sys::{
    Gfx_BindTexture, Gfx_Make2DQuad, Gfx_SetVertexFormat, Gfx_UpdateDynamicVb_IndexedTris,
    OwnedGfxVertexBuffer, PackedCol, Texture, VertexFormat__VERTEX_FORMAT_TEXTURED,
};

thread_local!(
    pub static TEX_VB: RefCell<Option<OwnedGfxVertexBuffer>> = const { RefCell::new(None) };
);

pub unsafe fn Gfx_Draw2DTexture(tex: &mut Texture, col: PackedCol) {
    let mut vertices = Gfx_Make2DQuad(tex, col);

    Gfx_SetVertexFormat(VertexFormat__VERTEX_FORMAT_TEXTURED);
    TEX_VB.with(|tex_vb| {
        let tex_vb = tex_vb.borrow_mut();
        let tex_vb = tex_vb.as_ref().unwrap();
        Gfx_UpdateDynamicVb_IndexedTris(tex_vb.resource_id, vertices.as_mut_ptr().cast(), 4);
    });
}

pub unsafe fn Texture_RenderShaded(tex: &mut Texture, shadeCol: PackedCol) {
    Gfx_BindTexture(tex.ID);
    Gfx_Draw2DTexture(tex, shadeCol);
}

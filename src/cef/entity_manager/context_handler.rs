use crate::helpers::*;
use classicube_helpers::events::gfx::{ContextLostEventHandler, ContextRecreatedEventHandler};
use classicube_sys::{
    OwnedGfxVertexBuffer, VertexFormat__VERTEX_FORMAT_P3FC4B, VertexFormat__VERTEX_FORMAT_P3FT2FC4B,
};

pub struct ContextHandler {
    context_lost_handler: ContextLostEventHandler,
    context_recreated_handler: ContextRecreatedEventHandler,
}

impl ContextHandler {
    pub fn new() -> Self {
        Self {
            context_lost_handler: ContextLostEventHandler::new(),
            context_recreated_handler: ContextRecreatedEventHandler::new(),
        }
    }

    fn context_recreated() {
        // create texture, vertex buffers, enable detour

        QUAD_VB.with(|cell| {
            *cell.borrow_mut() = Some(OwnedGfxVertexBuffer::create(
                VertexFormat__VERTEX_FORMAT_P3FC4B,
                4,
            ));
        });

        TEX_VB.with(|cell| {
            *cell.borrow_mut() = Some(OwnedGfxVertexBuffer::create(
                VertexFormat__VERTEX_FORMAT_P3FT2FC4B,
                4,
            ));
        });

        // Start calling our CefEntity's draw
        // unsafe {
        //     println!("enable RenderModel detour");
        //     self.local_player_render_model_detour.enable().unwrap();
        // }

        // CEF_CAN_DRAW.store(true, Ordering::SeqCst);
    }

    fn context_lost() {
        // CEF_CAN_DRAW.store(false, Ordering::SeqCst);

        // disable detour so we don't call our ModelRender
        // if self.local_player_render_model_detour.is_enabled() {
        //     println!("disable RenderModel detour");
        //     unsafe {
        //         self.local_player_render_model_detour.disable().unwrap();
        //     }
        // } else {
        //     println!("RenderModel detour already disabled?");
        // }

        // delete vertex buffers
        QUAD_VB.with(|cell| {
            cell.borrow_mut().take();
        });

        TEX_VB.with(|cell| {
            cell.borrow_mut().take();
        });
    }

    pub fn initialize(&mut self) {
        // we start with context created
        Self::context_recreated();

        self.context_lost_handler.on(|_| {
            println!("ContextLost {:?}", std::thread::current().id());

            Self::context_lost();
        });

        self.context_recreated_handler.on(|_| {
            println!("ContextRecreated {:?}", std::thread::current().id());

            Self::context_recreated();
        });
    }

    pub fn shutdown(&mut self) {
        Self::context_lost();
    }
}

use crate::helpers::*;
use classicube_helpers::events::gfx::{ContextLostEventHandler, ContextRecreatedEventHandler};
use classicube_sys::{
    OwnedGfxVertexBuffer, VertexFormat__VERTEX_FORMAT_P3FC4B, VertexFormat__VERTEX_FORMAT_P3FT2FC4B,
};
use log::debug;

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
        // create texture, vertex buffers

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
    }

    fn context_lost() {
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
            debug!("ContextLost {:?}", std::thread::current().id());

            Self::context_lost();
        });

        self.context_recreated_handler.on(|_| {
            debug!("ContextRecreated {:?}", std::thread::current().id());

            Self::context_recreated();
        });
    }

    pub fn shutdown(&mut self) {
        Self::context_lost();
    }
}

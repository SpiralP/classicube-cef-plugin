use std::{
    collections::VecDeque,
    mem,
    os::raw::c_short,
    sync::{Arc, Mutex},
    time::Duration,
};

use classicube_helpers::{async_manager, color::SILVER};
use classicube_sys::{
    cc_int16, Bitmap, Entity, EntityVTABLE, Entity_Init, Entity_SetModel, Gfx_UpdateTexturePart,
    LocationUpdate, Model_Render, OwnedGfxTexture, OwnedString, PackedCol, Texture, TextureRec,
    PACKEDCOL_WHITE,
};
use futures::channel::oneshot;
use tracing::{debug, warn};

use super::{BROWSER_ID_TO_ENTITY_ID, TEXTURE_HEIGHT, TEXTURE_WIDTH};
use crate::{
    api,
    cef::RustRefBrowser,
    chat::Chat,
    entity_manager::{DEFAULT_MODEL_HEIGHT, DEFAULT_MODEL_WIDTH},
    error::{Error, Result, ResultExt},
    helpers::format_duration,
    player::{Player, PlayerTrait, WebPlayer},
};

pub struct CefEntity {
    pub id: usize,
    pub name: Option<String>,

    pub entity: Box<Entity>,
    pub browser: Option<RustRefBrowser>,

    pub player: Player,
    pub queue: VecDeque<(Player, Arc<Mutex<Option<String>>>)>,
    pub should_send: bool,
    pub background_color: u32,

    v_table: Box<EntityVTABLE>,
    texture: OwnedGfxTexture,

    page_loaded_senders: Vec<oneshot::Sender<()>>,
}

impl CefEntity {
    pub fn register(
        id: usize,
        name: Option<String>,
        player: Player,
        mut queue: VecDeque<Player>,
        should_send: bool,
        background_color: u32,
    ) -> Self {
        let entity = Box::new(unsafe { mem::zeroed() });

        let v_table = Box::new(EntityVTABLE {
            Tick: Some(Self::tick),
            Despawn: Some(Self::despawn),
            SetLocation: Some(Self::set_location),
            GetCol: Some(Self::get_col),
            RenderModel: Some(Self::c_render_model),
            ShouldRenderName: Some(Self::should_render_name),
        });

        let mut pixels: Vec<u32> =
            vec![background_color; TEXTURE_WIDTH as usize * TEXTURE_HEIGHT as usize];

        let mut bmp = Bitmap {
            scan0: pixels.as_mut_ptr(),
            width: TEXTURE_WIDTH as i32,
            height: TEXTURE_HEIGHT as i32,
        };

        let texture = OwnedGfxTexture::new(&mut bmp, true, false);

        let mut this = Self {
            id,
            name,
            entity,
            v_table,
            texture,
            browser: None,
            player,
            // TODO spawn lookups here?
            queue: queue
                .drain(..)
                .map(|player| (player, Arc::new(Mutex::new(None))))
                .collect(),
            should_send,
            background_color,
            page_loaded_senders: Vec::new(),
        };

        this.register_entity();

        this
    }

    extern "C" fn tick(_entity: *mut Entity, _delta: f32) {}

    extern "C" fn despawn(_entity: *mut Entity) {}

    extern "C" fn set_location(_entity: *mut Entity, _update: *mut LocationUpdate) {}

    extern "C" fn get_col(_entity: *mut Entity) -> PackedCol {
        PACKEDCOL_WHITE
    }

    extern "C" fn c_render_model(_entity: *mut Entity, _delta_time: f32, _t: f32) {
        // we use the render_model function below directly instead
    }

    extern "C" fn should_render_name(_entity: *mut Entity) -> u8 {
        0
    }

    fn register_entity(&mut self) {
        let CefEntity {
            entity,
            v_table,
            texture,
            ..
        } = self;

        unsafe {
            Entity_Init(entity);
        }

        let model_name = OwnedString::new("cef");
        unsafe {
            Entity_SetModel(entity.as_mut(), model_name.as_cc_string());
        }

        entity.VTABLE = v_table.as_mut();
        entity.Velocity.set(0.0, 0.0, 0.0);
        entity.RotZ = 180.0;
        entity.TextureId = texture.resource_id;

        entity.Position.set(0.0, 0.0, 0.0);

        // hack so that Model can see browser resolution sizes
        // that are updated in update_texture
        // used in CefModel::draw
        entity.NameTex = Texture {
            ID: entity.TextureId,
            x: -(DEFAULT_MODEL_WIDTH as cc_int16 / 2),
            y: -(DEFAULT_MODEL_HEIGHT as cc_int16),
            width: DEFAULT_MODEL_WIDTH as _,
            height: DEFAULT_MODEL_HEIGHT as _,
            uv: TextureRec {
                u1: 0.0,
                v1: 0.0,
                u2: 1.0,
                v2: 1.0,
            },
        };
    }

    pub fn update_texture(&mut self, mut part: Bitmap) {
        // update uv's
        self.entity.NameTex.uv.u2 = part.width as f32 / TEXTURE_WIDTH as f32;
        self.entity.NameTex.uv.v2 = part.height as f32 / TEXTURE_HEIGHT as f32;

        unsafe {
            Gfx_UpdateTexturePart(self.texture.resource_id, 0, 0, &mut part, 0);
        }
    }

    pub fn render_model(&mut self) {
        if self.get_scale() != 0.0 {
            let entity = self.entity.as_mut();
            unsafe {
                Model_Render(entity.Model, entity);
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
        entity.ModelScale.x
    }

    pub fn set_size(&mut self, width: u16, height: u16) {
        let CefEntity { entity, .. } = self;
        entity.NameTex.x = -c_short::try_from(width / 2).unwrap();
        entity.NameTex.y = -c_short::try_from(height).unwrap();
        entity.NameTex.width = width;
        entity.NameTex.height = height;
    }

    pub fn get_size(&self) -> (u16, u16) {
        let CefEntity { entity, .. } = self;
        (entity.NameTex.width, entity.NameTex.height)
    }
}

impl CefEntity {
    /// add item to queue
    ///
    /// if item was queued, returns the size of queue,
    /// else returns None meaning we're about to play the item
    pub fn queue(&mut self, player: Player) -> Result<Option<usize>> {
        // this needs to determine if the current player was finished,
        // if it was then we play right away,
        // else we queue it for next

        if self.player.is_finished_playing() {
            self.play(player)?;

            Ok(None)
        } else {
            let shared = Arc::new(Mutex::new(None));

            // lookup title
            if let Player::YouTube(yt) = &player {
                let shared = shared.clone();
                let youtube_id = yt.id.clone();

                async_manager::spawn(async move {
                    debug!("lookup {}", youtube_id);

                    let f = async move {
                        let response = async_manager::timeout(
                            Duration::from_secs(5),
                            api::youtube::video(&youtube_id),
                        )
                        .await
                        .chain_err(|| "timed out")??;

                        // Justice - Cross (Full Album) (49:21)
                        let title = format!(
                            "{} ({})",
                            response.title,
                            format_duration(Duration::from_secs(response.duration_seconds as _))
                        );

                        let mut shared = shared.lock().unwrap();
                        *shared = Some(title.clone());

                        async_manager::spawn_on_main_thread(async move {
                            Chat::print(format!("{SILVER}{title}"));
                        });

                        Ok::<_, Error>(())
                    };

                    if let Err(e) = f.await {
                        warn!("youtube lookup error: {}", e);
                    }
                });
            }

            self.queue.push_back((player, shared));

            Ok(Some(self.queue.len()))
        }
    }

    pub fn stop(&mut self) -> Result<()> {
        self.play(Player::Web(WebPlayer::blank_page()))
    }

    pub fn play(&mut self, mut player: Player) -> Result<()> {
        let url = player.on_create()?;

        // TODO move this into the Player enum's on_create

        let browser = self.browser.as_ref().chain_err(|| "no browser")?;

        if self.player.type_name() == player.type_name() {
            // try to persist volume options
            //
            // only persist for same-type because if we went from a
            // Web player which has global volume to a YouTube, it would
            // make the youtube player global volume too
            let volume = self.player.get_volume();
            let volume_mode = self.player.get_volume_mode();
            self.player = player;

            let _ignore = self.player.set_volume(Some(browser), volume);
            let _ignore = self.player.set_volume_mode(Some(browser), volume_mode);
        } else {
            self.player = player;
        }

        browser.load_url(url)?;

        Ok(())
    }

    pub fn skip(&mut self) -> Result<()> {
        if let Some((new_player, _)) = self.queue.pop_front().take() {
            self.play(new_player)?;
        } else if !self.player.is_finished_playing() {
            // show blank page
            self.stop()?;
        }

        Ok(())
    }

    pub fn attach_browser(&mut self, browser: RustRefBrowser) {
        let browser_id = browser.get_identifier();

        BROWSER_ID_TO_ENTITY_ID.with(|ids| {
            let ids = &mut *ids.borrow_mut();

            ids.insert(browser_id, self.id);
            self.browser = Some(browser);
        });
    }

    pub fn on_page_loaded(&mut self, browser: &RustRefBrowser) {
        self.player.on_page_loaded(self.id, browser);

        for sender in self.page_loaded_senders.drain(..) {
            let _ignore = sender.send(());
        }
    }

    pub fn wait_for_page_load(&mut self) -> oneshot::Receiver<()> {
        let (sender, receiver) = oneshot::channel();
        self.page_loaded_senders.push(sender);

        receiver
    }
}

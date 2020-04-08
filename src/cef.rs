use crate::interface::*;
use classicube_helpers::tick::*;
use classicube_sys::*;
use std::cell::RefCell;

// Some means we are initialized
thread_local!(
    static CEF: RefCell<Option<Cef>> = RefCell::new(None);
);

fn print<S: Into<Vec<u8>>>(s: S) {
    let owned_string = OwnedString::new(s);
    unsafe {
        Chat_Add(owned_string.as_cc_string());
    }
}

pub fn initialize() {
    print("cef initialize");

    CEF.with(|option| {
        debug_assert!(option.borrow().is_none());

        *option.borrow_mut() = Some(Cef::new());
    });

    CEF.with(|option| {
        if let Some(cef) = &mut *option.borrow_mut() {
            cef.initialize();
        }
    });
}

pub fn shutdown() {
    print("cef shutdown");

    CEF.with(|option| {
        let mut cef = option.borrow_mut().take().unwrap();
        cef.shutdown();
    });
}

const WIDTH: usize = 512;
const HEIGHT: usize = 512;

static mut PIXELS: [u8; 4 * WIDTH * HEIGHT] = [0; 4 * WIDTH * HEIGHT];

static mut BMP: Bitmap = Bitmap {
    Scan0: unsafe { &mut PIXELS as *mut _ as *mut u8 },
    Width: WIDTH as i32,
    Height: HEIGHT as i32,
};

pub struct Cef {
    resource_id: Option<GfxResourceID>,
    tick_handler: TickEventHandler,
    initialized: bool,
}

impl Cef {
    pub fn new() -> Self {
        Self {
            resource_id: None,
            tick_handler: TickEventHandler::new(),
            initialized: false,
        }
    }

    pub fn initialize(&mut self) {
        let resource_id = unsafe {
            let resource_id = Gfx_CreateTexture(&mut BMP, 1, 0);
            print(format!("created resource {:?}", resource_id));

            // Atlas1D.TexIds[0] = resource_id;

            resource_id
        };

        self.resource_id = Some(resource_id);

        unsafe {
            assert_eq!(cef_init(Some(Self::on_paint_callback)), 0);
        }

        self.tick_handler.on(|_task| {
            //
            unsafe {
                assert_eq!(cef_step(), 0);
            }
        });

        self.initialized = true;
    }

    pub fn shutdown(&mut self) {
        if self.initialized {
            if let Some(mut cef_resource_id) = self.resource_id.take() {
                unsafe {
                    Gfx_DeleteTexture(&mut cef_resource_id);
                }
            }
            unsafe {
                assert_eq!(cef_free(), 0);
            }
            self.initialized = false;
        }
    }

    extern "C" fn on_paint_callback(
        new_pixels: *const ::std::os::raw::c_void,
        new_width: ::std::os::raw::c_int,
        new_height: ::std::os::raw::c_int,
    ) {
        CEF.with(|option| {
            if let Some(cef) = &*option.borrow() {
                print("cef");
                if let Some(resource_id) = cef.resource_id {
                    print("paint");

                    let mut part = Bitmap {
                        Scan0: new_pixels as *mut _,
                        Width: new_width as i32,
                        Height: new_height as i32,
                    };

                    unsafe {
                        Gfx_UpdateTexturePart(resource_id, 0, 0, &mut part, 0);
                    }
                }
            }
        });
    }
}

impl Drop for Cef {
    fn drop(&mut self) {
        self.shutdown();
    }
}

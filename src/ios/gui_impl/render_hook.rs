use std::os::raw::c_void;
use objc2::{msg_send, sel, Encode, Encoding};
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2::ffi::{class_getInstanceMethod, method_setImplementation, object_getClass, IMP};

static mut ORIG_NEXT_DRAWABLE: Option<IMP> = None;
static mut ORIG_PRESENT: Option<IMP> = None;
static mut DRAWABLE_SWIZZLED: bool = false;

static mut EGUI_COMMAND_QUEUE: *mut AnyObject = std::ptr::null_mut();

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct MTLClearColor {
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
}

unsafe impl Encode for MTLClearColor {
    const ENCODING: Encoding = Encoding::Struct(
        "MTLClearColor",
        &[f64::ENCODING, f64::ENCODING, f64::ENCODING, f64::ENCODING],
    );
}

extern "C" fn hooked_next_drawable(self_layer: *mut AnyObject, sel: Sel) -> *mut AnyObject {
    unsafe {
        if EGUI_COMMAND_QUEUE.is_null() {
            let device: *mut AnyObject = msg_send![self_layer, device];
            if !device.is_null() {
                EGUI_COMMAND_QUEUE = msg_send![device, newCommandQueue];
                info!("iOS: Created isolated Metal command queue for GUI");
            }
        }

        let orig_fn: extern "C" fn(*mut AnyObject, Sel) -> *mut AnyObject = std::mem::transmute(ORIG_NEXT_DRAWABLE.unwrap());
        let drawable = orig_fn(self_layer, sel);

        if !drawable.is_null() && !DRAWABLE_SWIZZLED {
            let drawable_class = object_getClass(drawable as *mut _);
            let present_sel = sel!(present);
            let method = class_getInstanceMethod(drawable_class, present_sel.as_ptr() as *const _);

            if !method.is_null() {
                let hooked_imp: IMP = Some(std::mem::transmute(hooked_present as extern "C" fn(*mut AnyObject, Sel)));
                ORIG_PRESENT = Some(method_setImplementation(method, hooked_imp));
                info!("iOS: CAMetalDrawable 'present' swizzled on the fly!");
                DRAWABLE_SWIZZLED = true;
            }
        }

        drawable
    }
}

extern "C" fn hooked_present(self_drawable: *mut AnyObject, sel: Sel) {
    unsafe {
        if !EGUI_COMMAND_QUEUE.is_null() {
            let texture: *mut AnyObject = msg_send![self_drawable, texture];

            if !texture.is_null() {
                let device: *mut AnyObject = msg_send![texture, device];

                let gui_lock = crate::core::gui::Gui::instance_or_init("ios.menu_open_key");

                if let Ok(mut gui) = gui_lock.lock() {
                    let width: usize = msg_send![texture, width];
                    let height: usize = msg_send![texture, height];
                    gui.set_screen_size(width as i32, height as i32);

                    let full_output = gui.run();

                    let pixels_per_point = gui.context.pixels_per_point();
                    let screen_size = gui.context.screen_rect().size();

                    let primitives = gui.context.tessellate(full_output.shapes, pixels_per_point);

                    if let Some(painter) = gui.get_or_init_painter(device) {
                        let pass_class = objc2::class!(MTLRenderPassDescriptor);
                        let pass: *mut AnyObject = msg_send![pass_class, renderPassDescriptor];

                        let color_attachments: *mut AnyObject = msg_send![pass, colorAttachments];
                        let attachment: *mut AnyObject = msg_send![color_attachments, objectAtIndexedSubscript: 0_usize];

                        let _: () = msg_send![attachment, setTexture: texture];
                        let _: () = msg_send![attachment, setLoadAction: 1_usize];
                        let _: () = msg_send![attachment, setStoreAction: 1_usize];

                        let cmd_buf: *mut AnyObject = msg_send![EGUI_COMMAND_QUEUE, commandBuffer];
                        let encoder: *mut AnyObject = msg_send![cmd_buf, renderCommandEncoderWithDescriptor: pass];

                        if !encoder.is_null() {
                            painter.paint(
                                device,
                                encoder,
                                screen_size,
                                pixels_per_point,
                                full_output.textures_delta,
                                primitives,
                            );
                            let _: () = msg_send![encoder, endEncoding];
                        }

                        let _: () = msg_send![cmd_buf, commit];
                    }
                }
            }
        }

        if let Some(orig_imp) = ORIG_PRESENT {
            let orig_fn: extern "C" fn(*mut AnyObject, Sel) = std::mem::transmute(orig_imp);
            orig_fn(self_drawable, sel);
        }
    }
}

pub fn init() {
    unsafe {
        let layer_class = AnyClass::get("CAMetalLayer").expect("Failed to find CAMetalLayer");
        let next_drawable_sel = sel!(nextDrawable);

        let method = class_getInstanceMethod(
            layer_class as *const _ as *mut _,
            next_drawable_sel.as_ptr() as *const _
        );

        if !method.is_null() {
            let hooked_fn_ptr = hooked_next_drawable as extern "C" fn(*mut AnyObject, Sel) -> *mut AnyObject;
            let hooked_imp: IMP = Some(std::mem::transmute(hooked_fn_ptr));
            let orig = method_setImplementation(method, hooked_imp);
            ORIG_NEXT_DRAWABLE = Some(orig);
            info!("iOS: CAMetalLayer nextDrawable swizzled");
        } else {
            error!("iOS: Failed to hook nextDrawable");
        }
    }
}
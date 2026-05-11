use std::os::raw::c_void;
use std::sync::atomic::Ordering;
use objc2::{msg_send, sel, Encode, Encoding};
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2::ffi::{class_getInstanceMethod, method_setImplementation, IMP};
use crate::core::gui::INSTANCE;

static mut ORIG_SEND_EVENT: Option<IMP> = None;
static LAST_MENU_TOGGLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CGPoint {
    x: f64,
    y: f64,
}

unsafe impl Encode for CGPoint {
    const ENCODING: Encoding = Encoding::Struct(
        "CGPoint",
        &[f64::ENCODING, f64::ENCODING],
    );
}

extern "C" fn hooked_send_event(self_obj: *mut AnyObject, sel: Sel, event: *mut AnyObject) {
    unsafe {
        let mut egui_wants_input = false;

        let event_type: isize = msg_send![event, type];
        let event_subtype: isize = msg_send![event, subtype];

        if event_type == 1 && event_subtype == 1 {
            if let Some(gui_mutex) = crate::core::gui::INSTANCE.get() {
                if let Ok(mut gui) = gui_mutex.lock() {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let last = LAST_MENU_TOGGLE.load(Ordering::Relaxed);
                    if now - last > 500 {
                        LAST_MENU_TOGGLE.store(now, Ordering::Relaxed);
                        gui.toggle_menu();
                        egui_wants_input = true;
                    }
                }
            }
        }

        let touches: *mut AnyObject = msg_send![event, allTouches];

        if !touches.is_null() {
            let enumerator: *mut AnyObject = msg_send![touches, objectEnumerator];
            let mut touch: *mut AnyObject = msg_send![enumerator, nextObject];

            while !touch.is_null() {
                let phase: isize = msg_send![touch, phase];
                let tap_count: usize = msg_send![touch, tapCount];
                let window: *mut AnyObject = msg_send![touch, window];

                if !window.is_null() {
                    let location: CGPoint = msg_send![touch, locationInView: window];
                    let scale: f64 = msg_send![window, contentScaleFactor];

                    let physical_x = (location.x * scale) as f32;
                    let physical_y = (location.y * scale) as f32;

                    if let Some(gui_mutex) = crate::core::gui::INSTANCE.get() {
                        if let Ok(mut gui) = gui_mutex.lock() {
                            let egui_scale = gui.context.pixels_per_point();
                            let pos = egui::pos2(physical_x / egui_scale, physical_y / egui_scale);

                            let mut events = vec![egui::Event::PointerMoved(pos)];

                            match phase {
                                0 => {
                                    events.push(egui::Event::PointerButton {
                                        pos,
                                        button: egui::PointerButton::Primary,
                                        pressed: true,
                                        modifiers: Default::default(),
                                    });
                                }
                                3 | 4 => {
                                    events.push(egui::Event::PointerButton {
                                        pos,
                                        button: egui::PointerButton::Primary,
                                        pressed: false,
                                        modifiers: Default::default(),
                                    });
                                }
                                _ => {}
                            }

                            gui.inject_events(events);

                            if phase == 0 && tap_count == 3 && location.x < 200.0 && location.y < 200.0 {
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64;

                                let last = LAST_MENU_TOGGLE.load(Ordering::Relaxed);

                                if now - last > 500 {
                                    LAST_MENU_TOGGLE.store(now, Ordering::Relaxed);
                                    gui.toggle_menu();
                                    egui_wants_input = true;
                                }
                            }

                            if gui.context.wants_pointer_input() || gui.context.is_pointer_over_area() || gui.is_consuming_input() {
                                egui_wants_input = true;
                            }
                        }
                    }
                }
                touch = msg_send![enumerator, nextObject];
            }
        }

        if !egui_wants_input {
            if let Some(orig_imp) = ORIG_SEND_EVENT {
                let orig_fn: extern "C" fn(*mut AnyObject, Sel, *mut AnyObject) = std::mem::transmute(orig_imp);
                orig_fn(self_obj, sel, event);
            }
        }
    }
}

pub fn init() {
    unsafe {
        let ui_window_cls = AnyClass::get("UIWindow").expect("Failed to find UIWindow");
        let send_event_sel = sel!(sendEvent:);

        let method = class_getInstanceMethod(
            ui_window_cls as *const _ as *mut _,
            send_event_sel.as_ptr() as *const _
        );

        if !method.is_null() {
            let hooked_fn_ptr = hooked_send_event as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject);
            let hooked_imp: IMP = Some(std::mem::transmute(hooked_fn_ptr));

            let orig = method_setImplementation(method, hooked_imp);
            ORIG_SEND_EVENT = Some(orig);
            info!("iOS: UIWindow sendEvent: successfully swizzled with objc2");
        } else {
            error!("iOS: Failed to get sendEvent: method");
        }
    }
}
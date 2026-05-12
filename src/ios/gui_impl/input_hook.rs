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

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CGSize {
    width: f64,
    height: f64,
}

unsafe impl Encode for CGSize {
    const ENCODING: Encoding = Encoding::Struct(
        "CGSize",
        &[f64::ENCODING, f64::ENCODING],
    );
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

unsafe impl Encode for CGRect {
    const ENCODING: Encoding = Encoding::Struct(
        "CGRect",
        &[CGPoint::ENCODING, CGSize::ENCODING],
    );
}

extern "C" fn hooked_send_event(self_obj: *mut AnyObject, sel: Sel, event: *mut AnyObject) {
    unsafe {
        let is_window: bool = if let Some(ui_window_cls) = AnyClass::get("UIWindow") {
            msg_send![self_obj, isKindOfClass: ui_window_cls]
        } else {
            false
        };

        if !is_window {
            if let Some(orig_imp) = ORIG_SEND_EVENT {
                let orig_fn: extern "C" fn(*mut AnyObject, Sel, *mut AnyObject) = std::mem::transmute(orig_imp);
                orig_fn(self_obj, sel, event);
            }
            return;
        }

        let mut egui_wants_input = false;
        let mut has_native_ui_touch = false;

        let event_type: isize = msg_send![event, type];

        if event_type == 0 {
            let all_touches: *mut AnyObject = msg_send![event, allTouches];
            let enumerator: *mut AnyObject = msg_send![all_touches, objectEnumerator];
            let mut touch: *mut AnyObject = msg_send![enumerator, nextObject];

            while !touch.is_null() {
                let phase: isize = msg_send![touch, phase];
                let tap_count: usize = msg_send![touch, tapCount];
                let view: *mut AnyObject = msg_send![touch, view];

                if !view.is_null() {
                    let view_cls = object_getClass(view as *mut c_void);
                    let view_cls_name = std::ffi::CStr::from_ptr(class_getName(view_cls)).to_string_lossy();

                    let is_main_view = view_cls_name.contains("UnityView") || view_cls_name.contains("MTKView");

                    if !is_main_view {
                        has_native_ui_touch = true;
                    } else {
                        let location: CGPoint = msg_send![touch, locationInView: view];
                        let bounds: CGRect = msg_send![view, bounds];

                        if bounds.size.width > 0.0 && bounds.size.height > 0.0 {
                            if let Some(gui_mutex) = crate::core::gui::INSTANCE.get() {
                                if let Ok(mut gui) = gui_mutex.lock() {
                                    let screen_rect = gui.context.screen_rect();

                                    let pos = egui::pos2(
                                        (location.x / bounds.size.width) as f32 * screen_rect.width(),
                                        (location.y / bounds.size.height) as f32 * screen_rect.height()
                                    );

                                    let mut events = vec![];

                                    let touch_phase = match phase {
                                        0 => Some(egui::TouchPhase::Start),
                                        1 => Some(egui::TouchPhase::Move),
                                        3 | 4 => Some(egui::TouchPhase::End),
                                        _ => None,
                                    };

                                    if let Some(t_phase) = touch_phase {
                                        events.push(egui::Event::Touch {
                                            device_id: egui::TouchDeviceId(0),
                                            id: egui::TouchId::from(touch as u64),
                                            phase: t_phase,
                                            pos,
                                            force: None,
                                        });
                                    }

                                    match phase {
                                        0 => {
                                            events.push(egui::Event::PointerMoved(pos));
                                            events.push(egui::Event::PointerButton {
                                                pos,
                                                button: egui::PointerButton::Primary,
                                                pressed: true,
                                                modifiers: Default::default(),
                                            });
                                        }
                                        1 | 2 => {
                                            events.push(egui::Event::PointerMoved(pos));
                                        }
                                        3 | 4 => {
                                            events.push(egui::Event::PointerMoved(pos));
                                            events.push(egui::Event::PointerButton {
                                                pos,
                                                button: egui::PointerButton::Primary,
                                                pressed: false,
                                                modifiers: Default::default(),
                                            });
                                            events.push(egui::Event::PointerGone);
                                        }
                                        _ => {}
                                    }

                                    gui.inject_events(events);

                                    let corner_zone_size = screen_rect.width().min(screen_rect.height()) * 0.12;

                                    if phase == 0 && tap_count == 3 {
                                        let config = crate::core::Hachimi::instance().config.load();

                                        if !config.disable_gui && pos.x < corner_zone_size && pos.y < corner_zone_size {
                                            let now = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap_or_default()
                                                .as_millis() as u64;

                                            let last = LAST_MENU_TOGGLE.load(std::sync::atomic::Ordering::Relaxed);

                                            if now - last > 500 {
                                                LAST_MENU_TOGGLE.store(now, std::sync::atomic::Ordering::Relaxed);
                                                gui.toggle_menu();
                                                egui_wants_input = true;
                                            }
                                        }
                                        else if config.hide_ingame_ui_hotkey && pos.x > (screen_rect.width() - corner_zone_size) && pos.y < corner_zone_size {
                                            let now = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap_or_default()
                                                .as_millis() as u64;
                                            static LAST_UI_TOGGLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                                            let last = LAST_UI_TOGGLE.load(std::sync::atomic::Ordering::Relaxed);
                                            if now - last > 500 {
                                                LAST_UI_TOGGLE.store(now, std::sync::atomic::Ordering::Relaxed);
                                                crate::il2cpp::symbols::Thread::main_thread().schedule(crate::core::Gui::toggle_game_ui);
                                                egui_wants_input = true;
                                            }
                                        }
                                    }

                                    if gui.context.wants_pointer_input() || gui.context.is_pointer_over_area() || gui.is_consuming_input() {
                                        egui_wants_input = true;
                                    }
                                }
                            }
                        }
                    }
                }
                touch = msg_send![enumerator, nextObject];
            }
        }
        else if event_type == 1 {
            let event_subtype: isize = msg_send![event, subtype];

            if event_subtype == 1 {
                if let Some(gui_mutex) = crate::core::gui::INSTANCE.get() {
                    if let Ok(mut gui) = gui_mutex.lock() {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;

                        let last = LAST_MENU_TOGGLE.load(std::sync::atomic::Ordering::Relaxed);

                        if now - last > 500 {
                            LAST_MENU_TOGGLE.store(now, std::sync::atomic::Ordering::Relaxed);
                            gui.toggle_menu();
                            egui_wants_input = true;
                        }
                    }
                }
            }
        }

        if has_native_ui_touch || !egui_wants_input {
            if let Some(orig_imp) = ORIG_SEND_EVENT {
                let orig_fn: extern "C" fn(*mut AnyObject, Sel, *mut AnyObject) = std::mem::transmute(orig_imp);
                orig_fn(self_obj, sel, event);
            }
        }
    }
}

extern "C" {
    fn objc_getClass(name: *const u8) -> *mut c_void;
    fn objc_allocateClassPair(superclass: *mut c_void, name: *const u8, extra_bytes: usize) -> *mut c_void;
    fn class_addMethod(cls: *mut c_void, sel: *mut c_void, imp: *mut c_void, types: *const u8) -> bool;
    fn objc_registerClassPair(cls: *mut c_void);
    fn object_getClass(obj: *mut c_void) -> *mut c_void;
    fn class_getName(cls: *mut c_void) -> *const std::os::raw::c_char;
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

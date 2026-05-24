//! macOS Force Touch monitor — see `~/rhizomatic-preset/guidance/design/native-gesture-bridge.md`.
//!
//! Installs an NSEvent local monitor at app startup. On a click whose
//! `stage()` is 2 (the haptic-trackpad force-click threshold), emits a
//! `force-touch` Tauri event with `{ pressure, x, y }` in window-local CSS
//! pixels. The block is kept alive for the app lifetime via `RcBlock` moved
//! into the closure; if a future objc2 revision changes that, capture the
//! monitor token from `addLocalMonitorForEventsMatchingMask_handler`.

#![cfg(target_os = "macos")]

use block2::RcBlock;
use objc2_app_kit::{NSEvent, NSEventMask, NSEventType};
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize)]
struct ForceTouchPayload {
    pressure: f32,
    x: f64,
    y: f64,
}

pub fn install(app: AppHandle) {
    let mask = NSEventMask::LeftMouseDown | NSEventMask::RightMouseDown;

    let block = RcBlock::new(move |event: core::ptr::NonNull<NSEvent>| -> *mut NSEvent {
        unsafe {
            let event_ref: &NSEvent = event.as_ref();
            let kind = event_ref.r#type();
            let is_click = kind == NSEventType::LeftMouseDown
                || kind == NSEventType::RightMouseDown;
            if is_click && event_ref.stage() == 2 {
                let loc = event_ref.locationInWindow();
                let _ = app.emit(
                    "force-touch",
                    ForceTouchPayload {
                        pressure: event_ref.pressure(),
                        x: loc.x,
                        y: loc.y,
                    },
                );
            }
        }
        // Return the event unchanged so the system keeps processing it.
        event.as_ptr()
    });

    unsafe {
        let _ = NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &block);
    }
}

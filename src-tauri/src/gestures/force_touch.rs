//! macOS Force Touch monitor — see `~/rhizomatic-preset/guidance/design/native-gesture-bridge.md`.
//!
//! Installs an NSEvent local monitor at app startup. On a click whose
//! `stage()` is 2 (the haptic-trackpad force-click threshold), emits a
//! `force-touch` Tauri event with `{ pressure, x, y }` in window-local CSS
//! pixels.
//!
//! The block runs on the AppKit main thread and is called via the ObjC FFI,
//! which cannot unwind. We catch any panic inside the block so a bug in the
//! handler logs instead of aborting the whole process.

#![cfg(target_os = "macos")]

use std::panic::{catch_unwind, AssertUnwindSafe};

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
        // SAFETY: the system gives us a valid NSEvent pointer for the
        // lifetime of this callback. We only borrow it.
        let event_ptr = event.as_ptr();
        let result = catch_unwind(AssertUnwindSafe(|| unsafe {
            let event_ref: &NSEvent = event.as_ref();
            let kind = event_ref.r#type();
            let is_click = kind == NSEventType::LeftMouseDown
                || kind == NSEventType::RightMouseDown;
            if !is_click {
                return;
            }
            // stage() returns 0 on non-haptic hardware — exactly what we want.
            if event_ref.stage() != 2 {
                return;
            }
            let loc = event_ref.locationInWindow();
            let _ = app.emit(
                "force-touch",
                ForceTouchPayload {
                    pressure: event_ref.pressure(),
                    x: loc.x,
                    y: loc.y,
                },
            );
        }));
        if result.is_err() {
            eprintln!("force-touch monitor: handler panicked (suppressed)");
        }
        // Pass the event through unchanged.
        event_ptr
    });

    unsafe {
        let _ = NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &block);
    }
    // Intentionally leak the RcBlock so the block stays alive for the app
    // lifetime. The system retains it via the monitor, but tying our local
    // RcBlock's drop to scope-exit was the simplest way to ensure no double-
    // free path during teardown shenanigans.
    std::mem::forget(block);
}

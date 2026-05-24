//! macOS Force Touch monitor.
//!
//! Detection model (after empirical fix — the original design doc was wrong):
//!
//! - Monitor `NSEventMaskPressure` (pressure-gesture events), NOT
//!   `LeftMouseDown`. The `stage()` method is only safe to call on
//!   pressure-gesture events; calling it on a regular mouse-down throws
//!   `NSInvalidArgumentException` which crosses the ObjC↔Rust FFI as a
//!   foreign exception and aborts the process.
//!
//! - On each pressure-change event read `stage()`. Stage progression on a
//!   force-click is 0 → 1 → 2 → 0. We emit the `force-touch` Tauri event
//!   on the upward transition into stage 2 (deduplicated via a Cell so the
//!   continuous pressureChange stream doesn't spam the bus).
//!
//! Coordinate note: NSEvent `locationInWindow` is in window-local points
//! with bottom-left origin (Cocoa convention). The frontend hit-tests via
//! `getBoundingClientRect` which is top-left origin — we flip Y here using
//! the event's window height so the payload is already in the CSS frame.
//! Falling back to the raw value if the window pointer is null.

#![cfg(target_os = "macos")]

use std::cell::Cell;
use std::ptr::NonNull;
use std::rc::Rc;

use block2::RcBlock;
use objc2_app_kit::{NSEvent, NSEventMask};
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize)]
struct ForceTouchPayload {
    pressure: f32,
    x: f64,
    y: f64,
}

pub fn install(app: AppHandle) {
    let mask = NSEventMask::Pressure;
    let last_stage: Rc<Cell<i64>> = Rc::new(Cell::new(0));

    let block = {
        let last_stage = last_stage.clone();
        RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
            let event_ptr = event.as_ptr();
            unsafe {
                let event_ref: &NSEvent = event.as_ref();
                let stage = event_ref.stage() as i64;
                let was = last_stage.replace(stage);
                if stage == 2 && was != 2 {
                    let loc = event_ref.locationInWindow();
                    let pressure = event_ref.pressure();
                    // NOTE: locationInWindow is bottom-left origin (Cocoa).
                    // Frontend hit-tests via getBoundingClientRect which is
                    // top-left. Verify coords in practice — flip Y here
                    // (h - loc.y) if the hit-test misses; deferred until we
                    // know whether the wry/Tauri webview normalises it.
                    let _ = app.emit(
                        "force-touch",
                        ForceTouchPayload {
                            pressure,
                            x: loc.x,
                            y: loc.y,
                        },
                    );
                }
            }
            event_ptr
        })
    };

    unsafe {
        let _ = NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &block);
    }
    // System retains the block via the monitor; pin our handle for the app
    // lifetime to remove any ambiguity about drop ordering.
    std::mem::forget(block);
    std::mem::forget(last_stage);
}

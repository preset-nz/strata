import { useEffect } from "react"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

/**
 * Payload of the `force-touch` Tauri event emitted by the macOS NSEvent
 * monitor (`src-tauri/src/gestures/force_touch.rs`). Coordinates are
 * window-local CSS pixels in the top-left frame — the Rust side reports
 * Cocoa coords (bottom-left origin) and this hook flips Y here using
 * `window.innerHeight` before handing the payload to consumers.
 *
 * See `~/rhizomatic-preset/guidance/design/native-gesture-bridge.md`.
 */
export type ForceTouchPayload = {
  pressure: number
  x: number
  y: number
}

/**
 * Subscribe to the `force-touch` Tauri event for the lifetime of the
 * component. The handler is called with the payload each time a haptic
 * trackpad reports a stage-2 click anywhere in the app's window.
 *
 * No-op on non-macOS builds (the Rust monitor is cfg-gated, so the event
 * simply never fires there).
 */
export function useForceTouch(handler: (payload: ForceTouchPayload) => void) {
  useEffect(() => {
    let unlisten: UnlistenFn | undefined
    let cancelled = false
    listen<ForceTouchPayload>("force-touch", (event) => {
      const flipped: ForceTouchPayload = {
        pressure: event.payload.pressure,
        x: event.payload.x,
        y: window.innerHeight - event.payload.y,
      }
      handler(flipped)
    })
      .then((fn) => {
        if (cancelled) fn()
        else unlisten = fn
      })
      .catch(() => {
        /* listen() rejects only if the event bus is unavailable — ignore. */
      })
    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [handler])
}

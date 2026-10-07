import { useCallback, useEffect, useRef } from "react"
import {
  type ForceTouchPayload,
  useForceTouch,
} from "./gestures/use-force-touch"

/**
 * Hard-press handler bound to a single element. The press is the macOS
 * Force Touch stage-2 gesture — see
 * `~/rhizomatic-preset/guidance/design/native-gesture-bridge.md`. The hook
 * returns a ref you attach to the element; when a `force-touch` event fires
 * with `{ x, y }` inside the element's viewport rectangle, `onActivate`
 * runs.
 *
 * No long-press fallback: long-press collided with HTML5 drag (the browser
 * suppresses pointermove during a drag session, so the timer couldn't be
 * cancelled by motion) and force-touch is the canonical haptic-trackpad
 * trigger. Non-Mac and non-haptic users will get a keyboard/menu surface
 * for Quickview when those land.
 */
export function useHardPress<T extends HTMLElement>(
  onActivate: (() => void) | undefined
) {
  const ref = useRef<T | null>(null)
  const cbRef = useRef(onActivate)
  useEffect(() => {
    cbRef.current = onActivate
  })

  const onForceTouch = useCallback((p: ForceTouchPayload) => {
    const el = ref.current
    if (!el || !cbRef.current) return
    const rect = el.getBoundingClientRect()
    if (
      p.x >= rect.left &&
      p.x <= rect.right &&
      p.y >= rect.top &&
      p.y <= rect.bottom
    ) {
      cbRef.current()
    }
  }, [])

  useForceTouch(onForceTouch)
  return ref
}

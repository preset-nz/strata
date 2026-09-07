// Tracks whether a modal-ish overlay (Quickview today) is on screen.
//
// Window-level keyboard handlers need to stand down while an overlay owns the
// screen, but two `window` keydown listeners can't negotiate via propagation —
// they share a target and fire in registration order. A module-scoped counter
// read at event time is the honest mechanism: handlers are imperative anyway,
// so there's nothing to re-render on.

let openCount = 0

// Call on overlay mount; the returned function releases on unmount.
export function registerOverlay(): () => void {
  openCount += 1
  let released = false
  return () => {
    if (released) return
    released = true
    openCount -= 1
  }
}

export function isOverlayOpen(): boolean {
  return openCount > 0
}

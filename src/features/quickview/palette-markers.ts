import { useSyncExternalStore } from "react"
import { listen } from "@tauri-apps/api/event"

export const MARKER_COUNTS = [3, 5, 7] as const
export type MarkerCount = (typeof MARKER_COUNTS)[number]

type State = { on: boolean; count: MarkerCount }

// Session-wide, so the markers stay on while stepping through Quickview.
let state: State = { on: false, count: 5 }
const subscribers = new Set<() => void>()

function set(next: Partial<State>) {
  state = { ...state, ...next }
  subscribers.forEach((s) => s())
}

export const paletteMarkers = {
  toggle: () => set({ on: !state.on }),
  setCount: (count: MarkerCount) => set({ on: true, count }),
}

// View > Palette Markers (Shift+Cmd+M) is the one command; the Quickview
// header button calls the same toggle.
listen("menu://view/palette-markers", () => paletteMarkers.toggle()).catch(() => {
  // No native menu outside the Tauri shell (`just run web`); the button still works.
})

export function usePaletteMarkers(): State {
  return useSyncExternalStore(
    (cb) => {
      subscribers.add(cb)
      return () => subscribers.delete(cb)
    },
    () => state,
  )
}

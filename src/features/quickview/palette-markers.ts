import { useSyncExternalStore } from "react"

export const MARKER_COUNTS = [3, 5, 7] as const
export type MarkerCount = (typeof MARKER_COUNTS)[number]

type State = { on: boolean; count: MarkerCount }

// Session-wide, so the markers stay on while stepping through Quickview.
let state: State = { on: false, count: 5 }
const subscribers = new Set<() => void>()

function set(next: Partial<State>) {
  state = { ...state, ...next }
  for (const s of subscribers) s()
}

export const paletteMarkers = {
  toggle: () => set({ on: !state.on }),
  setCount: (count: MarkerCount) => set({ on: true, count }),
}

// View > Palette Markers (Shift+Cmd+M, bound in App.tsx) and the Quickview
// header button both call `toggle`.

export function usePaletteMarkers(): State {
  return useSyncExternalStore(
    (cb) => {
      subscribers.add(cb)
      return () => subscribers.delete(cb)
    },
    () => state
  )
}

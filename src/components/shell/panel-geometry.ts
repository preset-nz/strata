// Geometry for the shell's side panels. Kept out of SidePanel.tsx so that file
// exports only its component — mixing value exports breaks Fast Refresh.

export const COLLAPSED_WIDTH = 28

export type PanelSide = "left" | "right"

export type PanelState = {
  width: number
  collapsed: boolean
}

// A width may arrive from a previous session on a wider display, or corrupt
// from storage; neither may escape this panel's bounds.
export function clampWidth(width: number, min: number, max: number): number {
  if (!Number.isFinite(width)) return min
  return Math.min(max, Math.max(min, Math.round(width)))
}

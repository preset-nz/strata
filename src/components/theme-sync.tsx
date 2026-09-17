// Keeps the theme provider and the `appearance.theme` preference agreed.
// The preference is the source on launch and whenever the Settings window
// changes it; the provider's own "d" toggle writes back so the file matches
// what is on screen. Guarded both ways, so no ping-pong.

import { useEffect, useRef } from "react"
import { usePreference, usePreferenceActions, usePreferences } from "@preset.nz/preferences"
import { useTheme } from "./theme-provider"

type Theme = "dark" | "light" | "system"

export function ThemeSync() {
  const { theme, setTheme } = useTheme()
  const loaded = usePreferences() !== null
  const pref = usePreference<string>("appearance.theme", "system") as Theme
  const { set } = usePreferenceActions()
  const lastPref = useRef<Theme | null>(null)

  useEffect(() => {
    if (!loaded) return
    if (pref !== lastPref.current) {
      lastPref.current = pref
      if (pref !== theme) setTheme(pref)
      return
    }
    if (theme !== pref) {
      lastPref.current = theme
      void set("appearance.theme", theme)
    }
  }, [loaded, pref, theme, setTheme, set])

  return null
}

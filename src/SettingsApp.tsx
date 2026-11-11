import { SettingsWindow, usePreferencesBootstrap } from "@preset.nz/preferences"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { ThemeSync } from "./components/theme-sync"

// The Settings window's content. Rust opens it from the Settings… menu item
// (src-tauri/src/menu.rs) at `index.html?window=settings`; main.tsx picks
// this over the app. Escape closes it; the window's own close button works
// as for any window.
export function SettingsApp() {
  usePreferencesBootstrap()
  return (
    <>
      <ThemeSync />
      <SettingsWindow
        onClose={() => void getCurrentWindow().close()}
        persistKey="strata.settings"
      />
    </>
  )
}

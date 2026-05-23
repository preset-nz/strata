use std::path::Path;

use anyhow::Result;
use trash::TrashContext;

#[cfg(target_os = "macos")]
use trash::macos::{DeleteMethod, TrashContextExtMacos};

#[allow(dead_code)]
pub fn move_to_trash(path: &Path) -> Result<()> {
    let mut ctx = TrashContext::default();
    // Default on macOS is DeleteMethod::Finder, which routes through
    // `osascript` + Finder.app — plays the trash sound per file and
    // adds a subprocess + IPC round-trip per delete. NsFileManager is
    // silent, faster, and needs no extra permissions.
    #[cfg(target_os = "macos")]
    ctx.set_delete_method(DeleteMethod::NsFileManager);
    ctx.delete(path)?;
    Ok(())
}

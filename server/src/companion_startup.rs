//! Per-user Windows sign-in shortcut. No credentials are stored in the link.
use std::{path::{Path, PathBuf}, process::Command, os::windows::process::CommandExt};

fn shortcut_path() -> Result<PathBuf, String> {
    let mut path = [0u16; 260];
    let result = unsafe {
        windows_sys::Win32::UI::Shell::SHGetFolderPathW(
            std::ptr::null_mut(), windows_sys::Win32::UI::Shell::CSIDL_STARTUP as i32,
            std::ptr::null_mut(), 0, path.as_mut_ptr())
    };
    if result < 0 { return Err("Could not find your Windows Startup folder.".into()); }
    use std::os::windows::ffi::OsStringExt;
    Ok(PathBuf::from(std::ffi::OsString::from_wide(
        &path[..path.iter().position(|c| *c == 0).unwrap_or(path.len())]
    )).join("Ironwood Companion.lnk"))
}

pub fn enabled() -> Result<bool, String> {
    shortcut_path()?.try_exists().map_err(|e| format!("Could not read the Startup shortcut: {e}"))
}

pub fn set_enabled(enabled: bool, directory: &Path, config: &Path) -> Result<(), String> {
    let shortcut = shortcut_path()?;
    if !enabled {
        return remove_shortcut(&shortcut);
    }
    // Both desktop and --tray server launches use the desktop executable at sign-in.
    let executable = std::env::current_exe().map_err(|e| e.to_string())?
        .with_file_name("ironwood-companion.exe");
    create_shortcut(&shortcut, &executable, directory, config)
}

fn remove_shortcut(shortcut: &Path) -> Result<(), String> {
    match std::fs::remove_file(shortcut) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("Could not remove the Startup shortcut: {e}")),
    }
}

fn quoted_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    // Double trailing backslashes so CommandLineToArgvW does not escape the closing quote.
    let trailing = text.chars().rev().take_while(|c| *c == '\\').count();
    format!("\"{text}{}\"", "\\".repeat(trailing))
}

fn create_shortcut(shortcut: &Path, executable: &Path, directory: &Path, config: &Path) -> Result<(), String> {
    if !executable.is_file() { return Err("Keep ironwood-companion.exe in the Companion application folder before enabling sign-in startup.".into()); }
    let directory = std::path::absolute(directory).map_err(|e| e.to_string())?;
    let config = std::path::absolute(config).map_err(|e| e.to_string())?;
    // Paths are passed as environment values, never interpolated into PowerShell code.
    // Windows paths cannot contain quotes; the explicit quoting preserves spaces.
    let arguments = format!("--tray --data-dir {} --config {}", quoted_path(&directory), quoted_path(&config));
    let powershell = PathBuf::from(std::env::var_os("SystemRoot").ok_or("Windows system folder is unavailable")?)
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let output = Command::new(powershell)
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
            include_str!("companion-startup.ps1")])
        .env_remove("IRONWOOD_STARTUP_READ_TEST")
        .env("IRONWOOD_STARTUP_LINK", shortcut)
        .env("IRONWOOD_STARTUP_EXE", executable)
        .env("IRONWOOD_STARTUP_ARGS", arguments)
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output().map_err(|e| format!("Could not create the Startup shortcut: {e}"))?;
    if !output.status.success() {
        return Err(format!("Could not create the Startup shortcut: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }
    if !shortcut.is_file() { return Err("Windows did not save the Startup shortcut.".into()); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_preserves_paths_and_can_be_removed_without_touching_other_files() {
        let root = std::env::temp_dir().join(format!("ironwood-startup-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&root).unwrap();
        let executable = root.join("ironwood-companion.exe");
        std::fs::write(&executable, b"test fixture, never executed").unwrap();
        let shortcut = root.join("Ironwood Companion.lnk");
        let data = root.join("custom data & $values ' Unicode ♟");
        let config = data.join("custom config.json");
        create_shortcut(&shortcut, &executable, &data, &config).unwrap();
        // Read the actual .lnk back through the Windows shell, without launching it.
        let output = Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command",
            include_str!("companion-startup.ps1")])
            .env("IRONWOOD_STARTUP_LINK", &shortcut).env("IRONWOOD_STARTUP_READ_TEST", "1")
            .creation_flags(0x08000000).output().unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(),
            format!("--tray --data-dir {} --config {}", quoted_path(&data), quoted_path(&config)));
        create_shortcut(&shortcut, &executable, &data, &config).unwrap();
        remove_shortcut(&shortcut).unwrap();
        remove_shortcut(&shortcut).unwrap();
        assert!(!shortcut.exists());
        assert!(executable.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn windows_root_path_keeps_its_trailing_separator() {
        assert_eq!(quoted_path(Path::new("C:\\")), "\"C:\\\\\"");
    }
}

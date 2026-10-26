//! Windows release updater. Trust configuration is embedded by the release build,
//! never accepted from a website, config.json, or the downloaded manifest.
use crate::config::Error;
use std::{
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const SCRIPT: &str = include_str!("../windows/update-companion.ps1");
const FEED: Option<&str> = option_env!("IRONWOOD_UPDATE_FEED");
const CERTIFICATE: Option<&str> = option_env!("IRONWOOD_UPDATE_CERTIFICATE");
const PUBLIC_KEY: Option<&str> = option_env!("IRONWOOD_UPDATE_PUBLIC_KEY");
const SIGNING_MODE: Option<&str> = option_env!("IRONWOOD_UPDATE_SIGNING_MODE");
fn trust() -> Option<(&'static str, &'static str)> {
    trust_policy(SIGNING_MODE, CERTIFICATE, PUBLIC_KEY)
}
fn trust_policy(mode: Option<&'static str>, certificate: Option<&'static str>, public_key: Option<&'static str>) -> Option<(&'static str, &'static str)> {
    match mode.unwrap_or("authenticode") {
        "authenticode" if public_key.is_none_or(|v| v.trim().is_empty()) =>
            certificate.filter(|v| !v.trim().is_empty()).map(|v| ("authenticode", v)),
        "private-preview-key" if certificate.is_none_or(|v| v.trim().is_empty()) =>
            public_key.filter(|v| !v.trim().is_empty()).map(|v| ("private-preview-key", v)),
        _ => None,
    }
}
fn private_release_access(feed: &str) -> bool {
    feed != "https://companion-releases.ironwoodchess.com/public/windows-x64/manifest.json"
}
#[cfg(test)]
mod tests {
    use super::trust_policy;
    #[test]
    fn public_feed_does_not_use_private_release_credentials() {
        assert!(!super::private_release_access("https://companion-releases.ironwoodchess.com/public/windows-x64/manifest.json"));
        assert!(super::private_release_access("https://companion-releases.ironwoodchess.com/private-preview/windows-x64/manifest.json"));
        assert!(super::private_release_access("https://other.example/public/windows-x64/manifest.json"));
    }
    #[test]
    fn updater_uses_windows_powershell_security_modules() {
        let folder = std::env::temp_dir().join(format!("ironwood-updater-modules-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let script = folder.join("probe.ps1");
        std::fs::write(&script, "$ErrorActionPreference='Stop'; $s=ConvertTo-SecureString 'updater regression fixture' -AsPlainText -Force; $encrypted=ConvertFrom-SecureString $s; $decoded=ConvertTo-SecureString $encrypted; $decoded.Dispose(); $s.Dispose()").unwrap();
        let mut cmd = super::command(&script);
        assert!(cmd.get_envs().any(|(key, value)| key == "PSModulePath" && value.is_none()));
        let status = cmd.status().unwrap();
        std::fs::remove_file(&script).unwrap();
        std::fs::remove_dir(&folder).unwrap();
        assert!(status.success(), "Windows PowerShell security module must load and decrypt credentials");
    }
    #[test]
    fn private_preview_trust_requires_explicit_mode_and_one_public_pin() {
        assert_eq!(trust_policy(None, Some("cert"), None), Some(("authenticode", "cert")));
        assert_eq!(trust_policy(Some("private-preview-key"), None, Some("public")), Some(("private-preview-key", "public")));
        assert_eq!(trust_policy(None, None, Some("public")), None);
        assert_eq!(trust_policy(Some("private-preview-key"), Some("cert"), Some("public")), None);
        assert_eq!(trust_policy(Some("authenticode"), Some("cert"), Some("public")), None);
        assert_eq!(trust_policy(Some("unknown"), Some("cert"), None), None);
        assert_eq!(trust_policy(Some("private-preview-key"), None, Some(" ")), None);
    }
}
pub fn configured() -> bool {
    FEED.is_some_and(|v| !v.trim().is_empty()) && trust().is_some()
}

#[derive(Clone)]
pub struct Candidate {
    pub directory: PathBuf,
    pub version: String,
}
fn shell_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else {
        value.strip_prefix(r"\\?\").unwrap_or(&value).to_owned()
    }
}
fn command(script: &Path) -> Command {
    let mut cmd = Command::new("powershell.exe");
    cmd.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
    ])
    .arg(script)
    // A Companion launched from PowerShell 7 can inherit its module paths.
    // Windows PowerShell must discover its own compatible security module for DPAPI.
    .env_remove("PSModulePath")
    .creation_flags(0x08000000)
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    cmd
}
pub fn check(data: &Path) -> Result<Option<Candidate>, Error> {
    let (Some(feed), Some((signing_mode, pin))) = (
        FEED.filter(|value| !value.trim().is_empty()),
        trust(),
    ) else {
        return Err("No signed update channel is configured for this development build. Companion has not been changed.".into());
    };
    let base = std::env::current_exe()?
        .parent()
        .ok_or("Missing application folder")?
        .to_path_buf();
    let work = base
        .parent()
        .ok_or("Missing update staging folder")?
        .join(format!(
            ".companion-update-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
    std::fs::create_dir(&work)?;
    let script = work.join("update.ps1");
    std::fs::write(&script, SCRIPT)?;
    let mut check = command(&script);
    check.arg("-Mode")
        .arg("Check")
        .arg("-Work")
        .arg(&work)
        .arg("-Feed")
        .arg(feed)
        .arg("-TrustMode")
        .arg(signing_mode)
        .arg("-CurrentVersion")
        .arg(env!("CARGO_PKG_VERSION"));
    if private_release_access(feed) {
        check.arg("-CredentialFile").arg(data.join(".release-channel-credential"));
    }
    check.arg(if signing_mode == "authenticode" { "-Certificate" } else { "-PublicKey" }).arg(pin);
    let status = check.status()?;
    if !status.success() {
        let message = std::fs::read_to_string(work.join("error.txt")).unwrap_or_else(|_| {
            "Update check failed. The installed application was not changed.".into()
        });
        return Err(message.into());
    }
    let version = std::fs::read_to_string(work.join("available.txt")).unwrap_or_default();
    if version.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(Candidate {
        directory: work,
        version: version.trim().to_owned(),
    }))
}
pub fn install(
    candidate: &Candidate,
    data: &Path,
    config: &Path,
    address: &str,
) -> Result<(), Error> {
    let exe = std::env::current_exe()?;
    let install = exe.parent().ok_or("Missing installation directory")?;
    let (signing_mode, pin) = trust().ok_or("Missing update signing policy")?;
    let request = serde_json::json!({
        "install": shell_path(install), "data": shell_path(&std::fs::canonicalize(data)?),
        "config": shell_path(&std::fs::canonicalize(config)?), "address": address,
        "pid": std::process::id(), "args": ["serve".to_owned(), "--config".to_owned(), shell_path(&std::fs::canonicalize(config)?), "--tray".to_owned()],
        "certificate": if signing_mode == "authenticode" { pin } else { "" },
        "trust_mode": signing_mode,
        "public_key": if signing_mode == "private-preview-key" { pin } else { "" },
        "executable": exe.file_name().ok_or("Missing executable name")?,
    });
    std::fs::write(
        candidate.directory.join("request.json"),
        serde_json::to_vec(&request)?,
    )?;
    let ready = candidate.directory.join("ready-to-stop");
    if ready.exists() {
        std::fs::remove_file(&ready)?;
    }
    let mut helper = command(&candidate.directory.join("update.ps1"))
        .arg("-Mode")
        .arg("Install")
        .arg("-Work")
        .arg(&candidate.directory)
        .spawn()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
    loop {
        if candidate.directory.join("ready-to-stop").is_file() {
            return Ok(());
        }
        if helper.try_wait()?.is_some() {
            let message = std::fs::read_to_string(candidate.directory.join("error.txt"))
                .unwrap_or_else(|_| {
                    "The update helper could not start. Companion is still running.".into()
                });
            return Err(message.into());
        }
        if std::time::Instant::now() >= deadline {
            let _ = helper.kill();
            let _ = helper.wait();
            return Err("Update preparation timed out. Companion is still running.".into());
        }
        thread_sleep();
    }
}
fn thread_sleep() {
    std::thread::sleep(std::time::Duration::from_millis(100));
}

//! Updating ArchiSim from inside itself (`docs/ACTUALIZAR.md`): the latest
//! version published on GitHub, what it changes, its installer for this
//! computer, and how to put it in place. No interface here: the window is
//! `app/update_ui`.

use std::path::{Path, PathBuf};

/// Where the published versions are asked for.
pub const LATEST_URL: &str = "https://api.github.com/repos/stronkel/archisim/releases/latest";

/// The version running.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// The Mac installation, the same line the published page gives: it
/// waits for ArchiSim to close, puts the new one in place and leaves it out
/// of quarantine.
#[cfg(target_os = "macos")]
const MAC_INSTALLER: &str = include_str!("../../../scripts/instalar_mac.sh");

/// A version published.
#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    /// «0.81.0».
    pub version: String,
    /// What changes, as the page of the version says it (Markdown).
    pub notes: String,
    /// Its page on GitHub.
    pub page: String,
    /// The installer for this computer, if there is one: its address and
    /// size in bytes.
    pub installer: Option<(String, u64)>,
}

/// The numbers of a version («0.81.0», «v0.81» → [0, 81, 0]).
pub fn numbers(v: &str) -> Vec<u64> {
    v.trim()
        .trim_start_matches('v')
        .split(['.', '-', '+'])
        .map_while(|p| p.parse::<u64>().ok())
        .collect()
}

/// Whether `other` is newer than `current`.
pub fn is_newer(other: &str, current: &str) -> bool {
    let (mut a, mut b) = (numbers(other), numbers(current));
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a > b
}

/// The name of the installer published for this computer.
pub fn installer_name() -> Option<&'static str> {
    if cfg!(target_os = "windows") {
        Some("ArchiSim-Windows.exe")
    } else if cfg!(target_os = "macos") {
        Some("ArchiSim-Mac.dmg")
    } else {
        None
    }
}

/// Reads GitHub's answer about the latest version.
pub fn read_release(json: &str) -> Result<Release, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("respuesta que no se entiende ({e})"))?;
    let tag = v["tag_name"]
        .as_str()
        .ok_or("la respuesta no dice la versión")?;
    let want = installer_name();
    let installer = v["assets"].as_array().and_then(|a| {
        a.iter().find_map(|x| {
            let name = x["name"].as_str()?;
            (Some(name) == want).then(|| {
                (
                    x["browser_download_url"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    x["size"].as_u64().unwrap_or(0),
                )
            })
        })
    });
    Ok(Release {
        version: tag.trim_start_matches('v').to_string(),
        notes: v["body"].as_str().unwrap_or_default().to_string(),
        page: v["html_url"].as_str().unwrap_or_default().to_string(),
        installer,
    })
}

/// What a version changes, without the installation instructions that the
/// page of the version carries for those who download it by hand.
pub fn changes(notes: &str) -> String {
    let text = match notes.find("## Novedades") {
        Some(i) => &notes[i + "## Novedades".len()..],
        None => notes,
    };
    text.trim().to_string()
}

/// Where the installer is downloaded.
pub fn download_path(version: &str) -> PathBuf {
    let name = installer_name().unwrap_or("ArchiSim");
    let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
    std::env::temp_dir().join(format!("{stem}-{version}.{ext}"))
}

/// Puts the downloaded version in place: starts its installer, which
/// waits for ArchiSim to close and opens it again (with `project`, if
/// given). The caller closes ArchiSim at once.
pub fn start_install(installer: &Path, project: Option<&Path>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new(installer);
        cmd.args(["/SILENT", "/SP-", "/CLOSEAPPLICATIONS", "/actualizar=1"]);
        if let Some(p) = project {
            cmd.arg(format!("/abrir={}", p.display()));
        }
        cmd.spawn()
            .map(|_| ())
            .map_err(|e| format!("no se puede abrir el instalador ({e})"))
    }
    #[cfg(target_os = "macos")]
    {
        // Into the folder this ArchiSim is in (Aplicaciones, usually).
        let app = std::env::current_exe()
            .ok()
            .and_then(|e| {
                e.ancestors()
                    .find(|a| a.extension().is_some_and(|x| x == "app"))
                    .map(Path::to_path_buf)
            })
            .ok_or("ArchiSim no está en una aplicación de Mac: instálala con el instalador")?;
        let dest = app
            .parent()
            .ok_or("no se sabe dónde está ArchiSim")?
            .to_path_buf();
        let script = std::env::temp_dir().join("instalar-archisim.sh");
        std::fs::write(&script, MAC_INSTALLER).map_err(|e| e.to_string())?;
        let reopen = match project {
            Some(p) => format!("open -a {} {}", quote(&dest.join("ArchiSim.app")), quote(p)),
            None => format!("open {}", quote(&dest.join("ArchiSim.app"))),
        };
        let line = format!(
            "ARCHISIM_DMG={} ARCHISIM_DESTINO={} /bin/bash {} > {} 2>&1 && {}",
            quote(installer),
            quote(&dest),
            quote(&script),
            quote(&std::env::temp_dir().join("instalar-archisim.log")),
            reopen
        );
        std::process::Command::new("/usr/bin/nohup")
            .args(["/bin/bash", "-c", &line])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("no se puede instalar ({e})"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = (installer, project);
        Err("en este sistema, ArchiSim se actualiza compilándolo".to_string())
    }
}

/// A path for the shell, in single quotes.
#[cfg(target_os = "macos")]
fn quote(p: &Path) -> String {
    format!("'{}'", p.display().to_string().replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_compared_by_their_numbers() {
        assert!(is_newer("v0.81.0", "0.80.0"));
        assert!(is_newer("0.100.0", "0.99.3"));
        assert!(is_newer("1.0", "0.200.0"));
        assert!(!is_newer("0.80.0", "0.80.0"));
        assert!(!is_newer("v0.80", "0.80.0"));
        assert!(!is_newer("0.79.9", "0.80.0"));
    }

    #[test]
    fn the_latest_version_is_read_from_its_page() {
        let json = r#"{
            "tag_name": "v0.81.0",
            "html_url": "https://github.com/stronkel/archisim/releases/tag/v0.81.0",
            "body": "**Mac**: …\n\n## Novedades\n\n- Despiece a mano.\n",
            "assets": [
                {"name": "ArchiSim-Mac.dmg", "size": 10, "browser_download_url": "https://x/ArchiSim-Mac.dmg"},
                {"name": "ArchiSim-Windows.exe", "size": 20, "browser_download_url": "https://x/ArchiSim-Windows.exe"}
            ]
        }"#;
        let r = read_release(json).unwrap();
        assert_eq!(r.version, "0.81.0");
        assert_eq!(changes(&r.notes), "- Despiece a mano.");
        match installer_name() {
            Some(n) => assert!(r.installer.unwrap().0.ends_with(n)),
            None => assert!(r.installer.is_none()),
        }
        assert!(read_release("{}").is_err());
    }
}

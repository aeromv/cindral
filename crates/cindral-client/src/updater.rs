// SPDX-License-Identifier: MIT OR Apache-2.0

//! Updates from the GitHub releases of Cindral.
//!
//! At start a background thread asks GitHub for the latest release. When it
//! is newer than this build, the title screen offers it; a click downloads
//! the file in the background, checks it against the SHA-256 GitHub lists
//! for it, then installs it and starts the new Cindral:
//!
//! - an installed Cindral (the installer's `unins000.exe` next to the game)
//!   runs the new installer silently, after this process has left, and
//!   starts the game again;
//! - a portable `cindral.exe` renames itself to `cindral.exe.old` (Windows
//!   allows that while it runs), puts the new file in its place and starts
//!   it. The next start deletes the old file.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The latest release of the repository.
const LATEST_RELEASE: &str = "https://api.github.com/repos/aeromv/cindral/releases/latest";
/// GitHub asks API clients for a user agent.
const USER_AGENT: &str = concat!("cindral-updater/", env!("CARGO_PKG_VERSION"));
/// The version of this build.
const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// Where the update stands, for the title screen.
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateState {
    /// Nothing to offer: checking, up to date, offline, or a development
    /// build.
    Idle,
    /// A newer release exists.
    Available {
        /// Its version, `0.9.0`.
        version: String,
    },
    /// The file is downloading; the share done, `0.0..=1.0`.
    Downloading(f32),
    /// The new version is starting: the game must close now.
    Restarting,
    /// The download or the install failed; the button tries again.
    Failed(String),
}

/// One file of a release.
#[derive(Debug, Clone, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
    #[serde(default)]
    size: u64,
}

/// The answer of the releases API.
#[derive(Debug, Clone, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    assets: Vec<Asset>,
}

/// What an update replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// The installed game: the new installer runs.
    Installer,
    /// A loose `cindral.exe`: the new one takes its place.
    Portable,
}

#[derive(Debug)]
struct Shared {
    state: UpdateState,
    file: Option<Asset>,
}

/// The update check and the update itself, both in the background.
#[derive(Debug, Clone)]
pub struct Updater {
    shared: Arc<Mutex<Shared>>,
}

impl Updater {
    /// Starts checking for a newer release.
    #[must_use]
    pub fn start() -> Self {
        remove_old_binary();
        let updater = Self {
            shared: Arc::new(Mutex::new(Shared {
                state: UpdateState::Idle,
                file: None,
            })),
        };
        let shared = Arc::clone(&updater.shared);
        let spawned = std::thread::Builder::new()
            .name("cindral-update-check".to_string())
            .spawn(move || {
                let Some(kind) = install_kind() else {
                    return;
                };
                match check(kind) {
                    Ok(Some((version, file))) => {
                        tracing::info!(%version, file = %file.name, "an update is available");
                        if let Ok(mut shared) = shared.lock() {
                            shared.state = UpdateState::Available { version };
                            shared.file = Some(file);
                        }
                    }
                    Ok(None) => tracing::debug!("Cindral is up to date"),
                    Err(error) => tracing::info!(%error, "could not check for updates"),
                }
            });
        if let Err(error) = spawned {
            tracing::warn!(%error, "could not start the update check");
        }
        updater
    }

    /// Where the update stands now.
    #[must_use]
    pub fn state(&self) -> UpdateState {
        self.shared
            .lock()
            .map(|shared| shared.state.clone())
            .unwrap_or(UpdateState::Idle)
    }

    /// Downloads and installs the offered release, in the background. Does
    /// nothing unless one is offered (or the last try failed).
    pub fn begin(&self) {
        let file = {
            let Ok(mut shared) = self.shared.lock() else {
                return;
            };
            if !matches!(
                shared.state,
                UpdateState::Available { .. } | UpdateState::Failed(_)
            ) {
                return;
            }
            let Some(file) = shared.file.clone() else {
                return;
            };
            shared.state = UpdateState::Downloading(0.0);
            file
        };
        let shared = Arc::clone(&self.shared);
        let spawned = std::thread::Builder::new()
            .name("cindral-update".to_string())
            .spawn(move || {
                let set = |state: UpdateState| {
                    if let Ok(mut shared) = shared.lock() {
                        shared.state = state;
                    }
                };
                let result = install_kind()
                    .ok_or_else(|| "this copy of Cindral cannot update itself".to_string())
                    .and_then(|kind| {
                        let path = download(&file, |done| set(UpdateState::Downloading(done)))?;
                        apply(kind, &path)
                    });
                match result {
                    Ok(()) => {
                        tracing::info!("the update is installing; Cindral restarts");
                        set(UpdateState::Restarting);
                    }
                    Err(error) => {
                        tracing::warn!(%error, "the update failed");
                        set(UpdateState::Failed(error));
                    }
                }
            });
        if let Err(error) = spawned {
            if let Ok(mut shared) = self.shared.lock() {
                shared.state = UpdateState::Failed(error.to_string());
            }
        }
    }
}

/// How this copy is updated; `None` off Windows or when the path of the
/// running program is unknown.
fn install_kind() -> Option<Kind> {
    if !cfg!(windows) {
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    let folder = exe.parent()?;
    Some(if folder.join("unins000.exe").is_file() {
        Kind::Installer
    } else {
        Kind::Portable
    })
}

/// `(major, minor, patch)` of `0.9.0` or `v0.9.0`.
fn parse_version(text: &str) -> Option<(u32, u32, u32)> {
    let mut parts = text.trim().trim_start_matches('v').split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().map_or(Some(0), |patch| {
        patch
            .split(|c: char| !c.is_ascii_digit())
            .next()?
            .parse()
            .ok()
    })?;
    Some((major, minor, patch))
}

/// The file of `release` this copy needs, when the release is newer.
fn pick(release: &Release, kind: Kind) -> Option<(String, Asset)> {
    let latest = parse_version(&release.tag_name)?;
    if latest <= parse_version(CURRENT)? {
        return None;
    }
    let wanted = |asset: &&Asset| match kind {
        Kind::Installer => asset.name.starts_with("cindral-setup-") && asset.name.ends_with(".exe"),
        Kind::Portable => asset.name == "cindral.exe",
    };
    let asset = release.assets.iter().find(wanted)?.clone();
    let version = release.tag_name.trim_start_matches('v').to_string();
    Some((version, asset))
}

fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|error| error.to_string())
}

/// Asks GitHub for the latest release; the version and file to download
/// when it is newer than this build.
fn check(kind: Kind) -> Result<Option<(String, Asset)>, String> {
    runtime()?.block_on(async {
        let response = client()?
            .get(LATEST_RELEASE)
            .header("Accept", "application/vnd.github+json")
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?;
        let release: Release = response.json().await.map_err(|error| error.to_string())?;
        Ok(pick(&release, kind))
    })
}

/// Downloads `file` into the temporary folder, reporting the share done,
/// and checks its SHA-256. Returns the path of the verified file.
fn download(file: &Asset, mut progress: impl FnMut(f32)) -> Result<PathBuf, String> {
    let folder = std::env::temp_dir().join("cindral-update");
    std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let path = folder.join(&file.name);
    let hash = runtime()?.block_on(async {
        let mut response = client()?
            .get(&file.browser_download_url)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?;
        let total = response.content_length().unwrap_or(file.size).max(1);
        let mut out = std::fs::File::create(&path).map_err(|error| error.to_string())?;
        let mut hasher = Sha256::new();
        let mut done = 0_u64;
        while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
            hasher.update(&chunk);
            out.write_all(&chunk).map_err(|error| error.to_string())?;
            done += chunk.len() as u64;
            progress((done as f32 / total as f32).min(1.0));
        }
        out.flush().map_err(|error| error.to_string())?;
        Ok::<_, String>(hex(&hasher.finalize()))
    })?;
    match file
        .digest
        .as_deref()
        .and_then(|digest| digest.strip_prefix("sha256:"))
    {
        Some(expected) if !expected.eq_ignore_ascii_case(&hash) => {
            let _ = std::fs::remove_file(&path);
            Err("the downloaded file does not match its checksum".to_string())
        }
        _ => Ok(path),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Installs the downloaded file and starts the new game; the caller then
/// closes this one.
#[cfg(windows)]
fn apply(kind: Kind, downloaded: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    /// `CREATE_NO_WINDOW`: no console flashes up.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    match kind {
        Kind::Installer => {
            // Wait two seconds for this process to close, install silently,
            // then start the game again.
            let script = format!(
                "/C ping -n 3 127.0.0.1 >nul & start \"\" /wait \"{}\" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- & start \"\" \"{}\"",
                downloaded.display(),
                exe.display()
            );
            std::process::Command::new("cmd")
                .raw_arg(script)
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map_err(|error| error.to_string())?;
        }
        Kind::Portable => {
            let old = old_binary(&exe);
            let _ = std::fs::remove_file(&old);
            std::fs::rename(&exe, &old).map_err(|error| error.to_string())?;
            if let Err(error) = std::fs::copy(downloaded, &exe) {
                // Put the running one back so the game still starts.
                let _ = std::fs::rename(&old, &exe);
                return Err(error.to_string());
            }
            std::process::Command::new(&exe)
                .spawn()
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn apply(_kind: Kind, _downloaded: &Path) -> Result<(), String> {
    Err("updates install on Windows only".to_string())
}

/// `cindral.exe.old` next to `exe`.
fn old_binary(exe: &Path) -> PathBuf {
    let mut name = exe.as_os_str().to_owned();
    name.push(".old");
    PathBuf::from(name)
}

/// Deletes the binary a portable update left behind.
fn remove_old_binary() {
    if let Ok(exe) = std::env::current_exe() {
        let old = old_binary(&exe);
        if old.is_file() {
            let _ = std::fs::remove_file(old);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str) -> Release {
        let asset = |name: &str| Asset {
            name: name.to_string(),
            browser_download_url: format!("https://example.invalid/{name}"),
            digest: None,
            size: 1,
        };
        Release {
            tag_name: tag.to_string(),
            assets: vec![
                asset("cindral-setup-99.0.0.exe"),
                asset("cindral.exe"),
                asset("dxcompiler.dll"),
            ],
        }
    }

    #[test]
    fn versions_compare_by_number_not_by_text() {
        assert_eq!(parse_version("v0.10.2"), Some((0, 10, 2)));
        assert_eq!(parse_version("0.9"), Some((0, 9, 0)));
        assert_eq!(parse_version("1.2.3-beta"), Some((1, 2, 3)));
        assert!(parse_version("v0.10.0") > parse_version("0.9.9"));
        assert_eq!(parse_version("latest"), None);
    }

    #[test]
    fn a_newer_release_offers_the_file_this_copy_needs() {
        let newer = release("v99.0.0");
        let (version, installer) = pick(&newer, Kind::Installer).expect("newer");
        assert_eq!(version, "99.0.0");
        assert_eq!(installer.name, "cindral-setup-99.0.0.exe");
        assert_eq!(pick(&newer, Kind::Portable).unwrap().1.name, "cindral.exe");
        assert!(pick(&release(&format!("v{CURRENT}")), Kind::Installer).is_none());
        assert!(pick(&release("v0.0.1"), Kind::Portable).is_none());
    }

    #[test]
    fn the_checksum_is_written_as_github_lists_it() {
        assert_eq!(
            hex(&Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn the_releases_answer_is_read() {
        let json = r#"{"tag_name":"v0.9.0","assets":[{"name":"cindral.exe","size":5,
            "browser_download_url":"https://github.com/x","digest":"sha256:ab"}]}"#;
        let release: Release = serde_json::from_str(json).expect("parses");
        assert_eq!(release.assets[0].digest.as_deref(), Some("sha256:ab"));
    }

    /// Downloads a real release file from GitHub and checks it against the
    /// SHA-256 GitHub lists; needs the network, so it runs by hand:
    /// `cargo test -p cindral-client updater -- --ignored`.
    #[test]
    #[ignore = "downloads from GitHub"]
    fn a_real_release_file_downloads_and_matches_its_checksum() {
        let release: Release = runtime()
            .unwrap()
            .block_on(async {
                client()?
                    .get("https://api.github.com/repos/aeromv/cindral/releases/tags/v0.8.0")
                    .send()
                    .await
                    .map_err(|error| error.to_string())?
                    .json::<Release>()
                    .await
                    .map_err(|error| error.to_string())
            })
            .expect("the release answers");
        let file = release
            .assets
            .iter()
            .find(|asset| asset.name == "dxcompiler.dll")
            .expect("the file is listed")
            .clone();
        assert!(file.digest.is_some(), "GitHub lists a checksum");
        let mut last = 0.0;
        let path = download(&file, |done| last = done).expect("downloads and matches");
        assert_eq!(last, 1.0);
        assert_eq!(std::fs::metadata(&path).unwrap().len(), file.size);
        let mut wrong = file.clone();
        wrong.digest = Some("sha256:00".to_string());
        assert!(
            download(&wrong, |_| {}).is_err(),
            "a wrong checksum is refused"
        );
    }
}

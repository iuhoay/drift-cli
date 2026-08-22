use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const REPO: &str = "iuhoay/drift-cli";
const USER_AGENT: &str = concat!("drift-cli/", env!("CARGO_PKG_VERSION"));
const META_TIMEOUT_SECS: u64 = 15;
const DOWNLOAD_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Outcome {
    pub current: String,
    pub latest: String,
    pub status: Status,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Updated,
    UpToDate,
}

#[derive(Debug)]
pub enum Error {
    UnsupportedPlatform { os: String, arch: String },
    InvalidVersion(String),
    NotABinary,
    Verify(String),
    Http { status: u16, body: String },
    Network(String),
    Io(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform { os, arch } => write!(
                f,
                "no published binary for {os} {arch} — see https://github.com/{REPO}/releases"
            ),
            Self::InvalidVersion(tag) => write!(f, "unrecognized version: {tag}"),
            Self::NotABinary => write!(f, "downloaded file was not a drift binary"),
            Self::Verify(msg) | Self::Network(msg) | Self::Io(msg) => write!(f, "{msg}"),
            Self::Http { status, body } if body.is_empty() => write!(f, "HTTP {status}"),
            Self::Http { status, body } => write!(f, "HTTP {status}: {body}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub fn perform() -> Result<Outcome, Error> {
    let current_version = parse_version(env!("CARGO_PKG_VERSION"))
        .ok_or_else(|| Error::InvalidVersion(env!("CARGO_PKG_VERSION").into()))?;
    let path = current_binary()?;
    let asset = asset_for(env::consts::OS, env::consts::ARCH).ok_or_else(|| {
        Error::UnsupportedPlatform {
            os: env::consts::OS.into(),
            arch: env::consts::ARCH.into(),
        }
    })?;

    let tag = fetch_latest_tag()?;
    let latest_version = parse_version(&tag).ok_or_else(|| Error::InvalidVersion(tag.clone()))?;

    if latest_version <= current_version {
        return Ok(Outcome {
            current: current_version.to_string(),
            latest: latest_version.to_string(),
            status: Status::UpToDate,
            path: path.display().to_string(),
        });
    }

    let tmp = TempFile(sibling_temp(&path));
    download_to(&download_url(&tag, asset), &tmp.0)?;
    ensure_binary(&tmp.0)?;
    ensure_runs(&tmp.0, &latest_version)?;
    replace_exe(&path, &tmp.0)?;

    Ok(Outcome {
        current: current_version.to_string(),
        latest: latest_version.to_string(),
        status: Status::Updated,
        path: path.display().to_string(),
    })
}

fn current_binary() -> Result<PathBuf, Error> {
    let exe = env::current_exe()
        .map_err(|err| Error::Io(format!("failed to locate current binary: {err}")))?;
    Ok(exe.canonicalize().unwrap_or(exe))
}

fn asset_for(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("macos", "aarch64") => Some("drift-aarch64-apple-darwin"),
        ("linux", "x86_64") => Some("drift-x86_64-unknown-linux-gnu"),
        _ => None,
    }
}

fn parse_version(tag: &str) -> Option<Version> {
    let tag = tag.trim().strip_prefix('v').unwrap_or(tag.trim());
    let mut parts = tag.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(Version {
        major,
        minor,
        patch,
    })
}

fn tag_from_location(location: &str) -> Option<&str> {
    let location = location.trim().trim_end_matches('/');
    let tag = location
        .rsplit('/')
        .next()
        .filter(|part| !part.is_empty())?;
    parse_version(tag)?;
    Some(tag)
}

fn download_url(tag: &str, asset: &str) -> String {
    format!("https://github.com/{REPO}/releases/download/{tag}/{asset}")
}

fn fetch_latest_tag() -> Result<String, Error> {
    match fetch_tag_via_redirect() {
        Ok(tag) => Ok(tag),
        Err(_) => fetch_tag_via_api(),
    }
}

fn fetch_tag_via_redirect() -> Result<String, Error> {
    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout(Duration::from_secs(META_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build();

    let response = match agent
        .head(&format!("https://github.com/{REPO}/releases/latest"))
        .call()
    {
        Ok(response) => response,
        Err(ureq::Error::Status(_, response)) => response,
        Err(ureq::Error::Transport(err)) => {
            return Err(Error::Network(format!("request failed: {err}")));
        }
    };

    response
        .header("location")
        .and_then(tag_from_location)
        .map(ToOwned::to_owned)
        .ok_or_else(|| Error::Network("failed to determine latest version".into()))
}

fn fetch_tag_via_api() -> Result<String, Error> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(META_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build();

    let response = agent
        .get(&format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .set("Accept", "application/vnd.github+json")
        .call();

    match response {
        Ok(response) => {
            let body: ApiRelease = response
                .into_json()
                .map_err(|err| Error::Network(format!("failed to parse latest release: {err}")))?;
            if parse_version(&body.tag_name).is_none() {
                Err(Error::InvalidVersion(body.tag_name))
            } else {
                Ok(body.tag_name)
            }
        }
        Err(ureq::Error::Status(status, response)) => {
            let body = response.into_string().unwrap_or_default();
            Err(Error::Http {
                status,
                body: snippet(&body),
            })
        }
        Err(ureq::Error::Transport(err)) => Err(Error::Network(format!("request failed: {err}"))),
    }
}

#[derive(Debug, Deserialize)]
struct ApiRelease {
    tag_name: String,
}

fn download_to(url: &str, dest: &Path) -> Result<(), Error> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build();

    let response = match agent.get(url).call() {
        Ok(response) => response,
        Err(ureq::Error::Status(status, response)) => {
            let body = response.into_string().unwrap_or_default();
            return Err(Error::Http {
                status,
                body: snippet(&body),
            });
        }
        Err(ureq::Error::Transport(err)) => {
            return Err(Error::Network(format!("request failed: {err}")));
        }
    };

    let mut reader = response.into_reader();
    let mut file = fs::File::create(dest)
        .map_err(|err| Error::Io(format!("failed to write {}: {err}", dest.display())))?;
    io::copy(&mut reader, &mut file)
        .map_err(|err| Error::Io(format!("failed to download update: {err}")))?;
    file.sync_all()
        .map_err(|err| Error::Io(format!("failed to write {}: {err}", dest.display())))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dest, fs::Permissions::from_mode(0o755))
            .map_err(|err| Error::Io(format!("failed to chmod {}: {err}", dest.display())))?;
    }

    Ok(())
}

fn looks_like_binary(magic: &[u8]) -> bool {
    matches!(
        magic,
        [0xCF, 0xFA, 0xED, 0xFE, ..]
            | [0xFE, 0xED, 0xFA, 0xCF, ..]
            | [0xCA, 0xFE, 0xBA, 0xBE, ..]
            | [0x7F, b'E', b'L', b'F', ..]
    )
}

fn ensure_binary(path: &Path) -> Result<(), Error> {
    let mut file = fs::File::open(path)
        .map_err(|err| Error::Io(format!("failed to read {}: {err}", path.display())))?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).map_err(|_| Error::NotABinary)?;
    if looks_like_binary(&magic) {
        Ok(())
    } else {
        Err(Error::NotABinary)
    }
}

fn ensure_runs(path: &Path, expected: &Version) -> Result<(), Error> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|err| Error::Verify(format!("new binary failed to run: {err}")))?;
    if !output.status.success() {
        return Err(Error::Verify("new binary failed to run".into()));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    if text.contains(&expected.to_string()) {
        Ok(())
    } else {
        Err(Error::Verify(format!(
            "new binary reported {text:?}, expected {expected}"
        )))
    }
}

fn sibling_temp(current: &Path) -> PathBuf {
    let name = current
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("drift");
    current.with_file_name(format!(".{name}.{}.new", std::process::id()))
}

fn replace_exe(dest: &Path, src: &Path) -> Result<(), Error> {
    if fs::rename(src, dest).is_ok() {
        return Ok(());
    }

    let name = dest
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("drift");
    let old = dest.with_file_name(format!(".{name}.old"));
    let _ = fs::remove_file(&old);
    fs::rename(dest, &old)
        .map_err(|err| Error::Io(format!("failed to replace {}: {err}", dest.display())))?;
    if let Err(err) = fs::rename(src, dest) {
        let _ = fs::rename(&old, dest);
        return Err(Error::Io(format!(
            "failed to replace {}: {err}",
            dest.display()
        )));
    }
    let _ = fs::remove_file(&old);
    Ok(())
}

fn snippet(body: &str) -> String {
    let collapsed = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = collapsed.chars();
    let taken: String = chars.by_ref().take(200).collect();
    if chars.next().is_some() {
        format!("{taken}…")
    } else {
        taken
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_for_published_targets() {
        assert_eq!(
            asset_for("macos", "aarch64"),
            Some("drift-aarch64-apple-darwin")
        );
        assert_eq!(
            asset_for("linux", "x86_64"),
            Some("drift-x86_64-unknown-linux-gnu")
        );
        assert_eq!(asset_for("macos", "x86_64"), None);
        assert_eq!(asset_for("windows", "x86_64"), None);
    }

    #[test]
    fn parse_version_accepts_optional_v() {
        assert_eq!(
            parse_version("v0.1.2"),
            Some(Version {
                major: 0,
                minor: 1,
                patch: 2
            })
        );
        assert_eq!(parse_version("0.1.2"), parse_version("v0.1.2"));
        assert_eq!(parse_version(" v1.0.0\n"), parse_version("1.0.0"));
    }

    #[test]
    fn parse_version_rejects_garbage() {
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("v"), None);
        assert_eq!(parse_version("0.1"), None);
        assert_eq!(parse_version("0.1.2.3"), None);
        assert_eq!(parse_version("0.1.2-rc1"), None);
    }

    #[test]
    fn newer_tag_should_install() {
        let current = parse_version("0.1.2").unwrap();
        let latest = parse_version("v0.1.3").unwrap();
        assert!(latest > current);
        assert!(current <= latest);
        assert!(current <= parse_version("0.1.2").unwrap());
    }

    #[test]
    fn tag_from_github_location() {
        assert_eq!(
            tag_from_location("https://github.com/iuhoay/drift-cli/releases/tag/v0.1.2"),
            Some("v0.1.2")
        );
        assert_eq!(
            tag_from_location("/iuhoay/drift-cli/releases/tag/v0.1.2\r\n"),
            Some("v0.1.2")
        );
        assert_eq!(
            tag_from_location("https://github.com/iuhoay/drift-cli/releases/latest"),
            None
        );
    }

    #[test]
    fn download_url_uses_release_tag() {
        assert_eq!(
            download_url("v0.1.3", "drift-aarch64-apple-darwin"),
            "https://github.com/iuhoay/drift-cli/releases/download/v0.1.3/drift-aarch64-apple-darwin"
        );
    }

    #[test]
    fn looks_like_macho_and_elf() {
        assert!(looks_like_binary(&[0xCF, 0xFA, 0xED, 0xFE]));
        assert!(looks_like_binary(&[0x7F, b'E', b'L', b'F']));
        assert!(!looks_like_binary(b"<htm"));
        assert!(!looks_like_binary(b"PK\x03\x04"));
    }

    #[test]
    fn replace_exe_overwrites_destination() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("drift");
        let src = dir.path().join(".drift.new");
        fs::write(&dest, b"old").unwrap();
        fs::write(&src, b"new").unwrap();
        replace_exe(&dest, &src).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"new");
        assert!(!src.exists());
    }

    #[test]
    fn outcome_json_uses_snake_case_status() {
        let json = serde_json::to_string(&Outcome {
            current: "0.1.2".into(),
            latest: "0.1.3".into(),
            status: Status::UpToDate,
            path: "/tmp/drift".into(),
        })
        .unwrap();
        assert!(json.contains("\"status\":\"up_to_date\""));
        let json = serde_json::to_string(&Outcome {
            current: "0.1.2".into(),
            latest: "0.1.3".into(),
            status: Status::Updated,
            path: "/tmp/drift".into(),
        })
        .unwrap();
        assert!(json.contains("\"status\":\"updated\""));
    }
}

use reqwest::Client;
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::{
    net::TcpListener,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};
use tokio::fs;

use crate::settings::find_profile;

pub const MINECRAFT_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const MODRINTH_API: &str = "https://api.modrinth.com/v2";
pub const PAPER_API: &str = "https://api.papermc.io/v2";

pub async fn get_json<T: for<'de> Deserialize<'de>>(
    client: &Client,
    url: &str,
) -> Result<T, String> {
    get_json_or(client, url, "요청한 리소스를 찾지 못했습니다.").await
}

pub async fn get_json_or<T: for<'de> Deserialize<'de>>(
    client: &Client,
    url: &str,
    not_found: &str,
) -> Result<T, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("요청 실패: {error}"))?;

    if response.status().as_u16() == 404 {
        return Err(not_found.to_string());
    }

    response
        .error_for_status()
        .map_err(|error| format!("응답 오류: {error}"))?
        .json::<T>()
        .await
        .map_err(|error| format!("JSON 파싱 실패: {error}"))
}

pub async fn download_file(client: &Client, url: &str, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|error| format!("다운로드 폴더 생성 실패: {error}"))?;
    }

    let bytes = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("다운로드 요청 실패: {error}"))?
        .error_for_status()
        .map_err(|error| format!("다운로드 응답 오류: {error}"))?
        .bytes()
        .await
        .map_err(|error| format!("다운로드 읽기 실패: {error}"))?;
    fs::write(path, &bytes)
        .await
        .map_err(|error| format!("파일 저장 실패: {error}"))
}

pub async fn file_matches_sha1(path: &Path, expected: &str) -> bool {
    let Ok(bytes) = fs::read(path).await else {
        return false;
    };
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(expected)
}

pub fn port_available(port: u16) -> bool {
    TcpListener::bind(("0.0.0.0", port)).is_ok()
}

pub fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("앱 데이터 폴더 확인 실패: {error}"))
}

pub fn default_profile_dir(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("servers").join(id))
}

pub fn safe_filename(filename: &str) -> Result<String, String> {
    Path::new(filename)
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "잘못된 파일 이름입니다.".to_string())
}

pub fn safe_relative_path(path: &str) -> bool {
    let path = Path::new(path);
    path.components()
        .all(|part| matches!(part, std::path::Component::Normal(_)))
}

pub fn stable_mc_version(version: &str) -> bool {
    version.starts_with("1.")
        && version
            .chars()
            .all(|character| character.is_ascii_digit() || character == '.')
}

pub fn crash_line(line: &str) -> bool {
    line.contains("---- Minecraft Crash Report ----")
        || line.contains("Crash report")
        || line.contains("This crash report has been saved to")
        || line.contains("Exception in server tick loop")
        || line.contains("Encountered an unexpected exception")
        || line.contains("java.lang.OutOfMemoryError")
        || line.contains("[FATAL]")
        || line.contains("Failed to start the minecraft server")
        || line.contains("Failed to bind to port")
}

pub fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

pub fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect();
    cleaned.trim_matches('-').to_string()
}

pub fn hyphenate_uuid(raw: &str) -> String {
    let id = raw.replace('-', "");
    if id.len() != 32 {
        return raw.to_string();
    }
    format!(
        "{}-{}-{}-{}-{}",
        &id[0..8],
        &id[8..12],
        &id[12..16],
        &id[16..20],
        &id[20..32]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_error_log_is_not_crash() {
        assert!(!crash_line(
            "[12:00:00 ERROR]: Could not pass event PlayerJoinEvent to ExamplePlugin"
        ));
    }

    #[test]
    fn generic_exception_log_is_not_crash() {
        assert!(!crash_line(
            "[12:00:00 WARN]: java.lang.IllegalArgumentException: plugin handled this"
        ));
    }

    #[test]
    fn minecraft_crash_report_is_crash() {
        assert!(crash_line("---- Minecraft Crash Report ----"));
        assert!(crash_line(
            "This crash report has been saved to: /server/crash-reports/crash.txt"
        ));
    }
}

pub fn open_path(path: &Path) -> Result<(), String> {
    let status = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(path).status()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("explorer").arg(path).status()
    } else {
        std::process::Command::new("xdg-open").arg(path).status()
    }
    .map_err(|error| format!("폴더 열기 실패: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("폴더 열기 명령이 실패했습니다: {status}"))
    }
}

#[tauri::command]
pub async fn open_server_path(
    app: AppHandle,
    profile_id: String,
    target: String,
) -> Result<(), String> {
    let profile = find_profile(&app, &profile_id).await?;
    let base = Path::new(&profile.server_dir);
    let path = match target.as_str() {
        "server" => base.to_path_buf(),
        "backups" => base.join("backups"),
        "logs" => base.join("logs"),
        "plugins" => base.join("plugins"),
        _ => base.to_path_buf(),
    };
    fs::create_dir_all(&path)
        .await
        .map_err(|error| format!("폴더 생성 실패: {error}"))?;
    open_path(&path)
}

use reqwest::Client;
use serde::Deserialize;
use sha1::{Digest, Sha1};
use sha2::Sha256;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, UdpSocket},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, State};
use tokio::{fs, net::TcpStream, time::Duration};

use crate::{
    config::read_config_bundle,
    models::{AppState, NetworkDiagnostics, SystemMetrics, UpnpMappingResult},
    settings::find_profile,
};

pub const MINECRAFT_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const MODRINTH_API: &str = "https://api.modrinth.com/v2";
pub const PAPER_API: &str = "https://api.papermc.io/v2";

static UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(0);

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
    let bytes = download_bytes(client, url).await?;
    write_file_atomic(path, &bytes).await
}

pub async fn download_file_with_sha1(
    client: &Client,
    url: &str,
    path: &Path,
    expected_sha1: &str,
) -> Result<(), String> {
    let bytes = download_bytes(client, url).await?;
    if !sha1_matches(&bytes, expected_sha1) {
        return Err("다운로드한 파일의 SHA1 해시가 일치하지 않습니다.".to_string());
    }
    write_file_atomic(path, &bytes).await
}

pub async fn download_file_with_sha256(
    client: &Client,
    url: &str,
    path: &Path,
    expected_sha256: &str,
) -> Result<(), String> {
    let bytes = download_bytes(client, url).await?;
    if !sha256_matches(&bytes, expected_sha256) {
        return Err("다운로드한 파일의 SHA256 해시가 일치하지 않습니다.".to_string());
    }
    write_file_atomic(path, &bytes).await
}

async fn download_bytes(client: &Client, url: &str) -> Result<Vec<u8>, String> {
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
    if bytes.is_empty() {
        return Err("다운로드한 파일이 비어 있습니다.".to_string());
    }
    Ok(bytes.to_vec())
}

pub async fn write_file_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|error| format!("파일 폴더 생성 실패: {error}"))?;
    }

    let temp = temporary_file_path(path);
    fs::write(&temp, bytes)
        .await
        .map_err(|error| format!("임시 파일 저장 실패: {error}"))?;

    if let Err(first_error) = fs::rename(&temp, path).await {
        if path.exists() {
            fs::remove_file(path)
                .await
                .map_err(|error| format!("기존 파일 교체 준비 실패: {error}"))?;
        }
        fs::rename(&temp, path)
            .await
            .map_err(|error| format!("파일 교체 실패: {error}; 최초 오류: {first_error}"))?;
    }

    Ok(())
}

pub async fn file_matches_sha1(path: &Path, expected: &str) -> bool {
    let Ok(bytes) = fs::read(path).await else {
        return false;
    };
    sha1_matches(&bytes, expected)
}

pub async fn file_matches_sha256(path: &Path, expected: &str) -> bool {
    let Ok(bytes) = fs::read(path).await else {
        return false;
    };
    sha256_matches(&bytes, expected)
}

pub async fn file_nonempty(path: &Path) -> bool {
    fs::metadata(path)
        .await
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
}

fn sha1_matches(bytes: &[u8], expected: &str) -> bool {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(expected)
}

fn sha256_matches(bytes: &[u8], expected: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(expected)
}

fn temporary_file_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download");
    path.with_file_name(format!(".{name}.{}.tmp", unique_suffix()))
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

pub fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

pub fn unique_id(prefix: &str) -> String {
    format!("{prefix}-{}", unique_suffix())
}

fn unique_suffix() -> String {
    let counter = UNIQUE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{counter}", timestamp_millis())
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

#[derive(Deserialize)]
struct PublicIpResponse {
    ip: String,
}

#[tauri::command]
pub async fn network_diagnostics(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<NetworkDiagnostics, String> {
    let profile = find_profile(&app, &profile_id).await?;
    let config = read_config_bundle(&profile).await?;
    let port = config.properties.server_port;
    let local_address = local_ipv4().map(|ip| ip.to_string());
    let public_address = public_ip(&state).await.ok();
    let local_reachable = tcp_reachable(IpAddr::from([127, 0, 0, 1]), port).await;
    let external_reachable = match public_address.as_deref().and_then(|ip| ip.parse().ok()) {
        Some(ip) => Some(tcp_reachable(ip, port).await),
        None => None,
    };
    let lan_endpoint = local_address.as_ref().map(|ip| endpoint(ip, port));
    let public_endpoint = public_address.as_ref().map(|ip| endpoint(ip, port));

    Ok(NetworkDiagnostics {
        port,
        local_address,
        public_address,
        lan_endpoint,
        public_endpoint,
        local_reachable,
        external_reachable,
        note: "외부 접속 확인은 공유기의 NAT loopback 지원 여부에 따라 실제 외부 결과와 다를 수 있습니다."
            .to_string(),
        checked_at: timestamp(),
    })
}

#[tauri::command]
pub async fn open_upnp_port(
    app: AppHandle,
    profile_id: String,
) -> Result<UpnpMappingResult, String> {
    let profile = find_profile(&app, &profile_id).await?;
    let config = read_config_bundle(&profile).await?;
    let port = config.properties.server_port;
    let local_ip = local_ipv4().ok_or_else(|| "로컬 IPv4 주소를 찾지 못했습니다.".to_string())?;

    tauri::async_runtime::spawn_blocking(move || {
        let gateway = igd::search_gateway(Default::default())
            .map_err(|error| format!("UPnP 게이트웨이 검색 실패: {error}"))?;
        gateway
            .add_port(
                igd::PortMappingProtocol::TCP,
                port,
                SocketAddrV4::new(local_ip, port),
                0,
                "Minehub Server Launcher",
            )
            .map_err(|error| format!("UPnP 포트 매핑 실패: {error}"))?;
        let external_address = gateway.get_external_ip().ok().map(|ip| ip.to_string());

        Ok(UpnpMappingResult {
            external_address,
            internal_address: local_ip.to_string(),
            external_port: port,
            internal_port: port,
            protocol: "TCP".to_string(),
            note: "공유기에서 UPnP가 활성화되어 있어야 유지됩니다.".to_string(),
        })
    })
    .await
    .map_err(|error| format!("UPnP 작업 실패: {error}"))?
}

#[tauri::command]
pub async fn system_metrics(state: State<'_, AppState>) -> Result<SystemMetrics, String> {
    let mut system = state.system.lock().await;
    system.refresh_cpu();
    system.refresh_memory();

    let memory_total_mb = system.total_memory() / 1024 / 1024;
    let memory_used_mb = system.used_memory() / 1024 / 1024;
    let memory_usage = if memory_total_mb == 0 {
        0.0
    } else {
        (memory_used_mb as f32 / memory_total_mb as f32) * 100.0
    };

    Ok(SystemMetrics {
        cpu_usage: system.global_cpu_info().cpu_usage(),
        memory_used_mb,
        memory_total_mb,
        memory_usage,
        sampled_at: timestamp(),
    })
}

fn local_ipv4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if !ip.is_loopback() => Some(ip),
        _ => None,
    }
}

async fn public_ip(state: &State<'_, AppState>) -> Result<String, String> {
    state
        .http
        .get("https://api.ipify.org?format=json")
        .send()
        .await
        .map_err(|error| format!("공인 IP 확인 실패: {error}"))?
        .error_for_status()
        .map_err(|error| format!("공인 IP 응답 오류: {error}"))?
        .json::<PublicIpResponse>()
        .await
        .map(|response| response.ip)
        .map_err(|error| format!("공인 IP 응답 파싱 실패: {error}"))
}

async fn tcp_reachable(ip: IpAddr, port: u16) -> bool {
    tokio::time::timeout(
        Duration::from_millis(1500),
        TcpStream::connect(SocketAddr::new(ip, port)),
    )
    .await
    .is_ok_and(|result| result.is_ok())
}

fn endpoint(address: &str, port: u16) -> String {
    if address.contains(':') {
        format!("[{address}]:{port}")
    } else {
        format!("{address}:{port}")
    }
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

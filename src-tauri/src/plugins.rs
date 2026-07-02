use crate::{
    models::{
        AppState, InstalledPlugin, ModrinthFile, ModrinthProject, ModrinthSearchResponse,
        ModrinthVersion, PluginFile, PluginUpdateInfo, PluginUpdateSummary, ServerKind,
        ServerProfile, UpdatedPlugin,
    },
    settings::find_profile,
    system::{
        download_file, download_file_with_sha1, file_sha1, get_json, safe_filename, sanitize_name,
        timestamp, MODRINTH_API,
    },
};
use reqwest::Client;
use reqwest::Url;
use std::{
    collections::BTreeMap,
    io::ErrorKind,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, State};
use tokio::fs;

const PLUGIN_BACKUP_DIR: &str = ".minehub-backups";

#[tauri::command]
pub async fn search_modrinth(
    state: State<'_, AppState>,
    query: String,
    game_version: String,
    loader: String,
) -> Result<Vec<ModrinthProject>, String> {
    let facets = serde_json::json!([
        ["project_type:plugin"],
        [format!("versions:{game_version}")],
        [format!("categories:{loader}")],
        ["server_side:required", "server_side:optional"]
    ])
    .to_string();
    let mut url = Url::parse(&format!("{MODRINTH_API}/search"))
        .map_err(|error| format!("Modrinth URL 생성 실패: {error}"))?;
    url.query_pairs_mut()
        .append_pair("query", query.trim())
        .append_pair("facets", &facets)
        .append_pair("index", "downloads")
        .append_pair("limit", "25");

    let response: ModrinthSearchResponse = get_json(&state.http, url.as_str()).await?;
    Ok(response.hits)
}

#[tauri::command]
pub async fn install_modrinth_plugin(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
    project_id: String,
    title: String,
    loader: String,
) -> Result<InstalledPlugin, String> {
    let profile = find_profile(&app, &profile_id).await?;
    if matches!(profile.kind, ServerKind::Vanilla) {
        return Err("Vanilla 서버는 플러그인을 로드할 수 없습니다.".to_string());
    }

    let mut url = Url::parse(&format!("{MODRINTH_API}/project/{project_id}/version"))
        .map_err(|error| format!("Modrinth URL 생성 실패: {error}"))?;
    url.query_pairs_mut()
        .append_pair(
            "loaders",
            &serde_json::to_string(&vec![loader]).map_err(|error| error.to_string())?,
        )
        .append_pair(
            "game_versions",
            &serde_json::to_string(&vec![profile.minecraft_version.clone()])
                .map_err(|error| error.to_string())?,
        )
        .append_pair("include_changelog", "false");

    let versions: Vec<ModrinthVersion> = get_json(&state.http, url.as_str()).await?;
    let version = versions
        .iter()
        .find(|version| version.version_type == "release")
        .or_else(|| versions.first())
        .ok_or_else(|| "선택한 버전에 맞는 Modrinth 릴리스를 찾지 못했습니다.".to_string())?;
    let file = version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| "다운로드 가능한 플러그인 파일이 없습니다.".to_string())?;
    let filename = safe_filename(&file.filename)?;
    let plugins_dir = Path::new(&profile.server_dir).join("plugins");
    fs::create_dir_all(&plugins_dir)
        .await
        .map_err(|error| format!("plugins 폴더 생성 실패: {error}"))?;
    let path = plugins_dir.join(&filename);
    if let Some(sha1) = file
        .hashes
        .as_ref()
        .and_then(|hashes| hashes.sha1.as_deref())
    {
        download_file_with_sha1(&state.http, &file.url, &path, sha1).await?;
    } else {
        download_file(&state.http, &file.url, &path).await?;
    }

    Ok(InstalledPlugin {
        title,
        version: version.version_number.clone(),
        filename,
        path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn list_plugins(app: AppHandle, profile_id: String) -> Result<Vec<PluginFile>, String> {
    let profile = find_profile(&app, &profile_id).await?;
    list_plugin_files(&profile).await
}

#[tauri::command]
pub async fn check_plugin_updates(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<PluginUpdateSummary, String> {
    let profile = find_profile(&app, &profile_id).await?;
    if matches!(profile.kind, ServerKind::Vanilla) {
        return Ok(plugin_update_summary(Vec::new()));
    }

    let mut plugins = list_plugin_files(&profile).await?;
    let dir = plugin_dir(&profile);
    let mut hashes = Vec::with_capacity(plugins.len());
    let mut plugin_hashes = Vec::with_capacity(plugins.len());

    for (index, plugin) in plugins.iter().enumerate() {
        let hash = file_sha1(&dir.join(&plugin.filename)).await?;
        hashes.push(hash.clone());
        plugin_hashes.push((index, hash));
    }

    let versions = check_latest_versions_by_hash(
        &state.http,
        &hashes,
        profile.kind.as_str(),
        &profile.minecraft_version,
    )
    .await?;

    for (index, hash) in plugin_hashes {
        let Some(version) = versions.get(&hash) else {
            continue;
        };
        let Ok(file) = primary_or_first_file(version) else {
            continue;
        };
        plugins[index].update = Some(plugin_update_info(&hash, version, file));
    }

    Ok(plugin_update_summary(plugins))
}

#[tauri::command]
pub async fn install_plugin_update(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
    filename: String,
) -> Result<UpdatedPlugin, String> {
    {
        let runtime = state.runtime.lock().await;
        if runtime.current_profile_id.as_deref() == Some(profile_id.as_str()) {
            return Err("서버를 중지한 뒤 플러그인을 업데이트해 주세요.".to_string());
        }
    }

    let profile = find_profile(&app, &profile_id).await?;
    if matches!(profile.kind, ServerKind::Vanilla) {
        return Err("Vanilla 서버는 플러그인을 로드할 수 없습니다.".to_string());
    }

    let filename = safe_filename(&filename)?;
    if !is_plugin_filename(&filename) {
        return Err("플러그인 파일이 아닙니다.".to_string());
    }

    let dir = plugin_dir(&profile);
    let source = dir.join(&filename);
    let metadata = fs::metadata(&source)
        .await
        .map_err(|error| format!("플러그인 파일 확인 실패: {error}"))?;
    if !metadata.is_file() {
        return Err("플러그인 파일이 아닙니다.".to_string());
    }

    let hash = file_sha1(&source).await?;
    let versions = check_latest_versions_by_hash(
        &state.http,
        std::slice::from_ref(&hash),
        profile.kind.as_str(),
        &profile.minecraft_version,
    )
    .await?;
    let version = versions
        .get(&hash)
        .ok_or_else(|| "Modrinth에서 업데이트 정보를 찾지 못했습니다.".to_string())?;
    let file = primary_or_first_file(version)?.clone();
    let latest_hash = file
        .hashes
        .as_ref()
        .and_then(|hashes| hashes.sha1.as_deref());
    if latest_hash.is_some_and(|latest| latest.eq_ignore_ascii_case(&hash)) {
        return Err("이미 최신 플러그인입니다.".to_string());
    }

    let latest_filename = safe_filename(&file.filename)?;
    let target_filename = plugin_filename_for_state(&latest_filename, plugin_enabled(&filename));
    if !is_plugin_filename(&target_filename) {
        return Err("다운로드한 파일 이름이 플러그인 JAR가 아닙니다.".to_string());
    }
    let target = dir.join(&target_filename);
    if source != target && file_exists(&target).await? {
        return Err("동일한 이름의 플러그인 파일이 이미 있습니다.".to_string());
    }

    let backup_dir = dir.join(PLUGIN_BACKUP_DIR);
    fs::create_dir_all(&backup_dir)
        .await
        .map_err(|error| format!("플러그인 백업 폴더 생성 실패: {error}"))?;
    let backup = backup_dir.join(backup_filename(&filename));
    fs::copy(&source, &backup)
        .await
        .map_err(|error| format!("기존 플러그인 백업 실패: {error}"))?;

    if let Some(sha1) = latest_hash {
        download_file_with_sha1(&state.http, &file.url, &target, sha1).await?;
    } else {
        download_file(&state.http, &file.url, &target).await?;
    }

    if source != target {
        if let Err(error) = fs::remove_file(&source).await {
            let _ = fs::remove_file(&target).await;
            return Err(format!("기존 플러그인 교체 실패: {error}"));
        }
    }

    Ok(UpdatedPlugin {
        filename: target_filename.clone(),
        display_name: plugin_display_name(&target_filename),
        version: version.version_number.clone(),
        backup_path: backup.to_string_lossy().to_string(),
        path: target.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn set_plugin_enabled(
    app: AppHandle,
    profile_id: String,
    filename: String,
    enabled: bool,
) -> Result<Vec<PluginFile>, String> {
    let profile = find_profile(&app, &profile_id).await?;
    let plugins_dir = plugin_dir(&profile);
    let filename = safe_filename(&filename)?;
    if !is_plugin_filename(&filename) {
        return Err("플러그인 파일이 아닙니다.".to_string());
    }
    let source = plugins_dir.join(&filename);
    let target = plugins_dir.join(plugin_filename_for_state(&filename, enabled));

    if source != target {
        fs::rename(&source, &target)
            .await
            .map_err(|error| format!("플러그인 상태 변경 실패: {error}"))?;
    }

    list_plugins(app, profile_id).await
}

async fn list_plugin_files(profile: &ServerProfile) -> Result<Vec<PluginFile>, String> {
    let dir = plugin_dir(profile);
    let mut plugins = Vec::new();
    let Ok(mut entries) = fs::read_dir(&dir).await else {
        return Ok(plugins);
    };

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|error| format!("plugins 폴더 읽기 실패: {error}"))?
    {
        let meta = entry
            .metadata()
            .await
            .map_err(|error| format!("플러그인 메타데이터 읽기 실패: {error}"))?;
        let filename = entry.file_name().to_string_lossy().to_string();
        if !meta.is_file() || !is_plugin_filename(&filename) {
            continue;
        }
        plugins.push(PluginFile {
            display_name: plugin_display_name(&filename),
            enabled: plugin_enabled(&filename),
            filename,
            size: meta.len(),
            update: None,
        });
    }

    plugins.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(plugins)
}

fn plugin_dir(profile: &ServerProfile) -> PathBuf {
    Path::new(&profile.server_dir).join("plugins")
}

fn is_plugin_filename(name: &str) -> bool {
    name.ends_with(".jar") || name.ends_with(".jar.disabled")
}

fn plugin_enabled(filename: &str) -> bool {
    filename.ends_with(".jar")
}

fn enabled_filename(filename: &str) -> &str {
    filename.strip_suffix(".disabled").unwrap_or(filename)
}

fn disabled_filename(filename: &str) -> String {
    format!("{}.disabled", enabled_filename(filename))
}

fn plugin_filename_for_state(filename: &str, enabled: bool) -> String {
    if enabled {
        enabled_filename(filename).to_string()
    } else {
        disabled_filename(filename)
    }
}

fn plugin_display_name(filename: &str) -> String {
    enabled_filename(filename)
        .trim_end_matches(".jar")
        .to_string()
}

fn backup_filename(filename: &str) -> String {
    let display_name = plugin_display_name(filename);
    let name = sanitize_name(&display_name);
    format!(
        "{}-{}-{filename}",
        if name.is_empty() { "plugin" } else { &name },
        timestamp()
    )
}

async fn file_exists(path: &Path) -> Result<bool, String> {
    match fs::metadata(path).await {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("플러그인 파일 확인 실패: {error}")),
    }
}

fn primary_or_first_file(version: &ModrinthVersion) -> Result<&ModrinthFile, String> {
    version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| "다운로드 가능한 플러그인 파일이 없습니다.".to_string())
}

fn plugin_update_info(
    current_hash: &str,
    version: &ModrinthVersion,
    file: &ModrinthFile,
) -> PluginUpdateInfo {
    let latest_hash = file
        .hashes
        .as_ref()
        .and_then(|hashes| hashes.sha1.as_deref());
    let available = latest_hash
        .map(|hash| !hash.eq_ignore_ascii_case(current_hash))
        .unwrap_or(true);

    PluginUpdateInfo {
        available,
        current_hash: current_hash.to_string(),
        project_id: version.project_id.clone(),
        current_version_id: (!available).then(|| version.id.clone()),
        current_version: (!available).then(|| version.version_number.clone()),
        latest_version_id: version.id.clone(),
        latest_version: version.version_number.clone(),
        latest_filename: file.filename.clone(),
        latest_size: file.size.unwrap_or(0),
        date_published: version.date_published.clone(),
        note: None,
    }
}

fn plugin_update_summary(plugins: Vec<PluginFile>) -> PluginUpdateSummary {
    let updatable = plugins
        .iter()
        .filter(|plugin| {
            plugin
                .update
                .as_ref()
                .is_some_and(|update| update.available)
        })
        .count();
    let unsupported = plugins
        .iter()
        .filter(|plugin| plugin.update.is_none())
        .count();

    PluginUpdateSummary {
        checked_at: timestamp(),
        total: plugins.len(),
        updatable,
        unsupported,
        plugins,
    }
}

async fn check_latest_versions_by_hash(
    client: &Client,
    hashes: &[String],
    loader: &str,
    game_version: &str,
) -> Result<BTreeMap<String, ModrinthVersion>, String> {
    if hashes.is_empty() {
        return Ok(BTreeMap::new());
    }

    let url = format!("{MODRINTH_API}/version_files/update");
    let body = serde_json::json!({
        "hashes": hashes,
        "algorithm": "sha1",
        "loaders": [loader],
        "game_versions": [game_version]
    });

    client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|error| format!("Modrinth 업데이트 확인 실패: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Modrinth 업데이트 응답 오류: {error}"))?
        .json::<BTreeMap<String, ModrinthVersion>>()
        .await
        .map_err(|error| format!("Modrinth 업데이트 응답 파싱 실패: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ModrinthHashes;

    fn modrinth_file(filename: &str, sha1: Option<&str>, primary: bool) -> ModrinthFile {
        ModrinthFile {
            url: "https://example.invalid/plugin.jar".to_string(),
            filename: filename.to_string(),
            primary,
            size: Some(123),
            hashes: sha1.map(|sha1| ModrinthHashes {
                sha1: Some(sha1.to_string()),
            }),
        }
    }

    fn modrinth_version(files: Vec<ModrinthFile>) -> ModrinthVersion {
        ModrinthVersion {
            id: "version-id".to_string(),
            project_id: "project-id".to_string(),
            version_number: "1.2.3".to_string(),
            version_type: "release".to_string(),
            date_published: Some("2026-01-01T00:00:00Z".to_string()),
            game_versions: vec!["1.21.4".to_string()],
            loaders: vec!["paper".to_string()],
            files,
        }
    }

    #[test]
    fn plugin_filename_detection_matches_supported_extensions() {
        assert!(is_plugin_filename("Example.jar"));
        assert!(is_plugin_filename("Example.jar.disabled"));
        assert!(!is_plugin_filename("Example.txt"));
    }

    #[test]
    fn disabled_update_filename_keeps_disabled_state() {
        assert_eq!(
            plugin_filename_for_state("NewPlugin.jar", false),
            "NewPlugin.jar.disabled"
        );
    }

    #[test]
    fn primary_or_first_file_prefers_primary_file() {
        let version = modrinth_version(vec![
            modrinth_file("secondary.jar", Some("a"), false),
            modrinth_file("primary.jar", Some("b"), true),
        ]);
        assert_eq!(
            primary_or_first_file(&version).unwrap().filename,
            "primary.jar"
        );
    }

    #[test]
    fn primary_or_first_file_falls_back_to_first_file() {
        let version = modrinth_version(vec![modrinth_file("first.jar", Some("a"), false)]);
        assert_eq!(
            primary_or_first_file(&version).unwrap().filename,
            "first.jar"
        );
    }

    #[test]
    fn plugin_update_info_is_current_when_hash_matches() {
        let version = modrinth_version(vec![modrinth_file("latest.jar", Some("abc"), true)]);
        let info = plugin_update_info("abc", &version, primary_or_first_file(&version).unwrap());
        assert!(!info.available);
        assert_eq!(info.current_version.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn plugin_update_info_is_available_when_hash_differs() {
        let version = modrinth_version(vec![modrinth_file("latest.jar", Some("def"), true)]);
        let info = plugin_update_info("abc", &version, primary_or_first_file(&version).unwrap());
        assert!(info.available);
        assert!(info.current_version.is_none());
    }
}

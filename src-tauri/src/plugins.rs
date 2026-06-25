use crate::{
    models::{
        AppState, InstalledPlugin, ModrinthProject, ModrinthSearchResponse, ModrinthVersion,
        PluginFile, ServerKind,
    },
    settings::find_profile,
    system::{download_file, get_json, safe_filename, MODRINTH_API},
};
use reqwest::Url;
use std::path::Path;
use tauri::{AppHandle, State};
use tokio::fs;

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
    download_file(&state.http, &file.url, &path).await?;

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
    let dir = Path::new(&profile.server_dir).join("plugins");
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
        if !meta.is_file() {
            continue;
        }
        let filename = entry.file_name().to_string_lossy().to_string();
        let enabled = filename.ends_with(".jar");
        if !enabled && !filename.ends_with(".jar.disabled") {
            continue;
        }
        let display_name = filename
            .trim_end_matches(".disabled")
            .trim_end_matches(".jar")
            .to_string();
        plugins.push(PluginFile {
            filename,
            display_name,
            enabled,
            size: meta.len(),
        });
    }

    plugins.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(plugins)
}

#[tauri::command]
pub async fn set_plugin_enabled(
    app: AppHandle,
    profile_id: String,
    filename: String,
    enabled: bool,
) -> Result<Vec<PluginFile>, String> {
    let profile = find_profile(&app, &profile_id).await?;
    let plugins_dir = Path::new(&profile.server_dir).join("plugins");
    let filename = safe_filename(&filename)?;
    let source = plugins_dir.join(&filename);
    let target = if enabled {
        plugins_dir.join(filename.trim_end_matches(".disabled"))
    } else {
        plugins_dir.join(format!("{filename}.disabled"))
    };

    if source != target {
        fs::rename(&source, &target)
            .await
            .map_err(|error| format!("플러그인 상태 변경 실패: {error}"))?;
    }

    list_plugins(app, profile_id).await
}

use crate::{
    models::{AppSettings, CreateProfileInput, ServerProfile, ServerProperties},
    system::{app_data_dir, default_profile_dir, timestamp},
};
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use tokio::fs;

#[tauri::command]
pub async fn list_profiles(app: AppHandle) -> Result<Vec<ServerProfile>, String> {
    Ok(load_settings(&app).await?.profiles)
}

#[tauri::command]
pub async fn create_profile(
    app: AppHandle,
    input: CreateProfileInput,
) -> Result<ServerProfile, String> {
    let mut settings = load_settings(&app).await?;
    let id = format!("profile-{}", timestamp());
    let server_dir = match input.server_dir {
        Some(path) if !path.trim().is_empty() => path,
        _ => default_profile_dir(&app, &id)?
            .to_string_lossy()
            .to_string(),
    };
    let name = if input.name.trim().is_empty() {
        format!("{} {}", input.kind.label(), input.minecraft_version)
    } else {
        input.name.trim().to_string()
    };

    let profile = ServerProfile {
        id: id.clone(),
        name,
        kind: input.kind,
        minecraft_version: input.minecraft_version,
        server_dir,
        memory_mb: input.memory_mb.max(512),
        java_path: input.java_path.filter(|path| !path.trim().is_empty()),
        last_used: None,
        settings: ServerProperties::default(),
    };

    fs::create_dir_all(&profile.server_dir)
        .await
        .map_err(|error| format!("서버 폴더 생성 실패: {error}"))?;
    settings.selected_profile_id = Some(id);
    settings.profiles.push(profile.clone());
    save_settings(&app, &settings).await?;
    Ok(profile)
}

#[tauri::command]
pub async fn update_profile(
    app: AppHandle,
    profile: ServerProfile,
) -> Result<ServerProfile, String> {
    let mut settings = load_settings(&app).await?;
    let index = settings
        .profiles
        .iter()
        .position(|item| item.id == profile.id)
        .ok_or_else(|| "프로필을 찾지 못했습니다.".to_string())?;

    fs::create_dir_all(&profile.server_dir)
        .await
        .map_err(|error| format!("서버 폴더 생성 실패: {error}"))?;
    settings.profiles[index] = profile.clone();
    settings.selected_profile_id = Some(profile.id.clone());
    save_settings(&app, &settings).await?;
    Ok(profile)
}

#[tauri::command]
pub async fn choose_server_directory(app: AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(app
            .dialog()
            .file()
            .blocking_pick_folder()
            .map(|path| path.to_string()))
    })
    .await
    .map_err(|error| format!("폴더 선택 실패: {error}"))?
}

pub async fn load_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(app)?;
    let Ok(content) = fs::read_to_string(&path).await else {
        return Ok(AppSettings::default());
    };
    serde_json::from_str(&content).map_err(|error| format!("settings.json 읽기 실패: {error}"))
}

pub async fn save_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|error| format!("앱 설정 폴더 생성 실패: {error}"))?;
    }
    let content = serde_json::to_string_pretty(settings)
        .map_err(|error| format!("앱 설정 직렬화 실패: {error}"))?;
    fs::write(path, content)
        .await
        .map_err(|error| format!("settings.json 저장 실패: {error}"))
}

pub async fn find_profile(app: &AppHandle, profile_id: &str) -> Result<ServerProfile, String> {
    load_settings(app)
        .await?
        .profiles
        .into_iter()
        .find(|profile| profile.id == profile_id)
        .ok_or_else(|| "프로필을 찾지 못했습니다.".to_string())
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("settings.json"))
}

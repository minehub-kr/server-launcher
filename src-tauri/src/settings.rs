use crate::{
    models::{AppSettings, AppState, CreateProfileInput, ServerProfile, ServerProperties},
    system::{app_data_dir, default_profile_dir, unique_id, write_file_atomic},
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::fs;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteProfileResult {
    pub profiles: Vec<ServerProfile>,
    pub file_delete_error: Option<String>,
}

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
    let id = next_profile_id(&settings);
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
pub async fn delete_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
    delete_files: bool,
) -> Result<DeleteProfileResult, String> {
    {
        let runtime = state.runtime.lock().await;
        if runtime.current_profile_id.as_deref() == Some(profile_id.as_str()) {
            return Err("실행 중인 프로필은 삭제할 수 없습니다.".to_string());
        }
    }

    let mut settings = load_settings(&app).await?;
    let index = settings
        .profiles
        .iter()
        .position(|profile| profile.id == profile_id)
        .ok_or_else(|| "프로필을 찾지 못했습니다.".to_string())?;
    let profile = settings.profiles.remove(index);

    if settings.selected_profile_id.as_deref() == Some(profile.id.as_str())
        || settings
            .selected_profile_id
            .as_ref()
            .is_some_and(|id| !settings.profiles.iter().any(|profile| &profile.id == id))
    {
        settings.selected_profile_id = settings.profiles.first().map(|profile| profile.id.clone());
    }

    save_settings(&app, &settings).await?;

    let file_delete_error = if delete_files {
        delete_profile_dir(&app, &profile.id, &profile.server_dir)
            .await
            .err()
    } else {
        None
    };

    Ok(DeleteProfileResult {
        profiles: settings.profiles,
        file_delete_error,
    })
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
    write_file_atomic(&path, content.as_bytes())
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

async fn delete_profile_dir(app: &AppHandle, profile_id: &str, path: &str) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("서버 폴더 경로가 비어 있어 파일 삭제를 건너뛰었습니다.".to_string());
    }

    let dir = PathBuf::from(trimmed);
    ensure_deletable_profile_dir(app, profile_id, &dir)?;

    match fs::symlink_metadata(&dir).await {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err("심볼릭 링크 서버 폴더는 삭제하지 않습니다.".to_string())
        }
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(&dir)
            .await
            .map_err(|error| format!("서버 폴더 삭제 실패: {error}")),
        Ok(_) => Err("서버 폴더 경로가 디렉터리가 아닙니다.".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("서버 폴더 확인 실패: {error}")),
    }
}

fn ensure_deletable_profile_dir(
    app: &AppHandle,
    profile_id: &str,
    dir: &Path,
) -> Result<(), String> {
    if !dir.is_absolute() {
        return Err("절대 경로가 아닌 서버 폴더는 삭제하지 않습니다.".to_string());
    }
    if dir.parent().is_none() {
        return Err("파일시스템 루트는 삭제할 수 없습니다.".to_string());
    }

    let default_dir = default_profile_dir(app, profile_id)?;
    if !same_path(dir, &default_dir) {
        return Err(
            "앱이 생성한 기본 서버 폴더만 자동 삭제할 수 있습니다. 사용자 지정 폴더는 직접 확인 후 삭제해 주세요."
                .to_string(),
        );
    }

    let app_data = app_data_dir(app)?;
    let servers_root = app_data.join("servers");
    let protected = [
        app.path()
            .home_dir()
            .map_err(|error| format!("홈 폴더 확인 실패: {error}"))?,
        app_data,
        servers_root,
    ];

    if protected.iter().any(|path| same_path(dir, path)) {
        return Err("보호된 폴더는 삭제할 수 없습니다.".to_string());
    }

    Ok(())
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    left == right
}

fn next_profile_id(settings: &AppSettings) -> String {
    loop {
        let id = unique_id("profile");
        if !settings.profiles.iter().any(|profile| profile.id == id) {
            return id;
        }
    }
}

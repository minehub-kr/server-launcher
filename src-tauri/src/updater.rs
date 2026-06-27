use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::{Update, UpdaterExt};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgressEvent {
    pub chunk_length: usize,
    pub content_length: Option<u64>,
}

fn current_version(app: &AppHandle) -> String {
    if cfg!(debug_assertions) {
        return "dev".to_string();
    }

    app.package_info().version.to_string()
}

fn to_info(app: &AppHandle, update: Option<Update>) -> UpdateInfo {
    UpdateInfo {
        available: update.is_some(),
        current_version: current_version(app),
        version: update.as_ref().map(|u| u.version.to_string()),
        notes: update.as_ref().and_then(|u| u.body.clone()),
        pub_date: update.as_ref().and_then(|u| u.date.map(|d| d.to_string())),
    }
}

#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<UpdateInfo, String> {
    let updater = app.updater().map_err(|error| error.to_string())?;
    let update = updater.check().await.map_err(|error| error.to_string())?;
    Ok(to_info(&app, update))
}

#[tauri::command]
pub async fn download_and_install_update(app: AppHandle) -> Result<bool, String> {
    let updater = app.updater().map_err(|error| error.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "사용 가능한 업데이트가 없습니다.".to_string())?;

    let app_for_progress = app.clone();
    update
        .download_and_install(
            move |chunk_length, content_length| {
                let _ = app_for_progress.emit(
                    "updater-progress",
                    UpdateProgressEvent {
                        chunk_length,
                        content_length,
                    },
                );
            },
            move || {
                let _ = app.emit("updater-installing", true);
            },
        )
        .await
        .map_err(|error| error.to_string())?;

    Ok(true)
}

#[tauri::command]
pub async fn current_app_version(app: AppHandle) -> Result<String, String> {
    Ok(current_version(&app))
}

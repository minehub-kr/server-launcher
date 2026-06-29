use crate::{
    models::BackupInfo,
    settings::find_profile,
    system::{sanitize_name, timestamp},
};
use std::{
    fs::File,
    io::copy,
    path::{Path, PathBuf},
};
use tauri::AppHandle;
use zip::{write::FileOptions, ZipWriter};

#[tauri::command]
pub async fn create_backup(app: AppHandle, profile_id: String) -> Result<BackupInfo, String> {
    let profile = find_profile(&app, &profile_id).await?;
    tauri::async_runtime::spawn_blocking(move || {
        let server_dir = std::path::PathBuf::from(&profile.server_dir);
        let server_root = server_dir
            .canonicalize()
            .map_err(|error| format!("서버 폴더 확인 실패: {error}"))?;
        let backup_dir = server_dir.join("backups");
        std::fs::create_dir_all(&backup_dir)
            .map_err(|error| format!("백업 폴더 생성 실패: {error}"))?;
        let filename = format!(
            "{}-{}-{}.zip",
            sanitize_name(&profile.name),
            profile.minecraft_version,
            timestamp()
        );
        let path = backup_dir.join(&filename);
        let file = File::create(&path).map_err(|error| format!("백업 파일 생성 실패: {error}"))?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for item in [
            "world",
            "world_nether",
            "world_the_end",
            "plugins",
            "server.properties",
            "bukkit.yml",
            "spigot.yml",
            "paper.yml",
            "purpur.yml",
            "ops.json",
            "whitelist.json",
            "banned-players.json",
        ] {
            let item_path = server_dir.join(item);
            if item_path.exists() {
                add_zip_entry(&mut zip, &server_dir, &server_root, &item_path, options)?;
            }
        }

        zip.finish()
            .map_err(|error| format!("백업 마무리 실패: {error}"))?;
        let size = std::fs::metadata(&path)
            .map_err(|error| format!("백업 메타데이터 확인 실패: {error}"))?
            .len();

        Ok(BackupInfo {
            filename,
            path: path.to_string_lossy().to_string(),
            size,
        })
    })
    .await
    .map_err(|error| format!("백업 생성 실패: {error}"))?
}

fn add_zip_entry(
    zip: &mut ZipWriter<File>,
    base: &Path,
    base_canonical: &Path,
    path: &Path,
    options: FileOptions,
) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("백업 항목 메타데이터 확인 실패: {error}"))?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }

    if metadata.is_dir() {
        for entry in
            std::fs::read_dir(path).map_err(|error| format!("백업 폴더 읽기 실패: {error}"))?
        {
            let entry = entry.map_err(|error| format!("백업 항목 읽기 실패: {error}"))?;
            add_zip_entry(zip, base, base_canonical, &entry.path(), options)?;
        }
        return Ok(());
    }

    ensure_inside_base(base_canonical, path)?;
    let relative = path
        .strip_prefix(base)
        .map_err(|error| format!("백업 상대 경로 계산 실패: {error}"))?
        .to_string_lossy()
        .replace('\\', "/");
    zip.start_file(relative, options)
        .map_err(|error| format!("zip 항목 생성 실패: {error}"))?;
    let mut file = File::open(path).map_err(|error| format!("백업 파일 열기 실패: {error}"))?;
    copy(&mut file, zip)
        .map_err(|error| format!("zip 파일 쓰기 실패: {error}"))
        .map(|_| ())
}

fn ensure_inside_base(base: &Path, path: &Path) -> Result<(), String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("백업 항목 경로 확인 실패: {error}"))?;
    if canonical.starts_with(base) {
        Ok(())
    } else {
        Err(format!(
            "서버 폴더 밖의 파일은 백업하지 않습니다: {}",
            display_path(path)
        ))
    }
}

fn display_path(path: &Path) -> String {
    PathBuf::from(path).to_string_lossy().to_string()
}

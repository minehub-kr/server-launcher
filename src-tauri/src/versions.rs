use crate::{
    java::choose_java,
    models::{
        AppState, PaperBuild, PaperBuilds, PaperProject, PurpurBuilds, PurpurProject, ServerKind,
        ServerPlan, ServerProfile, ServerVersion, VersionDetail, VersionManifest,
    },
    settings::find_profile,
    system::{
        download_file, download_file_with_sha1, download_file_with_sha256, file_matches_sha1,
        file_matches_sha256, file_nonempty, get_json, get_json_or, stable_mc_version,
        MINECRAFT_MANIFEST_URL, PAPER_API,
    },
};
use reqwest::Client;
use std::{path::Path, path::PathBuf};
use tauri::{AppHandle, State};

const UNSUPPORTED_SERVER_VERSION: &str =
    "선택한 서버 구현체가 해당 Minecraft 버전을 지원하지 않습니다.";

#[tauri::command]
pub async fn list_server_versions(
    state: State<'_, AppState>,
    kind: ServerKind,
) -> Result<Vec<ServerVersion>, String> {
    let versions: Vec<String> = match kind {
        ServerKind::Vanilla => {
            let manifest: VersionManifest = get_json(&state.http, MINECRAFT_MANIFEST_URL).await?;
            manifest
                .versions
                .into_iter()
                .filter(|version| version.kind == "release" && stable_mc_version(&version.id))
                .map(|version| version.id)
                .collect()
        }
        ServerKind::Paper | ServerKind::Folia => {
            let project = kind
                .papermc_project()
                .ok_or_else(|| "PaperMC 프로젝트 매핑 실패".to_string())?;
            let url = format!("{PAPER_API}/projects/{project}");
            let project: PaperProject =
                get_json_or(&state.http, &url, UNSUPPORTED_SERVER_VERSION).await?;
            project
                .versions
                .into_iter()
                .rev()
                .filter(|version| stable_mc_version(version))
                .collect()
        }
        ServerKind::Purpur => {
            let project: PurpurProject = get_json_or(
                &state.http,
                "https://api.purpurmc.org/v2/purpur",
                UNSUPPORTED_SERVER_VERSION,
            )
            .await?;
            project
                .versions
                .into_iter()
                .rev()
                .filter(|version| stable_mc_version(version))
                .collect()
        }
    };

    Ok(versions
        .into_iter()
        .map(|id| ServerVersion {
            label: format!("{} {id}", kind.label()),
            id,
            kind: "release".to_string(),
        })
        .collect())
}

#[tauri::command]
pub async fn resolve_server_plan(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<ServerPlan, String> {
    let profile = find_profile(&app, &profile_id).await?;
    let detail = fetch_version_detail_by_id(&state.http, &profile.minecraft_version).await?;
    let java = choose_java(&profile, detail.java_version.major_version).await?;
    let (server_available, server_note) =
        check_server_available(&state.http, &profile.kind, &profile.minecraft_version).await;

    Ok(ServerPlan {
        profile_id,
        version: profile.minecraft_version,
        server_kind: profile.kind,
        required_java: detail.java_version.major_version,
        java_component: detail.java_version.component,
        java,
        server_available,
        server_note,
    })
}

pub async fn fetch_version_detail_by_id(
    client: &Client,
    id: &str,
) -> Result<VersionDetail, String> {
    let manifest: VersionManifest = get_json(client, MINECRAFT_MANIFEST_URL).await?;
    let version = manifest
        .versions
        .into_iter()
        .find(|version| version.id == id)
        .ok_or_else(|| format!("Minecraft {id} 메타데이터를 찾지 못했습니다."))?;
    get_json(client, &version.url).await
}

pub async fn prepare_server_jar(
    client: &Client,
    profile: &ServerProfile,
    detail: &VersionDetail,
) -> Result<PathBuf, String> {
    let dir = Path::new(&profile.server_dir);
    match profile.kind {
        ServerKind::Vanilla => {
            let download = detail
                .downloads
                .server
                .clone()
                .ok_or_else(|| "공식 서버 JAR가 없는 버전입니다.".to_string())?;
            let path = dir.join(format!(
                "minecraft-server-{}.jar",
                profile.minecraft_version
            ));

            if file_matches_sha1(&path, &download.sha1).await {
                return Ok(path);
            }

            download_file_with_sha1(client, &download.url, &path, &download.sha1).await?;
            Ok(path)
        }
        ServerKind::Paper | ServerKind::Folia => {
            let project = profile
                .kind
                .papermc_project()
                .ok_or_else(|| "PaperMC 프로젝트 매핑 실패".to_string())?;
            let build = latest_papermc_build(client, project, &profile.minecraft_version).await?;
            let file_name = build.downloads.application.name;
            let path = dir.join(&file_name);

            if let Some(sha256) = build.downloads.application.sha256.as_deref() {
                if file_matches_sha256(&path, sha256).await {
                    return Ok(path);
                }
            } else if file_nonempty(&path).await {
                return Ok(path);
            }

            let url = format!(
                "{PAPER_API}/projects/{project}/versions/{}/builds/{}/downloads/{}",
                profile.minecraft_version, build.build, file_name
            );
            if let Some(sha256) = build.downloads.application.sha256.as_deref() {
                download_file_with_sha256(client, &url, &path, sha256).await?;
            } else {
                download_file(client, &url, &path).await?;
            }
            Ok(path)
        }
        ServerKind::Purpur => {
            let build = latest_purpur_build(client, &profile.minecraft_version).await?;
            let file_name = format!("purpur-{}-{build}.jar", profile.minecraft_version);
            let path = dir.join(&file_name);

            if file_nonempty(&path).await {
                return Ok(path);
            }

            let url = format!(
                "https://api.purpurmc.org/v2/purpur/{}/{build}/download",
                profile.minecraft_version
            );
            download_file(client, &url, &path).await?;
            Ok(path)
        }
    }
}

async fn check_server_available(
    client: &Client,
    kind: &ServerKind,
    version: &str,
) -> (bool, String) {
    match kind {
        ServerKind::Vanilla => match fetch_version_detail_by_id(client, version).await {
            Ok(detail) if detail.downloads.server.is_some() => {
                (true, "Vanilla 서버 JAR 다운로드 가능".to_string())
            }
            Ok(_) => (
                false,
                "이 Minecraft 버전은 공식 서버 JAR가 없습니다.".to_string(),
            ),
            Err(error) => (false, error),
        },
        ServerKind::Purpur => match latest_purpur_build(client, version).await {
            Ok(build) => (true, format!("Purpur 빌드 {build} 다운로드 가능")),
            Err(error) => (false, error),
        },
        ServerKind::Paper | ServerKind::Folia => {
            let project = kind.papermc_project().unwrap_or("paper");
            match latest_papermc_build(client, project, version).await {
                Ok(build) => (
                    true,
                    format!("{} 빌드 {} 다운로드 가능", kind.label(), build.build),
                ),
                Err(error) => (false, error),
            }
        }
    }
}

async fn latest_papermc_build(
    client: &Client,
    project: &str,
    version: &str,
) -> Result<PaperBuild, String> {
    let url = format!("{PAPER_API}/projects/{project}/versions/{version}/builds");
    let builds: PaperBuilds = get_json_or(client, &url, UNSUPPORTED_SERVER_VERSION).await?;
    builds
        .builds
        .into_iter()
        .max_by_key(|build| build.build)
        .ok_or_else(|| format!("선택한 버전의 {project} 빌드가 없습니다."))
}

async fn latest_purpur_build(client: &Client, version: &str) -> Result<String, String> {
    let url = format!("https://api.purpurmc.org/v2/purpur/{version}");
    let builds: PurpurBuilds = get_json_or(client, &url, UNSUPPORTED_SERVER_VERSION).await?;
    Ok(builds.builds.latest)
}

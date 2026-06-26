use crate::{
    config::{read_config_bundle, write_default_properties},
    java::choose_java,
    models::{AppState, ServerLogEvent, ServerRuntime, ServerStatus},
    settings::{load_settings, save_settings},
    system::{app_data_dir, crash_line, port_available, timestamp},
    versions::{fetch_version_detail_by_id, prepare_server_jar},
};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};
use tauri::{AppHandle, Emitter, State};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::Command,
    sync::Mutex,
};

const EULA_URL: &str = "https://aka.ms/MinecraftEULA";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EulaStatus {
    pub accepted: bool,
    pub path: String,
    pub url: String,
}

#[tauri::command]
pub async fn eula_status(app: AppHandle, profile_id: String) -> Result<EulaStatus, String> {
    let profile = crate::settings::find_profile(&app, &profile_id).await?;
    let path = eula_path(&profile.server_dir);
    Ok(EulaStatus {
        accepted: eula_accepted(&path).await?,
        path: path.to_string_lossy().to_string(),
        url: EULA_URL.to_string(),
    })
}

#[tauri::command]
pub async fn accept_eula(app: AppHandle, profile_id: String) -> Result<EulaStatus, String> {
    let profile = crate::settings::find_profile(&app, &profile_id).await?;
    tokio::fs::create_dir_all(&profile.server_dir)
        .await
        .map_err(|error| format!("서버 폴더 생성 실패: {error}"))?;
    let path = eula_path(&profile.server_dir);
    tokio::fs::write(&path, "eula=true\n")
        .await
        .map_err(|error| format!("EULA 파일 작성 실패: {error}"))?;
    Ok(EulaStatus {
        accepted: true,
        path: path.to_string_lossy().to_string(),
        url: EULA_URL.to_string(),
    })
}

#[tauri::command]
pub async fn start_server(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<ServerStatus, String> {
    {
        let runtime = state.runtime.lock().await;
        if runtime.current_profile_id.is_some() {
            return Err("이미 실행 중인 서버가 있습니다.".to_string());
        }
    }

    let mut settings = load_settings(&app).await?;
    let index = settings
        .profiles
        .iter()
        .position(|profile| profile.id == profile_id)
        .ok_or_else(|| "프로필을 찾지 못했습니다.".to_string())?;
    let mut profile = settings.profiles[index].clone();
    if !eula_accepted(&eula_path(&profile.server_dir)).await? {
        return Err("Minecraft EULA 동의가 필요합니다.".to_string());
    }

    let detail = fetch_version_detail_by_id(&state.http, &profile.minecraft_version).await?;
    let java = choose_java(&profile, detail.java_version.major_version)
        .await?
        .ok_or_else(|| {
            format!(
                "Java {}+ 런타임을 찾지 못했습니다. Java를 설치하거나 프로필에 java 경로를 지정해 주세요.",
                detail.java_version.major_version
            )
        })?;

    let config = read_config_bundle(&profile).await?;
    let port = config.properties.server_port;
    if !port_available(port) {
        return Err(format!(
            "포트 {port}이 이미 사용 중입니다. server.properties에서 포트를 바꿔 주세요."
        ));
    }

    tokio::fs::create_dir_all(&profile.server_dir)
        .await
        .map_err(|error| format!("서버 폴더 생성 실패: {error}"))?;
    write_default_properties(&profile, &config.properties).await?;

    let jar = prepare_server_jar(&state.http, &profile, &detail).await?;
    let xms = profile.memory_mb.clamp(512, 1024);
    let xmx = profile.memory_mb.max(512);
    let mut child = Command::new(&java.path)
        .arg(format!("-Xms{xms}M"))
        .arg(format!("-Xmx{xmx}M"))
        .arg("-jar")
        .arg(jar)
        .arg("nogui")
        .current_dir(&profile.server_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("서버 실행 실패: {error}"))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "서버 stdin 연결 실패".to_string())?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    profile.last_used = Some(timestamp().to_string());
    settings.selected_profile_id = Some(profile.id.clone());
    settings.profiles[index] = profile.clone();
    save_settings(&app, &settings).await?;

    {
        let mut runtime = state.runtime.lock().await;
        runtime.status = "running".to_string();
        runtime.players.clear();
        runtime.logs.clear();
        runtime.current_profile_id = Some(profile.id.clone());
        runtime.java = Some(java.clone());
        runtime.stdin = Some(Arc::new(Mutex::new(stdin)));
        runtime.crash_detected = false;
        runtime.exit_message = None;
        runtime.stop_requested = false;
        append_and_emit_log(
            &app,
            &mut runtime,
            format!(
                "Starting {} {} on port {} with Java {}",
                profile.kind.as_str(),
                profile.minecraft_version,
                port,
                java.major
            ),
        );
        emit_status(&app, &runtime);
    }

    if let Some(stdout) = stdout {
        tauri::async_runtime::spawn(read_server_output(
            app.clone(),
            stdout,
            state.runtime.clone(),
        ));
    }
    if let Some(stderr) = stderr {
        tauri::async_runtime::spawn(read_server_output(
            app.clone(),
            stderr,
            state.runtime.clone(),
        ));
    }

    let runtime = state.runtime.clone();
    let app_for_exit = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = child.wait().await;
        let mut runtime = runtime.lock().await;
        runtime.stdin = None;
        runtime.players.clear();
        runtime.current_profile_id = None;
        let crashed = process_crashed(
            runtime.stop_requested,
            result.as_ref().map(|status| status.success()).ok(),
        );
        let message = match result {
            Ok(status) => format!("Server process exited with {status}"),
            Err(error) => format!("Server process wait failed: {error}"),
        };
        runtime.status = if crashed { "crashed" } else { "stopped" }.to_string();
        runtime.crash_detected = crashed;
        runtime.exit_message = Some(message.clone());
        runtime.stop_requested = false;
        append_and_emit_log(&app_for_exit, &mut runtime, message);
        emit_status(&app_for_exit, &runtime);
    });

    status_snapshot(&app, &state).await
}

#[tauri::command]
pub async fn stop_server(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ServerStatus, String> {
    {
        let mut runtime = state.runtime.lock().await;
        runtime.status = "stopping".to_string();
        runtime.stop_requested = true;
        emit_status(&app, &runtime);
    }
    write_server_command(&app, &state, "stop").await?;
    status_snapshot(&app, &state).await
}

#[tauri::command]
pub async fn send_server_command(
    app: AppHandle,
    state: State<'_, AppState>,
    command: String,
) -> Result<ServerStatus, String> {
    write_server_command(&app, &state, &command).await?;
    status_snapshot(&app, &state).await
}

#[tauri::command]
pub async fn server_status(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ServerStatus, String> {
    status_snapshot(&app, &state).await
}

async fn write_server_command(
    app: &AppHandle,
    state: &State<'_, AppState>,
    command: &str,
) -> Result<(), String> {
    write_server_commands(app, state, vec![command.to_string()], true).await
}

async fn write_server_commands(
    app: &AppHandle,
    state: &State<'_, AppState>,
    commands: Vec<String>,
    echo: bool,
) -> Result<(), String> {
    let stdin = {
        let runtime = state.runtime.lock().await;
        if runtime.current_profile_id.is_none() {
            return Err("실행 중인 서버가 없습니다.".to_string());
        }
        runtime
            .stdin
            .clone()
            .ok_or_else(|| "서버 stdin이 준비되지 않았습니다.".to_string())?
    };

    let mut stdin = stdin.lock().await;
    for command in &commands {
        stdin
            .write_all(format!("{command}\n").as_bytes())
            .await
            .map_err(|error| format!("서버 명령 전송 실패: {error}"))?;
    }
    stdin
        .flush()
        .await
        .map_err(|error| format!("서버 명령 flush 실패: {error}"))?;

    let mut runtime = state.runtime.lock().await;
    if echo {
        for command in commands {
            append_and_emit_log(app, &mut runtime, format!("> {command}"));
        }
    }
    emit_status(app, &runtime);
    Ok(())
}

pub async fn write_runtime_command(
    app: &AppHandle,
    state: &State<'_, AppState>,
    command: &str,
) -> Result<(), String> {
    write_server_command(app, state, command).await
}

pub async fn running_profile_id(state: &State<'_, AppState>) -> Option<String> {
    state.runtime.lock().await.current_profile_id.clone()
}

async fn read_server_output<R>(app: AppHandle, reader: R, runtime: Arc<Mutex<ServerRuntime>>)
where
    R: AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let mut runtime = runtime.lock().await;
        parse_player_line(&mut runtime, &line);
        if crash_line(&line) {
            runtime.crash_detected = true;
        }
        append_and_emit_log(&app, &mut runtime, line);
        emit_status(&app, &runtime);
    }
}

fn parse_player_line(runtime: &mut ServerRuntime, line: &str) {
    let message = line
        .rsplit_once("]: ")
        .map(|(_, message)| message)
        .unwrap_or(line)
        .trim();

    if let Some(player) = message.strip_suffix(" joined the game") {
        insert_player(runtime, player);
        return;
    }

    if let Some((player, rest)) = message.split_once("[/") {
        if rest.contains("] logged in with entity id ") {
            insert_player(runtime, player);
            return;
        }
    }

    if let Some(player) = message.strip_suffix(" left the game") {
        remove_player(runtime, player);
        return;
    }

    if let Some((player, _)) = message.split_once(" lost connection:") {
        remove_player(runtime, player);
        return;
    }

    if message.contains("There are 0 of a max") {
        runtime.players.clear();
        return;
    }

    if let Some((_, players)) = message.split_once("players online:") {
        runtime.players = players
            .split(',')
            .map(str::trim)
            .filter(|name| valid_player_name(name))
            .map(ToString::to_string)
            .collect();
    }
}

fn insert_player(runtime: &mut ServerRuntime, player: &str) {
    let player = player.trim();
    if valid_player_name(player) {
        runtime.players.insert(player.to_string());
    }
}

fn remove_player(runtime: &mut ServerRuntime, player: &str) {
    let player = player.trim();
    if valid_player_name(player) {
        runtime.players.remove(player);
    }
}

fn valid_player_name(name: &str) -> bool {
    (1..=16).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn process_crashed(stop_requested: bool, exit_success: Option<bool>) -> bool {
    !stop_requested && !exit_success.unwrap_or(false)
}

fn eula_path(server_dir: &str) -> PathBuf {
    Path::new(server_dir).join("eula.txt")
}

async fn eula_accepted(path: &Path) -> Result<bool, String> {
    let Ok(content) = tokio::fs::read_to_string(path).await else {
        return Ok(false);
    };
    Ok(eula_content_accepted(&content))
}

fn eula_content_accepted(content: &str) -> bool {
    content.lines().any(|line| {
        line.split_once('=').is_some_and(|(key, value)| {
            key.trim().eq_ignore_ascii_case("eula") && value.trim().eq_ignore_ascii_case("true")
        })
    })
}

fn append_and_emit_log(app: &AppHandle, runtime: &mut ServerRuntime, line: String) {
    let event = ServerLogEvent { line: line.clone() };
    runtime.logs.push(line);

    if runtime.logs.len() > 1000 {
        let drop_count = runtime.logs.len() - 1000;
        runtime.logs.drain(0..drop_count);
    }

    let _ = app.emit("server-log", event);
}

async fn status_snapshot(
    app: &AppHandle,
    state: &State<'_, AppState>,
) -> Result<ServerStatus, String> {
    let runtime = state.runtime.lock().await;

    status_from_runtime(app, &runtime)
}

fn status_from_runtime(app: &AppHandle, runtime: &ServerRuntime) -> Result<ServerStatus, String> {
    let data_dir = app_data_dir(app)?;
    Ok(ServerStatus {
        running: runtime.current_profile_id.is_some(),
        status: runtime.status.clone(),
        players: runtime.players.iter().cloned().collect(),
        logs: runtime.logs.clone(),
        current_profile_id: runtime.current_profile_id.clone(),
        java: runtime.java.clone(),
        data_dir: data_dir.to_string_lossy().to_string(),
        crash_detected: runtime.crash_detected,
        exit_message: runtime.exit_message.clone(),
    })
}

fn emit_status(app: &AppHandle, runtime: &ServerRuntime) {
    if let Ok(status) = status_from_runtime(app, runtime) {
        let _ = app.emit("server-status", status);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime_with_players(players: &[&str]) -> ServerRuntime {
        let mut runtime = ServerRuntime::default();
        for player in players {
            runtime.players.insert((*player).to_string());
        }
        runtime
    }

    #[test]
    fn tracks_join_and_leave_messages() {
        let mut runtime = ServerRuntime::default();

        parse_player_line(&mut runtime, "[12:00:00 INFO]: Steve joined the game");
        assert!(runtime.players.contains("Steve"));

        parse_player_line(&mut runtime, "[12:01:00 INFO]: Steve left the game");
        assert!(!runtime.players.contains("Steve"));
    }

    #[test]
    fn tracks_paper_login_and_disconnect_messages() {
        let mut runtime = ServerRuntime::default();

        parse_player_line(
            &mut runtime,
            "[12:00:00 INFO]: Alex[/127.0.0.1:53211] logged in with entity id 42 at ([world]0.0, 64.0, 0.0)",
        );
        assert!(runtime.players.contains("Alex"));

        parse_player_line(
            &mut runtime,
            "[12:02:00 INFO]: Alex lost connection: Disconnected",
        );
        assert!(!runtime.players.contains("Alex"));
    }

    #[test]
    fn parses_list_output() {
        let mut runtime = ServerRuntime::default();

        parse_player_line(
            &mut runtime,
            "[12:00:00 INFO]: There are 2 of a max of 20 players online: Steve, Alex",
        );

        assert_eq!(
            runtime.players.iter().cloned().collect::<Vec<_>>(),
            vec!["Alex".to_string(), "Steve".to_string()]
        );
    }

    #[test]
    fn clears_empty_list_output() {
        let mut runtime = runtime_with_players(&["Steve"]);

        parse_player_line(
            &mut runtime,
            "[12:00:00 INFO]: There are 0 of a max of 20 players online:",
        );

        assert!(runtime.players.is_empty());
    }

    #[test]
    fn ignores_uuid_and_invalid_names() {
        let mut runtime = ServerRuntime::default();

        parse_player_line(
            &mut runtime,
            "[12:00:00 INFO]: UUID of player Steve is 00000000-0000-0000-0000-000000000000",
        );
        parse_player_line(
            &mut runtime,
            "[12:00:01 INFO]: invalid-name joined the game",
        );

        assert!(runtime.players.is_empty());
    }

    #[test]
    fn intentional_stop_is_not_crash_even_with_nonzero_exit() {
        assert!(!process_crashed(true, Some(false)));
    }

    #[test]
    fn unexpected_nonzero_exit_is_crash() {
        assert!(process_crashed(false, Some(false)));
    }

    #[test]
    fn eula_true_is_accepted() {
        assert!(eula_content_accepted("eula=true\n"));
        assert!(eula_content_accepted("# comment\neula = TRUE\n"));
    }

    #[test]
    fn eula_false_or_missing_is_not_accepted() {
        assert!(!eula_content_accepted(""));
        assert!(!eula_content_accepted("eula=false\n"));
        assert!(!eula_content_accepted("other=true\n"));
    }
}

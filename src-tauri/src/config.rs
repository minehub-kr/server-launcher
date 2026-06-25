use crate::{
    models::{
        AccessLists, AppState, BanEntry, ConfigFieldKind, ConfigFieldValue, ConfigFile,
        ConfigFormField, JsonListFile, MinecraftIdentity, MojangProfile, OpEntry,
        ServerConfigBundle, ServerKind, ServerProfile, ServerProperties, WhitelistEntry,
    },
    runtime::{running_profile_id, write_runtime_command},
    settings::{find_profile, load_settings, save_settings},
    system::{get_json_or, hyphenate_uuid, safe_relative_path, timestamp},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
use tauri::{AppHandle, State};
use tokio::fs;

#[tauri::command]
pub async fn read_server_config(
    app: AppHandle,
    profile_id: String,
) -> Result<ServerConfigBundle, String> {
    let profile = find_profile(&app, &profile_id).await?;
    read_config_bundle(&profile).await
}

#[tauri::command]
pub async fn save_server_config(
    app: AppHandle,
    profile_id: String,
    bundle: ServerConfigBundle,
) -> Result<ServerConfigBundle, String> {
    let mut settings = load_settings(&app).await?;
    let index = settings
        .profiles
        .iter()
        .position(|profile| profile.id == profile_id)
        .ok_or_else(|| "프로필을 찾지 못했습니다.".to_string())?;
    let mut profile = settings.profiles[index].clone();
    let dir = PathBuf::from(&profile.server_dir);

    fs::create_dir_all(&dir)
        .await
        .map_err(|error| format!("서버 폴더 생성 실패: {error}"))?;

    let properties_path = dir.join("server.properties");
    backup_existing(&dir, &properties_path).await?;
    let property_raw = merge_properties(&bundle.properties_raw, &bundle.properties);
    fs::write(&properties_path, property_raw)
        .await
        .map_err(|error| format!("server.properties 저장 실패: {error}"))?;

    for file in &bundle.config_files {
        if !safe_relative_path(&file.relative_path) {
            return Err("잘못된 설정 파일 경로입니다.".to_string());
        }
        if !file.exists {
            continue;
        }
        let path = dir.join(&file.relative_path);
        backup_existing(&dir, &path).await?;
        let content =
            apply_config_fields(&file.content, &file.relative_path, &bundle.config_fields)?;
        fs::write(path, content)
            .await
            .map_err(|error| format!("{} 저장 실패: {error}", file.name))?;
    }

    for file in &bundle.json_lists {
        serde_json::from_str::<serde_json::Value>(&file.content)
            .map_err(|error| format!("{} JSON 형식 오류: {error}", file.name))?;
        let path = dir.join(&file.name);
        backup_existing(&dir, &path).await?;
        fs::write(path, &file.content)
            .await
            .map_err(|error| format!("{} 저장 실패: {error}", file.name))?;
    }

    profile.settings = bundle.properties;
    settings.profiles[index] = profile.clone();
    save_settings(&app, &settings).await?;
    read_config_bundle(&profile).await
}

#[tauri::command]
pub async fn lookup_minecraft_profile(
    state: State<'_, AppState>,
    name: String,
) -> Result<MinecraftIdentity, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Minecraft 닉네임을 입력해 주세요.".to_string());
    }
    if !name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err("Minecraft 닉네임은 영문, 숫자, 밑줄만 사용할 수 있습니다.".to_string());
    }

    let url = format!("https://api.mojang.com/users/profiles/minecraft/{name}");
    let profile: MojangProfile =
        get_json_or(&state.http, &url, "Minecraft 닉네임을 찾지 못했습니다.").await?;
    Ok(MinecraftIdentity {
        name: profile.name,
        uuid: hyphenate_uuid(&profile.id),
    })
}

#[tauri::command]
pub async fn read_access_lists(app: AppHandle, profile_id: String) -> Result<AccessLists, String> {
    let profile = find_profile(&app, &profile_id).await?;
    read_access_lists_from_profile(&profile).await
}

#[tauri::command]
pub async fn save_access_lists(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
    lists: AccessLists,
) -> Result<AccessLists, String> {
    let profile = find_profile(&app, &profile_id).await?;
    let dir = Path::new(&profile.server_dir);
    fs::create_dir_all(dir)
        .await
        .map_err(|error| format!("서버 폴더 생성 실패: {error}"))?;

    let ops = parse_access_file::<OpEntry>("ops.json", &lists.raw_ops)?;
    let whitelist = parse_access_file::<WhitelistEntry>("whitelist.json", &lists.raw_whitelist)?;
    let banned = parse_access_file::<BanEntry>("banned-players.json", &lists.raw_banned_players)?;
    let before = read_access_lists_from_profile(&profile).await?;
    let after = AccessLists {
        ops: ops.clone(),
        whitelist: whitelist.clone(),
        banned_players: banned.clone(),
        raw_ops: lists.raw_ops.clone(),
        raw_whitelist: lists.raw_whitelist.clone(),
        raw_banned_players: lists.raw_banned_players.clone(),
    };
    let commands = if running_profile_id(&state).await.as_deref() == Some(profile_id.as_str()) {
        access_delta_commands(&before, &after)?
    } else {
        Vec::new()
    };

    write_access_file(dir, "ops.json", &ops).await?;
    write_access_file(dir, "whitelist.json", &whitelist).await?;
    write_access_file(dir, "banned-players.json", &banned).await?;

    for command in commands {
        write_runtime_command(&app, &state, &command).await?;
    }

    read_access_lists_from_profile(&profile).await
}

pub async fn read_config_bundle(profile: &ServerProfile) -> Result<ServerConfigBundle, String> {
    let dir = Path::new(&profile.server_dir);
    let properties_path = dir.join("server.properties");
    let properties_raw = fs::read_to_string(&properties_path)
        .await
        .unwrap_or_default();
    let properties = read_properties_from_raw(&properties_raw, &profile.settings);

    let mut config_files = Vec::new();
    let mut config_fields = Vec::new();
    for relative_path in config_file_paths(&profile.kind) {
        let path = dir.join(relative_path);
        let exists = path.exists();
        let content = if exists {
            fs::read_to_string(&path).await.unwrap_or_default()
        } else {
            String::new()
        };
        if exists {
            config_fields.extend(read_yaml_fields(relative_path, &content));
        }
        config_files.push(ConfigFile {
            name: Path::new(relative_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(relative_path)
                .to_string(),
            relative_path: relative_path.to_string(),
            exists,
            editable: exists,
            content,
        });
    }

    let mut json_lists = Vec::new();
    for name in ["ops.json", "whitelist.json", "banned-players.json"] {
        let path = dir.join(name);
        let exists = path.exists();
        let content = if exists {
            fs::read_to_string(path)
                .await
                .unwrap_or_else(|_| "[]\n".to_string())
        } else {
            "[]\n".to_string()
        };
        json_lists.push(JsonListFile {
            name: name.to_string(),
            exists,
            content,
        });
    }

    Ok(ServerConfigBundle {
        properties,
        properties_raw,
        config_files,
        config_fields,
        json_lists,
        restart_required: false,
    })
}

pub async fn write_default_properties(
    profile: &ServerProfile,
    properties: &ServerProperties,
) -> Result<(), String> {
    let path = Path::new(&profile.server_dir).join("server.properties");
    let raw = fs::read_to_string(&path).await.unwrap_or_default();
    let merged = merge_properties(&raw, properties);
    fs::write(path, merged)
        .await
        .map_err(|error| format!("server.properties 저장 실패: {error}"))
}

async fn read_access_lists_from_profile(profile: &ServerProfile) -> Result<AccessLists, String> {
    let dir = Path::new(&profile.server_dir);
    let raw_ops = read_access_raw(dir, "ops.json").await;
    let raw_whitelist = read_access_raw(dir, "whitelist.json").await;
    let raw_banned_players = read_access_raw(dir, "banned-players.json").await;

    Ok(AccessLists {
        ops: serde_json::from_str(&raw_ops).unwrap_or_default(),
        whitelist: serde_json::from_str(&raw_whitelist).unwrap_or_default(),
        banned_players: serde_json::from_str(&raw_banned_players).unwrap_or_default(),
        raw_ops,
        raw_whitelist,
        raw_banned_players,
    })
}

async fn read_access_raw(dir: &Path, name: &str) -> String {
    fs::read_to_string(dir.join(name))
        .await
        .unwrap_or_else(|_| "[]\n".to_string())
}

fn parse_access_file<T: for<'de> serde::Deserialize<'de>>(
    name: &str,
    raw: &str,
) -> Result<Vec<T>, String> {
    serde_json::from_str(raw).map_err(|error| format!("{name} JSON 형식 오류: {error}"))
}

async fn write_access_file<T: serde::Serialize>(
    dir: &Path,
    name: &str,
    entries: &[T],
) -> Result<(), String> {
    let path = dir.join(name);
    backup_existing(dir, &path).await?;
    let content = serde_json::to_string_pretty(entries)
        .map_err(|error| format!("{name} JSON 직렬화 실패: {error}"))?;
    fs::write(path, format!("{content}\n"))
        .await
        .map_err(|error| format!("{name} 저장 실패: {error}"))
}

fn read_properties_from_raw(raw: &str, defaults: &ServerProperties) -> ServerProperties {
    let map = parse_properties(raw);
    ServerProperties {
        server_port: prop_parse(&map, "server-port", defaults.server_port),
        online_mode: prop_parse(&map, "online-mode", defaults.online_mode),
        motd: prop_string(&map, "motd", &defaults.motd),
        max_players: prop_parse(&map, "max-players", defaults.max_players),
        difficulty: prop_string(&map, "difficulty", &defaults.difficulty),
        gamemode: prop_string(&map, "gamemode", &defaults.gamemode),
        pvp: prop_parse(&map, "pvp", defaults.pvp),
        view_distance: prop_parse(&map, "view-distance", defaults.view_distance),
        simulation_distance: prop_parse(&map, "simulation-distance", defaults.simulation_distance),
        enable_command_block: prop_parse(
            &map,
            "enable-command-block",
            defaults.enable_command_block,
        ),
        white_list: prop_parse(&map, "white-list", defaults.white_list),
    }
}

fn parse_properties(raw: &str) -> BTreeMap<String, String> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

fn prop_parse<T: std::str::FromStr>(map: &BTreeMap<String, String>, key: &str, fallback: T) -> T {
    map.get(key)
        .and_then(|value| value.parse::<T>().ok())
        .unwrap_or(fallback)
}

fn prop_string(map: &BTreeMap<String, String>, key: &str, fallback: &str) -> String {
    map.get(key)
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

fn merge_properties(raw: &str, props: &ServerProperties) -> String {
    let values = property_values(props);
    let mut seen = BTreeSet::new();
    let mut lines = Vec::new();

    for line in raw.lines() {
        if let Some((key, _)) = line.split_once('=') {
            let key = key.trim();
            if let Some(value) = values.get(key) {
                lines.push(format!("{key}={value}"));
                seen.insert(key.to_string());
                continue;
            }
        }
        lines.push(line.to_string());
    }

    if lines.is_empty() {
        lines.push("# Minecraft server properties".to_string());
    }

    for (key, value) in values {
        if !seen.contains(key) {
            lines.push(format!("{key}={value}"));
        }
    }

    format!("{}\n", lines.join("\n"))
}

fn property_values(props: &ServerProperties) -> BTreeMap<&'static str, String> {
    BTreeMap::from([
        ("server-port", props.server_port.to_string()),
        ("online-mode", props.online_mode.to_string()),
        ("motd", props.motd.clone()),
        ("max-players", props.max_players.to_string()),
        ("difficulty", props.difficulty.clone()),
        ("gamemode", props.gamemode.clone()),
        ("pvp", props.pvp.to_string()),
        ("view-distance", props.view_distance.to_string()),
        ("simulation-distance", props.simulation_distance.to_string()),
        (
            "enable-command-block",
            props.enable_command_block.to_string(),
        ),
        ("white-list", props.white_list.to_string()),
    ])
}

struct ConfigFieldDescriptor {
    file: &'static str,
    path: &'static str,
    label: &'static str,
    kind: ConfigFieldKind,
    options: Option<&'static [&'static str]>,
    restart_required: bool,
}

fn yaml_field_descriptors() -> Vec<ConfigFieldDescriptor> {
    use ConfigFieldKind::{Boolean, Number, Text};

    vec![
        ConfigFieldDescriptor {
            file: "bukkit.yml",
            path: "settings.allow-end",
            label: "엔드 허용",
            kind: Boolean,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "bukkit.yml",
            path: "settings.warn-on-overload",
            label: "과부하 경고",
            kind: Boolean,
            options: None,
            restart_required: false,
        },
        ConfigFieldDescriptor {
            file: "bukkit.yml",
            path: "settings.shutdown-message",
            label: "종료 메시지",
            kind: Text,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "bukkit.yml",
            path: "spawn-limits.monsters",
            label: "몬스터 스폰 제한",
            kind: Number,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "bukkit.yml",
            path: "spawn-limits.animals",
            label: "동물 스폰 제한",
            kind: Number,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "spigot.yml",
            path: "settings.restart-on-crash",
            label: "Crash 시 재시작",
            kind: Boolean,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "spigot.yml",
            path: "settings.timeout-time",
            label: "Timeout 시간",
            kind: Number,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "spigot.yml",
            path: "messages.whitelist",
            label: "Whitelist 메시지",
            kind: Text,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "spigot.yml",
            path: "messages.server-full",
            label: "서버 가득 참 메시지",
            kind: Text,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "spigot.yml",
            path: "world-settings.default.view-distance",
            label: "기본 View distance",
            kind: Number,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "spigot.yml",
            path: "world-settings.default.simulation-distance",
            label: "기본 Simulation distance",
            kind: Number,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "paper.yml",
            path: "settings.velocity-support.enabled",
            label: "Velocity 지원",
            kind: Boolean,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "paper.yml",
            path: "settings.velocity-support.online-mode",
            label: "Velocity online-mode",
            kind: Boolean,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "paper.yml",
            path: "settings.unsupported-settings.allow-headless-pistons",
            label: "Headless piston 허용",
            kind: Boolean,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "config/paper-global.yml",
            path: "proxies.velocity.enabled",
            label: "Velocity 프록시",
            kind: Boolean,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "config/paper-global.yml",
            path: "proxies.velocity.online-mode",
            label: "Velocity online-mode",
            kind: Boolean,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "config/paper-global.yml",
            path: "unsupported-settings.allow-headless-pistons",
            label: "Headless piston 허용",
            kind: Boolean,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "config/paper-global.yml",
            path: "unsupported-settings.allow-piston-duplication",
            label: "Piston duplication 허용",
            kind: Boolean,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "config/paper-world-defaults.yml",
            path: "chunks.auto-save-interval",
            label: "청크 자동 저장 간격",
            kind: Number,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "config/paper-world-defaults.yml",
            path: "collisions.max-entity-collisions",
            label: "최대 엔티티 충돌",
            kind: Number,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "purpur.yml",
            path: "settings.use-alternate-keepalive",
            label: "대체 Keepalive",
            kind: Boolean,
            options: None,
            restart_required: true,
        },
        ConfigFieldDescriptor {
            file: "purpur.yml",
            path: "world-settings.default.gameplay-mechanics.player.sleeping-percent",
            label: "수면 필요 비율",
            kind: Number,
            options: None,
            restart_required: true,
        },
    ]
}

fn read_yaml_fields(relative_path: &str, raw: &str) -> Vec<ConfigFormField> {
    let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(raw) else {
        return Vec::new();
    };

    yaml_field_descriptors()
        .into_iter()
        .filter(|descriptor| descriptor.file == relative_path)
        .filter_map(|descriptor| {
            let value = yaml_value_at(&value, descriptor.path)?;
            Some(ConfigFormField {
                file: descriptor.file.to_string(),
                path: descriptor.path.to_string(),
                label: descriptor.label.to_string(),
                value: config_field_value(value, &descriptor.kind)?,
                kind: descriptor.kind,
                options: descriptor
                    .options
                    .map(|options| options.iter().map(|option| (*option).to_string()).collect()),
                restart_required: descriptor.restart_required,
            })
        })
        .collect()
}

fn config_field_value(
    value: &serde_yaml::Value,
    kind: &ConfigFieldKind,
) -> Option<ConfigFieldValue> {
    match kind {
        ConfigFieldKind::Boolean => value.as_bool().map(ConfigFieldValue::Boolean),
        ConfigFieldKind::Number => value.as_f64().map(ConfigFieldValue::Number),
        ConfigFieldKind::Text | ConfigFieldKind::Select => value
            .as_str()
            .map(|value| ConfigFieldValue::Text(value.to_string())),
    }
}

fn apply_config_fields(
    raw: &str,
    relative_path: &str,
    fields: &[ConfigFormField],
) -> Result<String, String> {
    let fields: Vec<_> = fields
        .iter()
        .filter(|field| field.file == relative_path)
        .collect();
    if fields.is_empty() {
        return Ok(raw.to_string());
    }

    let mut yaml = serde_yaml::from_str::<serde_yaml::Value>(raw)
        .map_err(|error| format!("{relative_path} YAML 형식 오류: {error}"))?;
    for field in fields {
        let Some(slot) = yaml_value_at_mut(&mut yaml, &field.path) else {
            continue;
        };
        *slot = serde_yaml::to_value(&field.value)
            .map_err(|error| format!("{} 값 변환 실패: {error}", field.label))?;
    }

    serde_yaml::to_string(&yaml).map_err(|error| format!("{relative_path} YAML 저장 실패: {error}"))
}

fn yaml_value_at<'a>(value: &'a serde_yaml::Value, path: &str) -> Option<&'a serde_yaml::Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current
            .as_mapping()?
            .get(&serde_yaml::Value::String(segment.to_string()))?;
    }
    Some(current)
}

fn yaml_value_at_mut<'a>(
    value: &'a mut serde_yaml::Value,
    path: &str,
) -> Option<&'a mut serde_yaml::Value> {
    let mut current = value;
    let mut segments = path.split('.').peekable();
    while let Some(segment) = segments.next() {
        let key = serde_yaml::Value::String(segment.to_string());
        if segments.peek().is_none() {
            return current.as_mapping_mut()?.get_mut(&key);
        }
        current = current.as_mapping_mut()?.get_mut(&key)?;
    }
    None
}

fn access_delta_commands(before: &AccessLists, after: &AccessLists) -> Result<Vec<String>, String> {
    let before_ops = op_names(&before.ops);
    let after_ops = op_names(&after.ops);
    let before_whitelist = whitelist_names(&before.whitelist);
    let after_whitelist = whitelist_names(&after.whitelist);
    let before_banned = ban_entries(&before.banned_players);
    let after_banned = ban_entries(&after.banned_players);
    let mut commands = Vec::new();

    for (uuid, name) in &after_ops {
        if !before_ops.contains_key(uuid) {
            commands.push(format!("op {}", command_name(name)?));
        }
    }
    for (uuid, name) in &before_ops {
        if !after_ops.contains_key(uuid) {
            commands.push(format!("deop {}", command_name(name)?));
        }
    }

    let mut whitelist_changed = false;
    for (uuid, name) in &after_whitelist {
        if !before_whitelist.contains_key(uuid) {
            commands.push(format!("whitelist add {}", command_name(name)?));
            whitelist_changed = true;
        }
    }
    for (uuid, name) in &before_whitelist {
        if !after_whitelist.contains_key(uuid) {
            commands.push(format!("whitelist remove {}", command_name(name)?));
            whitelist_changed = true;
        }
    }
    if whitelist_changed {
        commands.push("whitelist reload".to_string());
    }

    for (uuid, (name, reason)) in &after_banned {
        if !before_banned.contains_key(uuid) {
            commands.push(format!(
                "ban {} {}",
                command_name(name)?,
                command_reason(reason)
            ));
        }
    }
    for (uuid, (name, _)) in &before_banned {
        if !after_banned.contains_key(uuid) {
            commands.push(format!("pardon {}", command_name(name)?));
        }
    }

    Ok(commands)
}

fn op_names(entries: &[OpEntry]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|entry| (entry.uuid.clone(), entry.name.clone()))
        .collect()
}

fn whitelist_names(entries: &[WhitelistEntry]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|entry| (entry.uuid.clone(), entry.name.clone()))
        .collect()
}

fn ban_entries(entries: &[BanEntry]) -> BTreeMap<String, (String, String)> {
    entries
        .iter()
        .map(|entry| {
            (
                entry.uuid.clone(),
                (entry.name.clone(), entry.reason.clone()),
            )
        })
        .collect()
}

fn command_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Ok(name.to_string());
    }
    Err(format!("서버 명령으로 반영할 수 없는 닉네임입니다: {name}"))
}

fn command_reason(reason: &str) -> String {
    let reason = reason.replace(['\r', '\n'], " ");
    let reason = reason.trim();
    if reason.is_empty() {
        "Banned by an operator.".to_string()
    } else {
        reason.to_string()
    }
}

fn config_file_paths(kind: &ServerKind) -> Vec<&'static str> {
    let mut paths = vec!["bukkit.yml", "spigot.yml"];
    match kind {
        ServerKind::Paper | ServerKind::Folia => {
            paths.extend([
                "paper.yml",
                "config/paper-global.yml",
                "config/paper-world-defaults.yml",
            ]);
        }
        ServerKind::Purpur => {
            paths.extend(["paper.yml", "purpur.yml", "config/paper-global.yml"]);
        }
        ServerKind::Vanilla => {}
    }
    paths
}

pub async fn backup_existing(server_dir: &Path, path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let backup_dir = server_dir.join("config-backups");
    fs::create_dir_all(&backup_dir)
        .await
        .map_err(|error| format!("설정 백업 폴더 생성 실패: {error}"))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "백업 파일 이름을 확인할 수 없습니다.".to_string())?;
    let target = backup_dir.join(format!("{name}.{}.bak", timestamp()));
    fs::copy(path, target)
        .await
        .map_err(|error| format!("설정 백업 실패: {error}"))?;
    Ok(())
}

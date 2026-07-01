use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use sysinfo::System;
use tokio::{process::ChildStdin, sync::Mutex};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServerKind {
    Folia,
    Paper,
    Purpur,
    Vanilla,
}

impl ServerKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Folia => "folia",
            Self::Paper => "paper",
            Self::Purpur => "purpur",
            Self::Vanilla => "vanilla",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Folia => "Folia",
            Self::Paper => "Paper",
            Self::Purpur => "Purpur",
            Self::Vanilla => "Vanilla",
        }
    }

    pub fn papermc_project(&self) -> Option<&'static str> {
        match self {
            Self::Folia => Some("folia"),
            Self::Paper => Some("paper"),
            _ => None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerProperties {
    pub server_port: u16,
    pub online_mode: bool,
    pub motd: String,
    pub max_players: u32,
    pub difficulty: String,
    pub gamemode: String,
    pub pvp: bool,
    pub view_distance: u32,
    pub simulation_distance: u32,
    pub enable_command_block: bool,
    pub white_list: bool,
}

impl Default for ServerProperties {
    fn default() -> Self {
        Self {
            server_port: 25565,
            online_mode: true,
            motd: "A Minecraft Server".to_string(),
            max_players: 20,
            difficulty: "easy".to_string(),
            gamemode: "survival".to_string(),
            pvp: true,
            view_distance: 10,
            simulation_distance: 10,
            enable_command_block: false,
            white_list: false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    pub kind: ServerKind,
    pub minecraft_version: String,
    pub server_dir: String,
    pub memory_mb: u32,
    pub java_path: Option<String>,
    pub last_used: Option<String>,
    pub settings: ServerProperties,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProfileInput {
    pub name: String,
    pub kind: ServerKind,
    pub minecraft_version: String,
    pub server_dir: Option<String>,
    pub memory_mb: u32,
    pub java_path: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub selected_profile_id: Option<String>,
    pub profiles: Vec<ServerProfile>,
}

#[derive(Default)]
pub struct ServerRuntime {
    pub status: String,
    pub players: BTreeSet<String>,
    pub logs: Vec<String>,
    pub current_profile_id: Option<String>,
    pub java: Option<JavaRuntime>,
    pub stdin: Option<Arc<Mutex<ChildStdin>>>,
    pub crash_detected: bool,
    pub exit_message: Option<String>,
    pub stop_requested: bool,
}

pub struct AppState {
    pub runtime: Arc<Mutex<ServerRuntime>>,
    pub http: Client,
    pub system: Arc<Mutex<System>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            runtime: Arc::new(Mutex::new(ServerRuntime {
                status: "stopped".to_string(),
                ..ServerRuntime::default()
            })),
            http: Client::builder()
                .user_agent("Minehub Server Launcher (kr.minehub.mclauncher)")
                .build()
                .expect("reqwest client should build"),
            system: Arc::new(Mutex::new(System::new_all())),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerVersion {
    pub id: String,
    pub kind: String,
    pub label: String,
}

#[derive(Deserialize)]
pub struct VersionManifest {
    pub versions: Vec<MinecraftVersion>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MinecraftVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Deserialize)]
pub struct VersionDetail {
    pub downloads: MinecraftDownloads,
    #[serde(rename = "javaVersion")]
    pub java_version: MinecraftJavaVersion,
}

#[derive(Deserialize)]
pub struct MinecraftDownloads {
    pub server: Option<MinecraftDownload>,
}

#[derive(Clone, Deserialize)]
pub struct MinecraftDownload {
    pub sha1: String,
    pub url: String,
}

#[derive(Deserialize)]
pub struct MinecraftJavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

#[derive(Deserialize)]
pub struct PaperProject {
    pub versions: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Deserialize)]
pub struct PaperBuild {
    pub id: u32,
    pub downloads: BTreeMap<String, PaperFile>,
}

#[derive(Clone, Deserialize)]
pub struct PaperFile {
    pub name: String,
    pub checksums: PaperChecksums,
    pub url: String,
}

#[derive(Clone, Deserialize)]
pub struct PaperChecksums {
    pub sha256: Option<String>,
}

#[derive(Deserialize)]
pub struct PurpurProject {
    pub versions: Vec<String>,
}

#[derive(Deserialize)]
pub struct PurpurBuilds {
    pub builds: PurpurBuildList,
}

#[derive(Deserialize)]
pub struct PurpurBuildList {
    pub latest: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRuntime {
    pub path: String,
    pub major: u32,
    pub version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerPlan {
    pub profile_id: String,
    pub version: String,
    pub server_kind: ServerKind,
    pub required_java: u32,
    pub java_component: String,
    pub java: Option<JavaRuntime>,
    pub server_available: bool,
    pub server_note: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub running: bool,
    pub status: String,
    pub players: Vec<String>,
    pub logs: Vec<String>,
    pub current_profile_id: Option<String>,
    pub java: Option<JavaRuntime>,
    pub data_dir: String,
    pub crash_detected: bool,
    pub exit_message: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDiagnostics {
    pub port: u16,
    pub local_address: Option<String>,
    pub public_address: Option<String>,
    pub lan_endpoint: Option<String>,
    pub public_endpoint: Option<String>,
    pub local_reachable: bool,
    pub external_reachable: Option<bool>,
    pub note: String,
    pub checked_at: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpnpMappingResult {
    pub external_address: Option<String>,
    pub internal_address: String,
    pub external_port: u16,
    pub internal_port: u16,
    pub protocol: String,
    pub note: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub memory_usage: f32,
    pub sampled_at: u64,
}

#[derive(Clone, Serialize)]
pub struct ServerLogEvent {
    pub line: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfigBundle {
    pub properties: ServerProperties,
    pub properties_raw: String,
    pub config_files: Vec<ConfigFile>,
    pub config_fields: Vec<ConfigFormField>,
    pub json_lists: Vec<JsonListFile>,
    pub restart_required: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFormField {
    pub file: String,
    pub path: String,
    pub label: String,
    pub kind: ConfigFieldKind,
    pub value: ConfigFieldValue,
    pub options: Option<Vec<String>>,
    pub restart_required: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFieldKind {
    Boolean,
    Number,
    Text,
    Select,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigFieldValue {
    Boolean(bool),
    Number(f64),
    Text(String),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFile {
    pub name: String,
    pub relative_path: String,
    pub exists: bool,
    pub editable: bool,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonListFile {
    pub name: String,
    pub exists: bool,
    pub content: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpEntry {
    pub uuid: String,
    pub name: String,
    pub level: u8,
    pub bypasses_player_limit: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WhitelistEntry {
    pub uuid: String,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BanEntry {
    pub uuid: String,
    pub name: String,
    pub created: String,
    pub source: String,
    pub expires: String,
    pub reason: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessLists {
    pub ops: Vec<OpEntry>,
    pub whitelist: Vec<WhitelistEntry>,
    pub banned_players: Vec<BanEntry>,
    pub raw_ops: String,
    pub raw_whitelist: String,
    pub raw_banned_players: String,
}

#[derive(Serialize)]
pub struct MinecraftIdentity {
    pub name: String,
    pub uuid: String,
}

#[derive(Deserialize)]
pub struct MojangProfile {
    pub id: String,
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginFile {
    pub filename: String,
    pub display_name: String,
    pub enabled: bool,
    pub size: u64,
}

#[derive(Deserialize)]
pub struct ModrinthSearchResponse {
    pub hits: Vec<ModrinthProject>,
}

#[derive(Serialize, Deserialize)]
pub struct ModrinthProject {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub categories: Vec<String>,
    pub versions: Vec<String>,
}

#[derive(Deserialize)]
pub struct ModrinthVersion {
    pub version_number: String,
    pub version_type: String,
    pub files: Vec<ModrinthFile>,
}

#[derive(Clone, Deserialize)]
pub struct ModrinthFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub hashes: Option<ModrinthHashes>,
}

#[derive(Clone, Deserialize)]
pub struct ModrinthHashes {
    pub sha1: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPlugin {
    pub title: String,
    pub version: String,
    pub filename: String,
    pub path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub filename: String,
    pub path: String,
    pub size: u64,
}

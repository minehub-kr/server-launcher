use crate::models::{JavaRuntime, ServerProfile};
use std::{env, path::Path};

#[tauri::command]
pub async fn scan_java_versions() -> Result<Vec<JavaRuntime>, String> {
    tauri::async_runtime::spawn_blocking(scan_java_sync)
        .await
        .map_err(|error| format!("Java 탐색 실패: {error}"))
}

pub async fn choose_java(
    profile: &ServerProfile,
    required_major: u32,
) -> Result<Option<JavaRuntime>, String> {
    if let Some(path) = &profile.java_path {
        let path = std::path::PathBuf::from(path);
        if let Some(java) = inspect_java(&path) {
            if java.major >= required_major {
                return Ok(Some(java));
            }
            return Err(format!(
                "지정한 Java는 {}입니다. Minecraft {}에는 Java {}+가 필요합니다.",
                java.major, profile.minecraft_version, required_major
            ));
        }
    }

    let mut versions = scan_java_versions().await?;
    versions.sort_by_key(|java| (java.major < required_major, java.major));
    Ok(versions
        .into_iter()
        .filter(|java| java.major >= required_major)
        .min_by_key(|java| java.major))
}

fn scan_java_sync() -> Vec<JavaRuntime> {
    let mut candidates = java_candidates();
    candidates.sort();
    candidates.dedup();

    let mut runtimes: Vec<JavaRuntime> = candidates
        .into_iter()
        .filter_map(|path| inspect_java(&path))
        .collect();
    runtimes.sort_by(|a, b| a.major.cmp(&b.major).then_with(|| a.path.cmp(&b.path)));
    runtimes.dedup_by(|a, b| a.path == b.path);
    runtimes
}

fn java_candidates() -> Vec<std::path::PathBuf> {
    let mut candidates = Vec::new();
    let executable = if cfg!(windows) { "java.exe" } else { "java" };

    if let Ok(java_home) = env::var("JAVA_HOME") {
        candidates.push(Path::new(&java_home).join("bin").join(executable));
    }

    if let Some(path_var) = env::var_os("PATH") {
        candidates.extend(env::split_paths(&path_var).map(|path| path.join(executable)));
    }

    for root in [
        "/Library/Java/JavaVirtualMachines",
        "/System/Library/Java/JavaVirtualMachines",
        "/usr/lib/jvm",
    ] {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                candidates.push(entry.path().join("Contents/Home/bin").join(executable));
                candidates.push(entry.path().join("bin").join(executable));
            }
        }
    }

    if cfg!(windows) {
        for var in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Ok(root) = env::var(var) {
                for vendor in ["Eclipse Adoptium", "Java", "Microsoft", "Zulu"] {
                    if let Ok(entries) = std::fs::read_dir(Path::new(&root).join(vendor)) {
                        for entry in entries.flatten() {
                            candidates.push(entry.path().join("bin").join(executable));
                        }
                    }
                }
            }
        }
    }

    candidates
        .into_iter()
        .filter(|path| path.exists())
        .map(|path| path.canonicalize().unwrap_or(path))
        .collect()
}

fn inspect_java(path: &Path) -> Option<JavaRuntime> {
    let output = std::process::Command::new(path)
        .arg("-version")
        .output()
        .ok()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let version_line = text.lines().find(|line| line.contains("version"))?;
    let quoted = version_line
        .split('"')
        .nth(1)
        .unwrap_or(version_line)
        .to_string();
    let major = parse_java_major(&quoted)?;

    Some(JavaRuntime {
        path: path.to_string_lossy().to_string(),
        major,
        version: version_line.to_string(),
    })
}

fn parse_java_major(version: &str) -> Option<u32> {
    let mut parts = version.split(['.', '+', '-', '_']);
    let first = parts.next()?.parse::<u32>().ok()?;
    if first == 1 {
        parts.next()?.parse::<u32>().ok()
    } else {
        Some(first)
    }
}

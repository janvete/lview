use std::fmt;

use crate::ssh::SshSession;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LogSource {
    File { path: String },
    Docker { container: String, image: String },
    Journal { unit: String },
}

impl fmt::Display for LogSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogSource::File { path } => write!(f, "{}", path),
            LogSource::Docker { container, image } => write!(f, "{} ({})", container, image),
            LogSource::Journal { unit } => write!(f, "{}", unit),
        }
    }
}

impl LogSource {
    pub fn category(&self) -> &'static str {
        match self {
            LogSource::File { path } => {
                if path.starts_with("/opt/") {
                    "opt"
                } else if path.starts_with("/var/log/") {
                    "system"
                } else {
                    "other"
                }
            }
            LogSource::Docker { .. } => "docker",
            LogSource::Journal { .. } => "journal",
        }
    }

    /// Human-readable label shown in the log picker.
    pub fn display_name(&self) -> String {
        match self {
            LogSource::File { path } => {
                let basename = std::path::Path::new(path)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone());
                if let Some(parent) = std::path::Path::new(path).parent() {
                    let parent_str = parent.to_string_lossy();
                    if parent_str == "/var/log" {
                        return format!("{} (system)", basename);
                    } else if parent_str.starts_with("/var/log/") || parent_str.starts_with("/opt/") {
                        return format!("{} ({})", basename, parent_str);
                    }
                }
                format!("{} ({})", basename, path)
            }
            LogSource::Docker { container, image } => format!("{} (docker: {})", container, image),
            LogSource::Journal { unit } => format!("{} (journal)", unit),
        }
    }

    /// Text used for fuzzy searching (label + raw source).
    pub fn search_text(&self) -> String {
        format!("{} {}", self.display_name(), self)
    }

    pub fn sort_key(&self) -> String {
        format!("{}:{}", self.category(), self.display_name())
    }
}

pub struct DiscoveryResult {
    pub sources: Vec<LogSource>,
    pub errors: Vec<String>,
}

pub async fn discover_logs(session: &SshSession, extra_paths: &[String], timeout_secs: u64) -> DiscoveryResult {
    let mut result = DiscoveryResult {
        sources: Vec::new(),
        errors: Vec::new(),
    };

    let system = discover_system_logs(session, timeout_secs);
    let docker = discover_docker(session, timeout_secs);
    let opt = discover_opt_logs(session, extra_paths, timeout_secs);
    let journal = discover_journal(session, timeout_secs);

    let (system, docker, opt, journal) = tokio::join!(system, docker, opt, journal);

    match system {
        Ok(items) => result.sources.extend(items),
        Err(e) => result.errors.push(format!("system logs: {}", e)),
    }
    match docker {
        Ok(items) => result.sources.extend(items),
        Err(e) => result.errors.push(format!("docker: {}", e)),
    }
    match opt {
        Ok(items) => result.sources.extend(items),
        Err(e) => result.errors.push(format!("/opt logs: {}", e)),
    }
    match journal {
        Ok(items) => result.sources.extend(items),
        Err(e) => result.errors.push(format!("journal: {}", e)),
    }

    result.sources.sort_by_key(|a| a.sort_key());
    result.sources.dedup();
    result
}

async fn discover_system_logs(session: &SshSession, timeout_secs: u64) -> Result<Vec<LogSource>, String> {
    let cmd = "find /var/log -maxdepth 3 -type f 2>/dev/null | head -n 200";
    let out = session.exec(cmd, timeout_secs).await.map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out);
    let mut items = Vec::new();
    for line in text.lines() {
        let path = line.trim();
        if !path.is_empty() {
            items.push(LogSource::File { path: path.to_string() });
        }
    }
    Ok(items)
}

async fn discover_docker(session: &SshSession, timeout_secs: u64) -> Result<Vec<LogSource>, String> {
    let cmd = "docker ps --format '{{.Names}}\\t{{.Image}}' 2>/dev/null | head -n 200";
    let out = session.exec(cmd, timeout_secs).await.map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out);
    let mut items = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            items.push(LogSource::Docker {
                container: parts[0].trim().to_string(),
                image: parts[1].trim().to_string(),
            });
        }
    }
    Ok(items)
}

async fn discover_opt_logs(
    session: &SshSession,
    extra_paths: &[String],
    timeout_secs: u64,
) -> Result<Vec<LogSource>, String> {
    let mut paths = vec!["/opt".to_string()];
    paths.extend(extra_paths.iter().cloned());
    let paths_str = paths
        .iter()
        .map(|p| shell_escape(p))
        .collect::<Vec<_>>()
        .join(" ");

    let cmd = format!(
        "find {} -maxdepth 3 -type f \\( -name '*.log' -o -name 'docker-compose.yml' -o -name 'compose.yml' \\) 2>/dev/null | head -n 200",
        paths_str
    );
    let out = session.exec(&cmd, timeout_secs).await.map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out);
    let mut items = Vec::new();
    for line in text.lines() {
        let path = line.trim();
        if !path.is_empty() {
            items.push(LogSource::File { path: path.to_string() });
        }
    }
    Ok(items)
}

async fn discover_journal(session: &SshSession, timeout_secs: u64) -> Result<Vec<LogSource>, String> {
    let cmd = "systemctl list-units --type=service --state=running --no-pager --plain 2>/dev/null | tail -n +2 | head -n 100";
    let out = session.exec(cmd, timeout_secs).await.map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out);
    let mut items = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if let Some(unit) = parts.first()
            && unit.ends_with(".service")
        {
            items.push(LogSource::Journal {
                unit: unit.to_string(),
            });
        }
    }
    Ok(items)
}

fn shell_escape(s: &str) -> String {
    if s.contains(' ') || s.contains('"') || s.contains('\\') || s.contains('\'') {
        format!("'{}'", s.replace('\'', "'\"'\"'"))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categories() {
        assert_eq!(
            LogSource::File { path: "/var/log/syslog".into() }.category(),
            "system"
        );
        assert_eq!(
            LogSource::File { path: "/opt/app/app.log".into() }.category(),
            "opt"
        );
        assert_eq!(
            LogSource::Docker { container: "c".into(), image: "i".into() }.category(),
            "docker"
        );
    }
}

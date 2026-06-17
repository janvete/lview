use crate::discovery::LogSource;
use crate::ssh::SshSession;

pub fn start_stream(session: &SshSession, source: &LogSource, tail_lines: usize) -> std::io::Result<tokio::sync::mpsc::UnboundedReceiver<String>> {
    let remote_command = build_command(source, tail_lines);
    session.stream_lines(&remote_command)
}

fn build_command(source: &LogSource, tail_lines: usize) -> String {
    match source {
        LogSource::File { path } => {
            format!("tail -n {} -f {}", tail_lines, shell_escape(path))
        }
        LogSource::Docker { container, .. } => {
            format!(
                "docker logs -f --tail {} {}",
                tail_lines,
                shell_escape(container)
            )
        }
        LogSource::Journal { unit } => {
            format!(
                "journalctl -u {} -f -n {} --no-pager",
                shell_escape(unit),
                tail_lines
            )
        }
    }
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
    fn test_build_command() {
        let s = LogSource::File { path: "/var/log/syslog".into() };
        assert!(build_command(&s, 100).contains("tail -n 100 -f /var/log/syslog"));

        let d = LogSource::Docker { container: "web".into(), image: "nginx".into() };
        assert!(build_command(&d, 50).contains("docker logs -f --tail 50 web"));
    }
}

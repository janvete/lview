use std::io;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct SshTarget {
    pub args: Vec<String>,
}

impl SshTarget {
    pub fn display(&self) -> String {
        self.args.join(" ")
    }

    /// Best-effort extraction of the host part from ssh args.
    pub fn host(&self) -> String {
        let skip_flags: &[&str] = &[
            "-p", "-P", "-i", "-l", "-F", "-o", "-W", "-D", "-L", "-R", "-J", "-c", "-m", "-Q",
            "-w", "-b", "-e", "-E",
        ];
        let mut iter = self.args.iter().peekable();
        while let Some(arg) = iter.next() {
            if arg.starts_with('-') {
                // Flag with inline value (e.g. -p22)
                let short = arg.get(..2).unwrap_or(arg);
                if skip_flags.contains(&short) && arg.len() == 2 {
                    iter.next(); // skip separate value
                }
            } else {
                return arg.clone();
            }
        }
        self.display()
    }
}

#[derive(Debug, Clone)]
pub struct SshSession {
    pub command: String,
    pub target: SshTarget,
}

impl SshSession {
    pub fn new(command: String, target: SshTarget) -> Self {
        Self { command, target }
    }

    fn base_cmd(&self) -> Command {
        let mut cmd = Command::new(&self.command);
        // Non-interactive mode: do not prompt for host keys or passwords.
        cmd.arg("-o").arg("BatchMode=yes");
        cmd.arg("-o").arg("StrictHostKeyChecking=accept-new");
        cmd.args(&self.target.args);
        cmd.stdin(Stdio::null());
        cmd
    }

    pub async fn exec(&self, remote_command: &str, secs: u64) -> io::Result<Vec<u8>> {
        let mut cmd = self.base_cmd();
        cmd.arg("--").arg(remote_command);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;
        let dur = Duration::from_secs(secs);
        let result = timeout(dur, child.wait()).await;

        match result {
            Ok(Ok(status)) => {
                let mut stdout = child.stdout.take().unwrap();
                let mut out = Vec::new();
                stdout.read_to_end(&mut out).await?;
                if status.success() {
                    Ok(out)
                } else {
                    let mut stderr = Vec::new();
                    if let Some(mut s) = child.stderr.take() {
                        let _ = s.read_to_end(&mut stderr).await;
                    }
                    Err(io::Error::other(format!(
                        "ssh command failed with status {}: {}",
                        status,
                        String::from_utf8_lossy(&stderr)
                    )))
                }
            }
            Ok(Err(e)) => Err(io::Error::other(e)),
            Err(_) => {
                let _ = child.start_kill();
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("ssh command timed out after {}s", secs),
                ))
            }
        }
    }

    pub fn spawn_stream(&self, remote_command: &str) -> io::Result<Child> {
        let mut cmd = self.base_cmd();
        cmd.arg("--").arg(remote_command);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());
        cmd.kill_on_drop(true);
        cmd.spawn()
    }

    /// Spawn a remote command and return a channel of lines from stdout.
    pub fn stream_lines(&self, remote_command: &str) -> io::Result<mpsc::UnboundedReceiver<String>> {
        let mut child = self.spawn_stream(remote_command)?;
        let stdout = child.stdout.take().ok_or_else(|| {
            io::Error::other("failed to capture stdout")
        })?;

        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if tx.send(line).is_err() {
                    break;
                }
            }
            let _ = child.wait().await;
        });
        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_extraction() {
        let t = SshTarget {
            args: vec!["-p".into(), "22".into(), "root@192.168.53.3".into()],
        };
        assert_eq!(t.host(), "root@192.168.53.3");

        let t2 = SshTarget {
            args: vec!["-p22".into(), "root@example.com".into()],
        };
        assert_eq!(t2.host(), "root@example.com");
    }
}

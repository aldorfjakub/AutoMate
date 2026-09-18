use std::{process::Stdio, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    time::timeout,
};
use uuid::Uuid;

use crate::*;

pub struct PreparedBot {
    pub bot_id: Option<Uuid>,
    pub docker_name: String,
    pub stdin: ChildStdin,
    pub reader: BufReader<ChildStdout>,
    pub stderr_reader: BufReader<ChildStderr>,
    pub process: Child,
}

/// Spawn the python-runner container, upload the bot's source code, and wait
/// for the worker to signal READY. Returns a [`PreparedBot`] usable for
/// validation or a match, or `Err(reason)` if anything during setup fails.
pub async fn prepare_bot(source_code: &str, bot_id: Option<Uuid>) -> Result<PreparedBot, String> {
    let docker_name = format!("bot-{}", Uuid::new_v4());
    let mut process = Command::new("docker")
        .args(&[
            "run",
            "-i",
            "--rm",
            "--name",
            &docker_name,
            "--read-only",
            "--tmpfs",
            "/tmp:rw,noexec,nosuid,nodev,size=64m",
            "--cap-drop=ALL",
            "--security-opt=no-new-privileges:true",
            "--network=none",
            "--memory=256m",
            "--memory-swap=256m",
            "--cpus=0.5",
            "--pids-limit=64",
            "python-runner",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("failed to spawn container: {e}"))?;

    let mut stdin = match process.stdin.take() {
        Some(s) => s,
        None => {
            let _ = process.kill().await;
            return Err("container has no stdin pipe".to_string());
        }
    };
    let stdout = process.stdout.take().expect("missing stdout pipe");
    let stderr = process.stderr.take().expect("missing stderr pipe");
    let mut reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Upload the source code, terminated by the sentinel the wrapper reads.
    let source_bytes = source_code.as_bytes();
    if stdin.write_all(source_bytes).await.is_err()
        || stdin.write_all(b"\n").await.is_err()
        || stdin
            .write_all(b"===END_OF_WORKER_CODE===\n")
            .await
            .is_err()
    {
        kill_bot(&mut PreparedBot {
            bot_id,
            docker_name,
            stdin,
            reader,
            stderr_reader,
            process,
        })
        .await;
        return Err("failed to send code to worker".to_string());
    }
    let _ = stdin.flush().await;

    // TODO check if this container can't get stuck by keep printing stuff and stuck in loop
    // Wait for the worker to finish loading its code and print READY.
    let mut line = String::new();
    loop {
        line.clear();
        match timeout(Duration::from_secs(10), reader.read_line(&mut line)).await {
            Ok(Ok(0)) => {
                return Err("worker exited before READY".to_string());
            }
            Ok(Ok(_)) => (),
            Ok(Err(e)) => {
                return Err(format!("read error while loading worker: {e}"));
            }
            Err(_) => {
                return Err("worker code took too long to load".to_string());
            }
        }
        let trimmed = line.trim();
        if trimmed == "READY" {
            break;
        }
        if trimmed.starts_with("ERROR") {
            println!("{}", trimmed);
            return Err("worker failed to load".to_string());
        }
    }

    Ok(PreparedBot {
        bot_id,
        docker_name,
        stdin,
        reader,
        stderr_reader,
        process,
    })
}

pub async fn kill_bot(prepared: &mut PreparedBot) {
    if let Ok(mut child) = Command::new("docker")
        .args(["kill", &prepared.docker_name])
        .spawn()
    {
        let _ = timeout(Duration::from_secs(5), child.wait()).await;
    }
    let _ = Command::new("docker")
        .args(["rm", "--force", &prepared.docker_name])
        .spawn();

    let _ = prepared.process.kill().await;
    let _ = timeout(Duration::from_secs(2), prepared.process.wait()).await;
}

fn parse_move_line(line: &str) -> Option<Result<String, String>> {
    let line = line.trim();
    if let Some(uci) = line.strip_prefix("move:") {
        Some(Ok(uci.trim().to_string()))
    } else if line.starts_with("ERROR") {
        Some(Err(line.to_string()))
    } else {
        None
    }
}

pub async fn read_move(reader: &mut BufReader<ChildStdout>) -> Result<String, String> {
    let mut line = String::new();
    let deadline = Instant::now() + Duration::from_millis(1200);
    for _ in 0..10 {
        line.clear();
        match timeout_at(deadline, reader.read_line(&mut line)).await {
            Ok(Ok(0)) => {
                return Err("Bot exited".into());
            }
            Ok(Ok(_)) => (),
            Ok(Err(_)) => {
                return Err("failed to read a line".into());
            }
            Err(_) => {
                return Err("took too long to respond".into());
            }
        }

        if line.len() > 1_000_000 {
            return Err("line too large".into());
        }

        let trimmed = line.trim();
        match parse_move_line(trimmed) {
            Some(Ok(uci)) => return Ok(uci),
            Some(Err(msg)) => {
                println!("{}", msg);
                return Err(msg);
            }
            None => {}
        }
    }
    Err("Bot has outputed too many lines".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_move_line() {
        assert_eq!(parse_move_line("move: e2e4"), Some(Ok("e2e4".to_string())));
        assert_eq!(parse_move_line("  move:  d2d4  "), Some(Ok("d2d4".to_string())));
    }

    #[test]
    fn parses_error_line() {
        assert!(matches!(parse_move_line("ERROR: boom"), Some(Err(e)) if e == "ERROR: boom"));
    }

    #[test]
    fn ignores_junk_lines() {
        assert_eq!(parse_move_line("received: rnbqkbnr..."), None);
        assert_eq!(parse_move_line(""), None);
    }
}

use std::process::Stdio;

use anyhow::{Result, bail};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::{Child, Command},
    sync::mpsc::{UnboundedReceiver, unbounded_channel},
};

use super::command::LaunchCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessLogSource {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessLogLine {
    pub source: ProcessLogSource,
    pub line: String,
}

pub struct LoggedChild {
    pub child: Child,
    pub lines: UnboundedReceiver<ProcessLogLine>,
}

pub fn spawn_command(command: LaunchCommand) -> Result<Child> {
    if command.program.as_os_str().is_empty() {
        bail!("launch executable is empty");
    }

    let mut child = Command::new(&command.program);
    child.args(&command.args);

    if let Some(cwd) = &command.cwd {
        child.current_dir(cwd);
    }

    for (key, value) in &command.env {
        child.env(key, value);
    }

    Ok(child.spawn()?)
}

pub fn spawn_command_logged(command: LaunchCommand) -> Result<LoggedChild> {
    if command.program.as_os_str().is_empty() {
        bail!("launch executable is empty");
    }

    let mut child = Command::new(&command.program);
    child.args(&command.args);

    if let Some(cwd) = &command.cwd {
        child.current_dir(cwd);
    }

    for (key, value) in &command.env {
        child.env(key, value);
    }

    let mut child = child
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let (sender, receiver) = unbounded_channel();

    if let Some(stdout) = child.stdout.take() {
        spawn_log_reader(ProcessLogSource::Stdout, stdout, sender.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_log_reader(ProcessLogSource::Stderr, stderr, sender.clone());
    }
    drop(sender);

    Ok(LoggedChild {
        child,
        lines: receiver,
    })
}

fn spawn_log_reader<R>(
    source: ProcessLogSource,
    reader: R,
    sender: tokio::sync::mpsc::UnboundedSender<ProcessLogLine>,
) where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    if sender.send(ProcessLogLine { source, line }).is_err() {
                        return;
                    }
                }
                Ok(None) => return,
                Err(_) => return,
            }
        }
    });
}

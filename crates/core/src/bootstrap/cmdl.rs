use std::process::Command;

pub struct GameProcess {}

impl GameProcess {
    pub fn new() -> Self {
        GameProcess {}
    }

    pub fn run(&self, command: &str) -> std::io::Result<()> {
        let mut cmd = Command::new(command);
        cmd.spawn()?;
        Ok(())
    }
}

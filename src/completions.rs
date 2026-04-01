use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{generate, shells, Generator};

use crate::main_cli::Cli;

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum CompletionShell {
    Zsh,
    Bash,
    Fish,
    Powershell,
}

pub fn emit(shell: CompletionShell) -> Result<()> {
    let mut cmd = Cli::command();
    match shell {
        CompletionShell::Zsh => write_generated(shells::Zsh, &mut cmd),
        CompletionShell::Bash => write_generated(shells::Bash, &mut cmd),
        CompletionShell::Fish => write_generated(shells::Fish, &mut cmd),
        CompletionShell::Powershell => write_generated(shells::PowerShell, &mut cmd),
    }
}

pub fn zsh_script() -> Result<String> {
    let mut cmd = Cli::command();
    generate_to_string(shells::Zsh, &mut cmd)
}

pub fn powershell_script() -> Result<String> {
    let mut cmd = Cli::command();
    generate_to_string(shells::PowerShell, &mut cmd)
}

fn write_generated<G: Generator>(generator: G, cmd: &mut clap::Command) -> Result<()> {
    let name = cmd.get_name().to_string();
    let mut stdout = std::io::stdout();
    generate(generator, cmd, name, &mut stdout);
    Ok(())
}

fn generate_to_string<G: Generator>(generator: G, cmd: &mut clap::Command) -> Result<String> {
    let name = cmd.get_name().to_string();
    let mut buf: Vec<u8> = Vec::new();
    generate(generator, cmd, name, &mut buf);
    Ok(String::from_utf8(buf)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zsh_script_contains_command_name() {
        let script = zsh_script().expect("zsh completion script should be generated");
        assert!(script.contains("projm"));
    }

    #[test]
    fn powershell_script_contains_command_name() {
        let script = powershell_script().expect("powershell completion script should be generated");
        assert!(script.contains("projm"));
    }
}


use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{generate, shells, Generator};

use crate::main_cli::Cli;

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionShell {
    Zsh,
    Bash,
    Fish,
    Powershell,
    Nushell,
}

pub fn emit(shell: CompletionShell) -> Result<()> {
    let mut cmd = Cli::command();
    match shell {
        CompletionShell::Zsh => write_generated(shells::Zsh, &mut cmd),
        CompletionShell::Bash => write_generated(shells::Bash, &mut cmd),
        CompletionShell::Fish => write_generated(shells::Fish, &mut cmd),
        CompletionShell::Powershell => write_generated(shells::PowerShell, &mut cmd),
        CompletionShell::Nushell => {
            print!("{}", nushell_script()?);
            Ok(())
        }
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

pub fn nushell_script() -> Result<String> {
    Ok(r#"# projm completions for Nushell
export extern "projm" []

export extern "projm organize" [
    dir: path             # Directory to scan
    --dry-run(-n)         # Preview only — no files moved
]

export extern "projm g" [
    query?: string        # Search query or project name
    --last(-l)            # Jump to the last entered project
]

export extern "projm init" [
    --alias(-a): string = "pg"  # Shell function/alias name
    --non-interactive           # Run in non-interactive mode
    --shell(-s): string         # Override shell target
    --profile-path(-p): path    # Override shell profile path
]

export extern "projm completions" [
    shell: string         # Target shell (zsh, bash, fish, powershell, nushell)
]

export extern "projm set-base" [
    path: path            # Override the base projects directory
]

export extern "projm editors" []

export extern "projm blueprint" []
export extern "projm blueprint add" []
export extern "projm blueprint list" []
export extern "projm blueprint run" [ name?: string ]
export extern "projm blueprint edit" [ name?: string ]
export extern "projm blueprint delete" [ name?: string ]

export extern "projm check" []

export extern "projm run" [
    path_or_query?: string
]

export extern "projm clone" [
    url: string
    name?: string
    --branch(-b): string
    --open(-o)
]
"#.to_string())
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

    #[test]
    fn nushell_script_contains_command_name() {
        let script = nushell_script().expect("nushell completion script should be generated");
        assert!(script.contains("projm"));
    }
}


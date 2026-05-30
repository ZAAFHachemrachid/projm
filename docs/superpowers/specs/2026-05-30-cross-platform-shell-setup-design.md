# Design Spec: Multi-Shell, Cross-Platform Shell Setup & completions (`projm init`)

A modular, robust, cross-platform shell integration and auto-completion system for `projm init`. It automatically detects the user's active shell (Zsh, Bash, Fish, PowerShell, or Nushell), resolves appropriate configuration and completion profile paths, permits custom overrides, and safely updates shell profile files.

---

## 1. Objectives

- **Multi-Shell support**: Add first-class support for `Zsh`, `Bash`, `Fish`, `PowerShell`, and `Nushell`.
- **Cross-Platform robustness**: Work flawlessly on Windows, macOS, and Linux.
- **Dynamic detection**: Auto-detect the active shell by scanning the `$SHELL` environment variable or checking the parent process, showing the detected shell as the default in the wizard.
- **Path customization**: Allow the user to review the profile path and customize it (via interactive prompt or `--profile-path` CLI option).
- **Absolute-path sourcing**: Sourced completions in configuration blocks will use exact absolute paths to prevent resolution failures.
- **Complete completions**: Expand `projm completions` to cover all five shells, including a custom native Nushell completions generator.

---

## 2. CLI Interface Changes

We will modify the `Init` command in `crates/projm-cli/src/main_cli.rs`:

```rust
Init {
    /// Shell function/alias name
    #[arg(short = 'a', long, default_value = "pg")]
    alias: String,

    /// Run in non-interactive mode, bypassing the onboarding wizard
    #[arg(long)]
    non_interactive: bool,

    /// Override the shell target to configure
    #[arg(short = 's', long, value_enum)]
    shell: Option<crate::completions::CompletionShell>,

    /// Override the shell profile path to update
    #[arg(short = 'p', long)]
    profile_path: Option<std::path::PathBuf>,
}
```

We will also update `crates/projm-cli/src/completions.rs`'s `CompletionShell` to support `Nushell`:

```rust
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionShell {
    Zsh,
    Bash,
    Fish,
    Powershell,
    Nushell,
}
```

---

## 3. Dynamic Shell Detection

We will implement active shell detection:

1. **Environment Variables**: Inspect `$SHELL` or `$PROJM_SHELL`. If it contains `zsh`, `bash`, `fish`, `nu` / `nushell`, or `pwsh` / `powershell`, we select that shell.
2. **Parent Process Name**: Check parent process names if the environment variables are inconclusive.
3. **OS-Specific Defaults**:
   - **Windows**: Default to `Powershell`.
   - **macOS / Linux**: Default to `Zsh`.

---

## 4. Default Path Matrix & Query System

To resolve profile paths with maximum accuracy, `projm` will query shells programmatically if their binaries are present on the system:

| Shell | Default Profile Path | Default Completions Path | Query command (If Installed) |
|---|---|---|---|
| **Zsh** | `~/.zshrc` (or `$ZDOTDIR/.zshrc` if set) | `~/.config/zsh/completions/_projm` | N/A |
| **Bash** | macOS: `~/.bash_profile`<br>Linux/Windows: `~/.bashrc` | `~/.config/projm/completions/projm.bash` | N/A |
| **Fish** | `~/.config/fish/config.fish` | `~/.config/fish/completions/projm.fish` | N/A |
| **PowerShell** | `Documents/PowerShell/Microsoft.PowerShell_profile.ps1` | `~/.config/powershell/completions/projm.ps1` | `pwsh -NoProfile -Command "$PROFILE"` |
| **Nushell** | Unix: `~/.config/nushell/config.nu`<br>Windows: `%APPDATA%\nushell\config.nu` | `~/.config/projm/completions/projm.nu` | `nu -c "$nu.config-path"` |

---

## 5. Shell Integration Blocks

Each shell will have a specific script block injected into its configuration file:

### Zsh
```zsh
# >>> projm >>>
pg() {
    local cmd
    cmd=$(projm g "$@" 2>/dev/tty </dev/tty) || return
    [ -n "$cmd" ] && eval "$cmd"
}
pn() {
    projm run "$@"
}
fpath=("<ABS_COMPLETIONS_DIR>" $fpath)
autoload -Uz compinit && compinit
# <<< projm <<<
```

### Bash
```bash
# >>> projm >>>
pg() {
    local cmd
    cmd=$(projm g "$@" 2>/dev/tty </dev/tty) || return
    [ -n "$cmd" ] && eval "$cmd"
}
pn() {
    projm run "$@"
}
. "<ABS_COMPLETIONS_FILE>"
# <<< projm <<<
```

### Fish
```fish
# >>> projm >>>
function pg
    set -l cmd (projm g $argv 2>/dev/tty </dev/tty)
    if test -n "$cmd"
        eval $cmd
    end
end

function pn
    projm run $argv
end
# <<< projm <<<
```

### PowerShell
```powershell
# >>> projm >>>
function pg {
  $cmd = projm g $args
  if ($cmd) { Invoke-Expression $cmd }
}
function pn {
  projm run $args
}
. "<ABS_COMPLETIONS_FILE>"
# <<< projm <<<
```

### Nushell
```nushell
# >>> projm >>>
def --env pg [...args] {
    let cmd = (projm g ...$args | into string | str trim)
    if ($cmd | is-empty) == false {
        let parts = ($cmd | split row " && ")
        if ($parts | length) >= 2 {
            let cd_part = ($parts | get 0)
            let edit_part = ($parts | get 1)
            
            let path = ($cd_part | str replace -r "^(cd|z)\\s+'(.*)'$" "$2")
            cd $path
            
            let editor = ($edit_part | str replace -r "\\s+\\.$" "")
            run-external $editor "."
        }
    }
}

def --env pn [...args] {
    projm run ...$args
}
source "<ABS_COMPLETIONS_FILE>"
# <<< projm <<<
```

---

## 6. Verification & Test Plan

### Automated Tests
- Unit tests to verify correct active shell auto-detection behavior under mock env configurations.
- Unit tests to verify that `ensure_projm_block` is strictly idempotent for Zsh, Bash, Fish, PowerShell, and Nushell.
- Unit tests to verify Nushell dynamic script generation.

### Manual Verification
- Walk through the interactive `projm init` wizard in different environments (e.g. Zsh, Bash, Fish) to verify:
  1. The correct shell is auto-detected.
  2. The default profile path is detected and the user can accept it or type a custom override path.
  3. Integrations and completions are correctly written and work as expected on next shell launch.
- Test `projm init --non-interactive --shell fish --profile-path ./my_config.fish` to verify non-interactive overrides work flawlessly.

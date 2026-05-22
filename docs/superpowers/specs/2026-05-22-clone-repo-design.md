# Repository Cloning & Auto-Organization Spec

Design and specification for adding direct repository cloning and automatic classification to `projm`.

## Overview

Currently, developers using `projm` have to manually clone a repository into some directory, and then run `projm organize` to classify and structure it into their base projects folder (`~/projects/`). 

This specification introduces a direct `projm clone <url> [custom-name]` feature. It enables users to clone a remote git repository (via HTTPS/SSH) directly into a temporary workspace folder inside the base projects folder, run the automatic classification/suffix grouping engine on it, and place it immediately in its final categorized directory.

---

## User Interface & Commands

We introduce a new command path to `projm` with the following CLI signature:

```bash
projm clone <url> [name] [flags]
```

### Subcommand Specification:
* **Arguments:**
  * `url` (Required): The Git repository URL (e.g. `https://github.com/user/repo.git` or `git@github.com:user/repo.git`).
  * `name` (Optional): A custom name override for the cloned project directory.
* **Flags:**
  * `-b`, `--branch <branch>` (Optional): Clone a specific branch, tag, or revision instead of the default branch.
  * `-o`, `--open` (Optional): Automatically open the newly cloned project in the user's preferred text editor after successful organization.

---

## Technical Architecture

### 1. URL Parser & Name Extraction (`src/clone.rs`)
A helper function `extract_repo_name(url: &str) -> Option<String>` will parse git URLs to extract the default directory name:
* `https://github.com/rust-lang/regex.git` -> `regex`
* `https://github.com/rust-lang/regex` -> `regex`
* `git@github.com:rust-lang/regex.git` -> `regex`
* `file:///path/to/local/repo.git` -> `repo`

### 2. Git Clone Execution via CLI Delegation (`src/clone.rs`)
We spawn `git clone` using Rust's `std::process::Command` to delegate networking, authentication, and progress displays to the native git program:
```rust
let mut cmd = std::process::Command::new("git");
cmd.arg("clone").arg(url).arg(&temp_dir);
if let Some(branch) = branch {
    cmd.arg("--branch").arg(branch);
}
```

### 3. Execution Pipeline
1. **Verification:** Ensure the system has `git` installed by running a lightweight command or querying the PATH.
2. **Path Resolve:** Identify the potential project name. Check if a directory with this name already exists in the destination to prevent redundant downloads.
3. **Temporary Staging:** Create a temporary directory named `~/projects/.tmp_clone_<random_id>` inside the user's base projects directory. Cloning inside `base` ensures that the subsequent move operation is on the same filesystem/mount point, enabling instantaneous directory rename operations.
4. **Clone Action:** Execute the `git clone` command. If the clone fails, clean up the temporary directory and exit with the appropriate error.
5. **Classify & Move:** Run `classify::classify` on the temporary folder to determine its `Category`. Move the directory to its final resolved target: `~/projects/<category>/[group-folder/]<name>`.
6. **Editor Launch:** If `-o`/`--open` is passed, trigger `projm`'s existing editor detection to open the workspace.

---

## Error Handling & Clean-up

* **Aborted/Failed Clones:** If `git clone` returns a non-zero exit code or is terminated, a drop-guard (or `std::fs::remove_dir_all` block) ensures `~/projects/.tmp_clone_<random_id>` is deleted immediately to prevent cluttering the projects base directory.
* **Pre-existing Target Check:** Prior to starting the clone, check if `~/projects/<any-category>/<name>` or `~/projects/<any-category>/<group>/<name>` already exists. If it does, abort early with a descriptive error message.

---

## Verification Plan

### Automated Tests
1. **`test_extract_repo_name`**: Validate that HTTPS, SSH, and file URLs are correctly parsed to get the base repository name.
2. **`test_clone_and_organize_integration`**: Use a local mock git repo to perform a test clone inside the test suite base. Verify that the project is cloned, classified under `services` or `tools`, moved to the correct destination, and the temp directory is cleaned up successfully.

### Manual Verification
1. Run `projm clone https://github.com/rust-lang/regex` and verify it is cloned and organized under `~/projects/tools/regex` (or the classified category).
2. Run `projm clone https://github.com/rust-lang/regex my-regex --open` and verify it is renamed to `my-regex` and opens in the editor.

# Projm Context

A command-line tool and shell integration that scans, classifies, groups, and navigates developer directories automatically.

## Language

**Project**:
A directory containing source code or configuration files, which is scanned, categorized, and managed by the organizer.
_Avoid_: Repository, codebase, workspace, folder

**Standalone Project**:
A **Project** that sits directly under its **Category** directory because it does not share a prefix with any other projects in the scan.
_Avoid_: Single project, root project

**Grouped Project**:
A **Project** that is organized inside a **Group Folder** because it shares a common prefix and a recognized suffix with at least one other sibling project.
_Avoid_: Suffix project, microservice project

**Group Folder**:
A directory named after a shared prefix that groups multiple related **Grouped Projects** together.
_Avoid_: Group, project set, parent folder

**Lab Project**:
A **Project** with no recognizable markers, or one explicitly marked with a **Labs Marker**, which is classified under the `labs` category.
_Avoid_: Unmarked folder, experiment

**Category**:
One of the eight predefined, mutually exclusive project types (`apps`, `services`, `ui`, `embedded`, `ml`, `tools`, `labs`, `content`) that determines a project's physical parent folder inside the **Base Directory**.
_Avoid_: Classification, type, group, tag

**Stack Marker**:
A file or directory (such as `Cargo.toml`, `package.json`, `pubspec.yaml`, `build.gradle`, `pom.xml`, `go.mod`, `Gemfile`, `composer.json`, `mix.exs`, `*.csproj`, `*.sln`, `CMakeLists.txt`, or `.python-version`) whose presence identifies a project's framework or programming language stack.
_Avoid_: Configuration file, manifest, lockfile

**Labs Marker**:
A specific, empty file (`doc-lab.md`) placed at a project's root to manually override the stack-based classification and force the project into the `labs` category.
_Avoid_: Manual override, lab file, lab tag

**Base Directory**:
The root directory (defaulting to `~/projects`) that acts as the target storage where all organized **Projects** are moved and structured into their respective **Category** folders.
_Avoid_: Target directory, project root, home directory

**Project Suffix**:
A case-insensitive, known keyword (such as `api`, `web`, `backend`, or `cli`) at the end of a project directory's name, separated by a dash (`-`) or underscore (`_`), indicating the project's role. It is used to influence both **Category** classification and prefix-based grouping.
_Avoid_: Project extension, tag, suffix separator

**Fuzzy Jump**:
The fast command-line navigation action (triggered by the `pg` shell function) that allows the user to search, select, change the working directory (`cd`) to a **Project**, and open their preferred development editor in a single step.
_Avoid_: Navigation command, go command

**Editor Picker**:
An interactive CLI selection prompt presented to the user when launching a **Project** if multiple supported text editors or IDEs are detected on the system and no preference has been saved yet.
_Avoid_: Editor list, editor selection

**Project Preferences**:
Persistent local configuration stored in `~/.config/projm/prefs.json` that remembers metadata such as the user's last chosen text editor for each individual **Project**.
_Avoid_: User configuration, config file, project metadata

**Git Branch Indicator**:
The display in the **Fuzzy Jump** selector showing the active branch name of the **Project** if it is a Git repository.
_Avoid_: Branch name, git version

**Git Status Indicator**:
A visual status icon (`✓` for clean or `*` for dirty/uncommitted changes) next to the **Git Branch Indicator** in the navigator prompt.
_Avoid_: Repo status, changes tag

**Environment Diagnostics / Environment Check**:
A subcommand (`check`) that scans the local machine's `$PATH` to identify and verify the health, location, and versions of active compilers, runtimes, package managers, and system utilities.
_Avoid_: System check, dependency test, system scan

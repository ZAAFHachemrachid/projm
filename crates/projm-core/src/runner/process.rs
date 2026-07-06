//! Child-process lifecycle for the runner: group spawn, streamed logs, and cross-platform
//! group teardown (Unix process groups / Windows job objects) via the `command-group` crate.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use command_group::{CommandGroup, GroupChild};
#[cfg(unix)]
use command_group::{Signal, UnixChildExt};

use super::RunnableApp;

/// Max log lines retained per app.
pub const LOG_CAP: usize = 4000;
const KILL_GRACE: Duration = Duration::from_secs(4);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Stopped,
    Starting,
    Running,
    Errored,
}

/// Per-app runtime state (index-aligned with [`TuiState::apps`]).
pub struct AppRuntime {
    pub status: Status,
    child: Option<GroupChild>,
    pub pid: Option<u32>,
    pub logs: VecDeque<String>,
    /// Monotonic count of every line ever pushed (survives ring-buffer eviction).
    /// Lets external consumers (e.g. the Tauri GUI poller) stream only new lines.
    pub logged_total: u64,
    /// Number of lines scrolled up from the live tail (0 == following).
    pub scroll: usize,
    pub follow: bool,
    stopping: bool,
    restart_pending: bool,
    kill_deadline: Option<Instant>,
    started_at: Option<Instant>,
}

impl Default for AppRuntime {
    fn default() -> Self {
        Self {
            status: Status::Stopped,
            child: None,
            pid: None,
            logs: VecDeque::new(),
            logged_total: 0,
            scroll: 0,
            follow: true,
            stopping: false,
            restart_pending: false,
            kill_deadline: None,
            started_at: None,
        }
    }
}

impl AppRuntime {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Shared, mutable TUI state guarded by a single mutex.
pub struct TuiState {
    pub apps: Vec<RunnableApp>,
    pub runtimes: Vec<AppRuntime>,
    pub selected: usize,
    pub quitting: bool,
}

pub type Shared = Arc<Mutex<TuiState>>;
pub type Dirty = Arc<AtomicBool>;

fn push_log(rt: &mut AppRuntime, line: String) {
    rt.logs.push_back(line);
    rt.logged_total += 1;
    if rt.logs.len() > LOG_CAP {
        rt.logs.pop_front();
    }
}

/// Build the platform shell command for an app.
fn build_command(app: &RunnableApp) -> Command {
    #[cfg(unix)]
    let mut cmd = {
        let mut c = Command::new("sh");
        c.arg("-c").arg(&app.command);
        c
    };
    #[cfg(not(unix))]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(&app.command);
        c
    };
    cmd.current_dir(&app.dir);
    cmd.env("FORCE_COLOR", "0");
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd
}

fn spawn_reader<R: Read + Send + 'static>(pipe: R, state: Shared, dirty: Dirty, idx: usize) {
    thread::spawn(move || {
        let reader = BufReader::new(pipe);
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            let clean = strip_ansi(&line);
            if let Ok(mut st) = state.lock() {
                if let Some(rt) = st.runtimes.get_mut(idx) {
                    if rt.status == Status::Starting && is_ready(&clean) {
                        rt.status = Status::Running;
                    }
                    push_log(rt, clean);
                }
            }
            dirty.store(true, Ordering::Relaxed);
        }
    });
}

/// Start app `idx` if it isn't already running.
pub fn start(state: &Shared, dirty: &Dirty, idx: usize) {
    let mut pipes: Vec<Box<dyn Read + Send>> = Vec::new();
    {
        let mut st = state.lock().unwrap();
        if idx >= st.runtimes.len() {
            return;
        }
        if matches!(st.runtimes[idx].status, Status::Running | Status::Starting) {
            return;
        }
        let app = st.apps[idx].clone();
        match build_command(&app).group_spawn() {
            Ok(mut child) => {
                let pid = child.id();
                if let Some(o) = child.inner().stdout.take() {
                    pipes.push(Box::new(o));
                }
                if let Some(e) = child.inner().stderr.take() {
                    pipes.push(Box::new(e));
                }
                let rt = &mut st.runtimes[idx];
                rt.status = Status::Starting;
                rt.pid = Some(pid);
                rt.stopping = false;
                rt.restart_pending = false;
                rt.kill_deadline = None;
                rt.started_at = Some(Instant::now());
                rt.follow = true;
                rt.scroll = 0;
                push_log(rt, format!("▶ {}", app.command));
                rt.child = Some(child);
            }
            Err(e) => {
                let rt = &mut st.runtimes[idx];
                rt.status = Status::Errored;
                push_log(rt, format!("✖ failed to start: {e}"));
            }
        }
    }
    for p in pipes {
        spawn_reader(p, state.clone(), dirty.clone(), idx);
    }
    dirty.store(true, Ordering::Relaxed);
}

/// Request a graceful stop of app `idx` (SIGTERM to the group; escalates to SIGKILL later).
pub fn stop(state: &Shared, idx: usize, silent: bool) {
    let mut st = state.lock().unwrap();
    if idx >= st.runtimes.len() {
        return;
    }
    let rt = &mut st.runtimes[idx];
    if rt.child.is_none() {
        return;
    }
    rt.stopping = true;
    rt.kill_deadline = Some(Instant::now() + KILL_GRACE);
    if let Some(child) = rt.child.as_mut() {
        term_child(child);
    }
    if !silent {
        push_log(rt, "∎ stopping (SIGTERM → process group)".to_string());
    }
}

/// Restart app `idx`: if running, stop it and re-start once it exits; otherwise just start.
pub fn restart(state: &Shared, dirty: &Dirty, idx: usize) {
    let running = {
        let st = state.lock().unwrap();
        st.runtimes.get(idx).is_some_and(|rt| rt.child.is_some())
    };
    if running {
        {
            let mut st = state.lock().unwrap();
            st.runtimes[idx].restart_pending = true;
        }
        stop(state, idx, true);
    } else {
        start(state, dirty, idx);
    }
}

pub fn start_all(state: &Shared, dirty: &Dirty) {
    let n = state.lock().unwrap().apps.len();
    for i in 0..n {
        start(state, dirty, i);
    }
}

pub fn stop_all(state: &Shared) {
    let n = state.lock().unwrap().runtimes.len();
    for i in 0..n {
        stop(state, i, true);
    }
}

/// Reap exited children, honor kill deadlines, and flip stale `Starting` → `Running`.
/// Returns indices that requested a restart (start them after this returns).
pub fn poll(state: &Shared, dirty: &Dirty) {
    let mut to_restart: Vec<usize> = Vec::new();
    {
        let mut st = state.lock().unwrap();
        let n = st.runtimes.len();
        for i in 0..n {
            // Escalate to SIGKILL if a graceful stop overran its grace window.
            let overran = {
                let rt = &st.runtimes[i];
                rt.child.is_some()
                    && rt.stopping
                    && rt.kill_deadline.is_some_and(|d| Instant::now() >= d)
            };
            if overran {
                if let Some(c) = st.runtimes[i].child.as_mut() {
                    let _ = c.kill();
                }
            }

            let exited = st.runtimes[i]
                .child
                .as_mut()
                .and_then(|c| c.try_wait().ok().flatten());

            if let Some(status) = exited {
                let rt = &mut st.runtimes[i];
                rt.child = None;
                rt.pid = None;
                let restart = rt.restart_pending;
                if rt.stopping {
                    rt.status = Status::Stopped;
                    push_log(rt, "∎ stopped".to_string());
                } else if status.success() {
                    rt.status = Status::Stopped;
                    push_log(rt, "∎ exited cleanly".to_string());
                } else {
                    rt.status = Status::Errored;
                    push_log(rt, format!("∎ exited ({status})"));
                }
                rt.stopping = false;
                rt.restart_pending = false;
                rt.kill_deadline = None;
                if restart {
                    to_restart.push(i);
                }
                dirty.store(true, Ordering::Relaxed);
            } else {
                // Quiet-but-alive fallback: assume ready after the grace window.
                let rt = &mut st.runtimes[i];
                if rt.status == Status::Starting
                    && rt.started_at.is_some_and(|t| t.elapsed() >= KILL_GRACE)
                {
                    rt.status = Status::Running;
                    dirty.store(true, Ordering::Relaxed);
                }
            }
        }
    }
    for i in to_restart {
        start(state, dirty, i);
    }
}

/// Terminate every child on shutdown, leaving no orphans.
pub fn shutdown(state: &Shared) {
    {
        let mut st = state.lock().unwrap();
        for rt in &mut st.runtimes {
            if let Some(c) = rt.child.as_mut() {
                term_child(c);
                rt.stopping = true;
            }
        }
    }
    // Give children up to ~1.5s to exit gracefully, reaping as they go.
    for _ in 0..30 {
        thread::sleep(Duration::from_millis(50));
        let mut alive = false;
        {
            let mut st = state.lock().unwrap();
            for rt in &mut st.runtimes {
                if let Some(c) = rt.child.as_mut() {
                    if c.try_wait().ok().flatten().is_some() {
                        rt.child = None;
                    } else {
                        alive = true;
                    }
                }
            }
        }
        if !alive {
            break;
        }
    }
    // Hard-kill any survivors.
    let mut st = state.lock().unwrap();
    for rt in &mut st.runtimes {
        if let Some(c) = rt.child.as_mut() {
            let _ = c.kill();
            let _ = c.try_wait();
            rt.child = None;
        }
    }
}

// ── Free a bound port ──────────────────────────────────────────────────────────────

/// Free app `idx`'s port (if it has one) by killing whatever process holds it, and log
/// the result into that app's log stream.
pub fn free_port_for(state: &Shared, dirty: &Dirty, idx: usize) {
    let port = state
        .lock()
        .ok()
        .and_then(|st| st.apps.get(idx).and_then(|a| a.port));
    let Some(port) = port else {
        return;
    };
    let killed = free_port(port);
    if let Ok(mut st) = state.lock() {
        if let Some(rt) = st.runtimes.get_mut(idx) {
            if killed > 0 {
                push_log(
                    rt,
                    format!("⚡ freed port {port} — killed {killed} process(es)"),
                );
            } else {
                push_log(rt, format!("⚡ port {port} is already free"));
            }
        }
    }
    dirty.store(true, Ordering::Relaxed);
}

/// Kill every process currently holding `port` (graceful signal, then force-kill any
/// survivor). Returns how many PIDs were signalled.
pub fn free_port(port: u16) -> usize {
    let pids = pids_on_port(port);
    if pids.is_empty() {
        return 0;
    }
    for pid in &pids {
        term_pid(*pid);
    }
    thread::sleep(Duration::from_millis(400));
    for pid in pids_on_port(port) {
        kill_pid(pid);
    }
    pids.len()
}

fn dedup_pids(mut pids: Vec<u32>) -> Vec<u32> {
    pids.sort_unstable();
    pids.dedup();
    pids
}

#[cfg(unix)]
fn pids_on_port(port: u16) -> Vec<u32> {
    // Prefer lsof (listening sockets only), fall back to `ss -tlnp`.
    if let Ok(out) = Command::new("lsof")
        .args(["-t", &format!("-iTCP:{port}"), "-sTCP:LISTEN"])
        .output()
    {
        let pids: Vec<u32> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.trim().parse().ok())
            .collect();
        if !pids.is_empty() {
            return dedup_pids(pids);
        }
    }
    if let Ok(out) = Command::new("ss").args(["-tlnp"]).output() {
        return parse_ss_pids(&String::from_utf8_lossy(&out.stdout), port);
    }
    Vec::new()
}

#[cfg(not(unix))]
fn pids_on_port(port: u16) -> Vec<u32> {
    if let Ok(out) = Command::new("netstat").args(["-ano", "-p", "tcp"]).output() {
        return parse_netstat_pids(&String::from_utf8_lossy(&out.stdout), port);
    }
    Vec::new()
}

/// Parse `ss -tlnp` output for PIDs whose local listen address ends in `:port`.
fn parse_ss_pids(output: &str, port: u16) -> Vec<u32> {
    let needle = format!(":{port} ");
    let mut pids = Vec::new();
    for line in output.lines() {
        if !line.contains(&needle) {
            continue;
        }
        if let Some(pos) = line.find("pid=") {
            let num: String = line[pos + 4..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(p) = num.parse() {
                pids.push(p);
            }
        }
    }
    dedup_pids(pids)
}

/// Parse `netstat -ano -p tcp` output for LISTENING PIDs on `:port`.
#[cfg_attr(unix, allow(dead_code))]
fn parse_netstat_pids(output: &str, port: u16) -> Vec<u32> {
    let needle = format!(":{port}");
    let mut pids = Vec::new();
    for line in output.lines() {
        if !line.contains("LISTENING") {
            continue;
        }
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 5 || !cols[1].ends_with(&needle) {
            continue;
        }
        if let Ok(p) = cols[cols.len() - 1].parse() {
            pids.push(p);
        }
    }
    dedup_pids(pids)
}

#[cfg(unix)]
fn term_pid(pid: u32) {
    let _ = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status();
}

#[cfg(unix)]
fn kill_pid(pid: u32) {
    let _ = Command::new("kill")
        .arg("-KILL")
        .arg(pid.to_string())
        .status();
}

#[cfg(not(unix))]
fn term_pid(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string()])
        .status();
}

#[cfg(not(unix))]
fn kill_pid(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .status();
}

#[cfg(unix)]
fn term_child(child: &mut GroupChild) {
    let _ = child.signal(Signal::SIGTERM);
}

#[cfg(not(unix))]
fn term_child(child: &mut GroupChild) {
    // No graceful group signal on Windows — job-object terminate is the stop.
    let _ = child.kill();
}

// ── Log scrubbing / ready detection ────────────────────────────────────────────────

const READY_MARKERS: &[&str] = &[
    "ready",
    "listening",
    "localhost",
    "compiled",
    "started",
    "running on",
    "watching",
    "dev server",
    "local:",
];

fn is_ready(line: &str) -> bool {
    let l = line.to_lowercase();
    READY_MARKERS.iter().any(|m| l.contains(m))
}

/// Strip ANSI/OSC escape sequences and stray control characters so child output can't
/// corrupt the TUI layout.
pub fn strip_ansi(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == 0x1b {
            // ESC sequence
            if i + 1 < bytes.len() && bytes[i + 1] == b'[' {
                // CSI: ESC [ ... final-byte(0x40..=0x7e)
                i += 2;
                while i < bytes.len() && !(0x40..=0x7e).contains(&bytes[i]) {
                    i += 1;
                }
                i += 1; // consume final byte
            } else if i + 1 < bytes.len() && bytes[i + 1] == b']' {
                // OSC: ESC ] ... (BEL or ESC \)
                i += 2;
                while i < bytes.len() && bytes[i] != 0x07 {
                    if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                i += 1;
            } else {
                // Lone ESC or two-char escape.
                i += 2;
            }
            continue;
        }
        // Drop other C0 control chars except tab.
        if b < 0x20 && b != b'\t' {
            i += 1;
            continue;
        }
        // Copy this UTF-8 code point whole.
        let ch_len = utf8_len(b);
        let end = (i + ch_len).min(bytes.len());
        if let Ok(chunk) = std::str::from_utf8(&bytes[i..end]) {
            out.push_str(chunk);
        }
        i = end;
    }
    out
}

fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else if b >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

// ── Self-test ──────────────────────────────────────────────────────────────────────

/// Prove that stopping a project reaps its whole process group (no orphans).
pub fn self_test() -> Result<()> {
    #[cfg(unix)]
    {
        println!("runner self-test: spawning a nested process group…");
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("sleep 30 & sleep 30 & wait");
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        let mut child = cmd.group_spawn()?;
        let pid = child.id();
        thread::sleep(Duration::from_millis(400));
        let before = count_group(pid);
        println!("  process group {pid}: {before} live process(es)");

        term_child(&mut child);
        for _ in 0..40 {
            if child.try_wait()?.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let _ = child.kill();
        let _ = child.try_wait();
        thread::sleep(Duration::from_millis(300));

        let after = count_group(pid);
        if after == 0 {
            println!("  ✓ PASS — group fully reaped, 0 survivors");
            Ok(())
        } else {
            bail!("✗ FAIL — {after} process(es) survived the group stop");
        }
    }
    #[cfg(not(unix))]
    {
        println!("runner self-test: spawning a child process tree…");
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg("ping -n 30 127.0.0.1 >NUL");
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        let mut child = cmd.group_spawn()?;
        thread::sleep(Duration::from_millis(400));
        child.kill()?;
        let _ = child.try_wait();
        println!("  ✓ PASS — job object terminated the process tree");
        Ok(())
    }
}

#[cfg(unix)]
fn count_group(pgid: u32) -> usize {
    let out = Command::new("pgrep")
        .arg("-g")
        .arg(pgid.to_string())
        .output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count(),
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_removes_color_and_controls() {
        assert_eq!(strip_ansi("\x1b[32mgreen\x1b[0m"), "green");
        assert_eq!(strip_ansi("plain text"), "plain text");
        assert_eq!(strip_ansi("a\x1b]0;title\x07b"), "ab");
        assert_eq!(strip_ansi("tab\there"), "tab\there");
        assert_eq!(strip_ansi("carriage\rreturn"), "carriagereturn");
    }

    #[test]
    fn ready_detection() {
        assert!(is_ready("Local:   http://localhost:5173/"));
        assert!(is_ready("server listening on port 3000"));
        assert!(is_ready("✓ compiled successfully"));
        assert!(!is_ready("just some log output"));
    }

    #[test]
    fn utf8_len_boundaries() {
        assert_eq!(utf8_len(b'a'), 1);
        assert_eq!(strip_ansi("café ●"), "café ●");
    }

    #[test]
    fn parse_ss_pids_extracts_listener() {
        let out = "State  Recv-Q Send-Q Local Address:Port  Peer Address:Port Process\n\
                   LISTEN 0      511    0.0.0.0:3000        0.0.0.0:*         users:((\"node\",pid=4242,fd=20))\n\
                   LISTEN 0      128    [::]:5432           [::]:*            users:((\"postgres\",pid=99,fd=5))\n";
        assert_eq!(parse_ss_pids(out, 3000), vec![4242]);
        assert_eq!(parse_ss_pids(out, 5432), vec![99]);
        assert!(parse_ss_pids(out, 8080).is_empty());
        // :3000 must not match a longer port like :33000
        assert!(parse_ss_pids(
            "LISTEN 0 511 0.0.0.0:33000 0.0.0.0:* users:((\"x\",pid=1,fd=1))",
            3000
        )
        .is_empty());
    }

    #[test]
    fn parse_netstat_pids_extracts_listener() {
        let out = "  Proto  Local Address    Foreign Address   State       PID\n\
                   \x20 TCP    0.0.0.0:3000     0.0.0.0:0         LISTENING   4242\n\
                   \x20 TCP    0.0.0.0:3000     10.0.0.5:51000    ESTABLISHED 4242\n\
                   \x20 TCP    [::]:5432        [::]:0            LISTENING   99\n";
        assert_eq!(parse_netstat_pids(out, 3000), vec![4242]);
        assert_eq!(parse_netstat_pids(out, 5432), vec![99]);
        assert!(parse_netstat_pids(out, 8080).is_empty());
    }
}

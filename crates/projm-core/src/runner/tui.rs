//! Tabbed-log terminal UI: a top tab bar of apps + one full-width live log pane for the
//! focused tab. Painted with `console::Term` + raw ANSI (no extra TUI crate).

use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use console::{Key, Term};

use super::process::{self, AppRuntime, Dirty, Shared, Status, TuiState};
use super::RunnableApp;

const ALT_ON: &str = "\x1b[?1049h";
const ALT_OFF: &str = "\x1b[?1049l";
const RESET: &str = "\x1b[0m";
const INVERSE: &str = "\x1b[7m";
const DIM: &str = "\x1b[2m";

/// Run the interactive tabbed runner until the user quits.
pub fn run_tui(root: &Path, apps: Vec<RunnableApp>, start_all: bool) -> Result<()> {
    let repo = root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "repo".to_string());

    let runtimes = apps.iter().map(|_| AppRuntime::new()).collect();
    let state: Shared = Arc::new(Mutex::new(TuiState {
        apps,
        runtimes,
        selected: 0,
        quitting: false,
    }));
    let dirty: Dirty = Arc::new(AtomicBool::new(true));

    let term = Term::stdout();
    print!("{ALT_ON}");
    let _ = term.hide_cursor();
    let _ = io::stdout().flush();

    // Input thread: console `read_key` blocks, so it lives on its own thread.
    let input_state = state.clone();
    let input_dirty = dirty.clone();
    let input = thread::spawn(move || {
        let t = Term::stdout();
        loop {
            match t.read_key() {
                Ok(key) => {
                    if handle_key(&input_state, &input_dirty, key) {
                        break;
                    }
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(50));
                }
            }
            if input_state.lock().map(|st| st.quitting).unwrap_or(true) {
                break;
            }
        }
    });

    if start_all {
        process::start_all(&state, &dirty);
    }

    // Render / supervise loop.
    loop {
        if state.lock().map(|st| st.quitting).unwrap_or(true) {
            break;
        }
        process::poll(&state, &dirty);
        if dirty.swap(false, Ordering::Relaxed) {
            render(&state, &term, &repo);
        }
        thread::sleep(Duration::from_millis(80));
    }

    process::shutdown(&state);
    let _ = term.show_cursor();
    print!("{ALT_OFF}");
    let _ = io::stdout().flush();
    // The input thread exits after its own key handling set `quitting`; don't block on it.
    drop(input);
    Ok(())
}

/// Apply a keypress. Returns `true` when the user asked to quit.
fn handle_key(state: &Shared, dirty: &Dirty, key: Key) -> bool {
    let (sel, n) = {
        let st = state.lock().unwrap();
        (st.selected, st.apps.len())
    };
    if n == 0 {
        return true;
    }

    match key {
        Key::Char('q') | Key::CtrlC | Key::Escape => {
            state.lock().unwrap().quitting = true;
            return true;
        }
        Key::Tab | Key::ArrowRight | Key::Char('l') => {
            state.lock().unwrap().selected = (sel + 1) % n;
        }
        Key::BackTab | Key::ArrowLeft | Key::Char('h') => {
            state.lock().unwrap().selected = (sel + n - 1) % n;
        }
        Key::Char(c @ '1'..='9') => {
            let idx = (c as usize) - ('1' as usize);
            if idx < n {
                state.lock().unwrap().selected = idx;
            }
        }
        Key::Enter | Key::Char('s') => process::start(state, dirty, sel),
        Key::Char('x') => process::stop(state, sel, false),
        Key::Char('r') => process::restart(state, dirty, sel),
        Key::Char('a') => process::start_all(state, dirty),
        Key::Char('X') => process::stop_all(state),
        Key::Char('f') => process::free_port_for(state, dirty, sel),
        Key::PageUp | Key::ArrowUp | Key::Char('k') => scroll(state, sel, 1),
        Key::PageDown | Key::ArrowDown | Key::Char('j') => scroll(state, sel, -1),
        Key::Char('g') => {
            let mut st = state.lock().unwrap();
            let total = st.runtimes[sel].logs.len();
            let rt = &mut st.runtimes[sel];
            rt.follow = false;
            rt.scroll = total; // clamped at render time
        }
        Key::Char('G') => {
            let rt = &mut state.lock().unwrap().runtimes[sel];
            rt.follow = true;
            rt.scroll = 0;
        }
        _ => {}
    }
    dirty.store(true, Ordering::Relaxed);
    false
}

fn scroll(state: &Shared, sel: usize, delta: i32) {
    let mut st = state.lock().unwrap();
    let rt = &mut st.runtimes[sel];
    let step = 3i32;
    if delta > 0 {
        rt.follow = false;
        rt.scroll = rt.scroll.saturating_add((step * delta) as usize);
    } else {
        let dec = (step * -delta) as usize;
        rt.scroll = rt.scroll.saturating_sub(dec);
        if rt.scroll == 0 {
            rt.follow = true;
        }
    }
}

// ── Rendering ──────────────────────────────────────────────────────────────────────

fn render(state: &Shared, term: &Term, repo: &str) {
    let (rows, cols) = term.size();
    let (rows, cols) = (rows as usize, cols as usize);
    if rows < 4 || cols < 10 {
        return;
    }

    let st = state.lock().unwrap();
    let n = st.apps.len();
    let sel = st.selected.min(n.saturating_sub(1));
    let running = st
        .runtimes
        .iter()
        .filter(|r| matches!(r.status, Status::Running | Status::Starting))
        .count();

    let mut lines: Vec<String> = Vec::with_capacity(rows);

    // Header
    let header = format!(" {repo} ▸ dev control    {running}/{n} running");
    lines.push(format!("{INVERSE}{}{RESET}", fit(&header, cols)));

    // Tab bar
    lines.push(tab_bar(&st, sel, cols));

    // Pane title (command + optional port/url for the focused app)
    let app = &st.apps[sel];
    let rt = &st.runtimes[sel];
    let url = app
        .port
        .map(|p| format!("  http://localhost:{p}"))
        .unwrap_or_default();
    let title = format!(
        " {} {}·{} {}",
        status_dot(rt.status),
        app.label,
        app.command,
        url
    );
    lines.push(format!("{DIM}{}{RESET}", fit(&title, cols)));

    // Log pane
    let log_rows = rows.saturating_sub(4); // header, tabs, title, footer
    let total = rt.logs.len();
    let start = if rt.follow {
        total.saturating_sub(log_rows)
    } else {
        let scroll = rt.scroll.min(total.saturating_sub(1));
        total.saturating_sub(log_rows + scroll)
    };
    for k in 0..log_rows {
        match rt.logs.get(start + k) {
            Some(line) => lines.push(fit(line, cols)),
            None => lines.push(fit("", cols)),
        }
    }

    // Footer
    let mode = if rt.follow { "LIVE" } else { "PAUSED" };
    let footer = format!(
        " ↹ tab · ↵/s start · x stop · r restart · f free-port · a all · X stop-all · PgUp/PgDn scroll · g/G · q quit    [{mode}]"
    );
    lines.push(format!("{INVERSE}{}{RESET}", fit(&footer, cols)));

    drop(st);

    // Paint: home, each line + clear-to-EOL, then clear below.
    let mut frame = String::from("\x1b[H");
    for (i, line) in lines.iter().enumerate() {
        frame.push_str(line);
        frame.push_str("\x1b[K");
        if i + 1 < lines.len() {
            frame.push_str("\r\n");
        }
    }
    frame.push_str("\x1b[J");

    let mut out = io::stdout().lock();
    let _ = out.write_all(frame.as_bytes());
    let _ = out.flush();
}

fn tab_bar(st: &TuiState, sel: usize, cols: usize) -> String {
    let mut bar = String::new();
    let mut used = 0usize;
    for (i, app) in st.apps.iter().enumerate() {
        let rt = &st.runtimes[i];
        let dot = status_dot_char(rt.status);
        let port = app.port.map(|p| format!(":{p}")).unwrap_or_default();
        let plain = format!(" {dot} {}{port} ", app.label);
        let vis = plain.chars().count();
        if used + vis > cols {
            break;
        }
        let body = format!(
            " {}{}{RESET} {}{port} ",
            status_color(rt.status),
            dot,
            app.label
        );
        if i == sel {
            bar.push_str(&format!("{INVERSE}{body}{RESET}"));
        } else {
            bar.push_str(&body);
        }
        used += vis;
    }
    if used < cols {
        bar.push_str(&" ".repeat(cols - used));
    }
    bar
}

fn status_dot(status: Status) -> String {
    format!("{}{}{RESET}", status_color(status), status_dot_char(status))
}

fn status_dot_char(status: Status) -> char {
    match status {
        Status::Running => '●',
        Status::Starting => '◐',
        Status::Stopped => '○',
        Status::Errored => '✖',
    }
}

fn status_color(status: Status) -> &'static str {
    match status {
        Status::Running => "\x1b[32m",  // green
        Status::Starting => "\x1b[33m", // yellow
        Status::Stopped => "\x1b[90m",  // gray
        Status::Errored => "\x1b[31m",  // red
    }
}

/// Truncate or pad a plain (ANSI-free) string to exactly `w` display columns.
fn fit(s: &str, w: usize) -> String {
    let mut out = String::new();
    let mut count = 0usize;
    for ch in s.chars() {
        if count >= w {
            break;
        }
        out.push(ch);
        count += 1;
    }
    if count < w {
        out.push_str(&" ".repeat(w - count));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_truncates_and_pads() {
        assert_eq!(fit("hello", 3), "hel");
        assert_eq!(fit("hi", 5), "hi   ");
        assert_eq!(fit("", 2), "  ");
        assert_eq!(fit("exact", 5), "exact");
    }

    #[test]
    fn dot_chars_distinct() {
        assert_eq!(status_dot_char(Status::Running), '●');
        assert_eq!(status_dot_char(Status::Errored), '✖');
    }
}

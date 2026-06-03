use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use dialoguer::{Input, Select, theme::ColorfulTheme};

use crate::context::{ContextOutput, Label, ModuleContext, ProviderContext};
use crate::providers::local::LocalProvider;

const ORANGE: &str = "\x1b[38;2;249;115;22m";
const GRAY: &str = "\x1b[90m";
const WHITE: &str = "\x1b[97m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

// ── dashboard ────────────────────────────────────────────────────────────────

pub fn dashboard(root: &Path, ctx: ContextOutput) -> anyhow::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    let result = render_and_wait(&mut stdout, root, &ctx);

    let _ = execute!(stdout, cursor::Show, LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();

    result
}

fn render_and_wait(stdout: &mut impl Write, root: &Path, ctx: &ContextOutput) -> anyhow::Result<()> {
    // Count visible modules per visible provider for navigation.
    let visible_mods: Vec<usize> = ctx.providers.iter()
        .map(|p| p.modules.iter().filter(|m| !m.items.is_empty()).count())
        .filter(|&c| c > 0)
        .collect();
    let total_vp = visible_mods.len();

    let mut focus_p: usize = 0;
    let mut focus_m: usize = 0;
    let mut show_help = false;
    let mut sync_status: Option<(&'static str, &'static str)> = None; // (color, msg)

    macro_rules! redraw {
        ($stdout:expr) => {{
            execute!($stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
            render_main($stdout, root, ctx, focus_p, focus_m, sync_status)?;
            $stdout.flush()?;
        }};
    }

    redraw!(stdout);

    loop {
        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), KeyModifiers::NONE)
                    | (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,

                    (KeyCode::Char('?'), _) => {
                        show_help = !show_help;
                        if show_help { render_help_overlay(stdout)?; stdout.flush()?; }
                        else { redraw!(stdout); }
                    }
                    (KeyCode::Esc, _) if show_help => {
                        show_help = false;
                        redraw!(stdout);
                    }

                    (KeyCode::Char('j'), KeyModifiers::NONE) if !show_help && total_vp > 0 => {
                        focus_p = (focus_p + 1) % total_vp;
                        focus_m = focus_m.min(visible_mods[focus_p].saturating_sub(1));
                        redraw!(stdout);
                    }
                    (KeyCode::Char('k'), KeyModifiers::NONE) if !show_help && total_vp > 0 => {
                        focus_p = (focus_p + total_vp - 1) % total_vp;
                        focus_m = focus_m.min(visible_mods[focus_p].saturating_sub(1));
                        redraw!(stdout);
                    }
                    (KeyCode::Tab, KeyModifiers::NONE) if !show_help && total_vp > 0 => {
                        if focus_m + 1 < visible_mods[focus_p] { focus_m += 1; }
                        else { focus_p = (focus_p + 1) % total_vp; focus_m = 0; }
                        redraw!(stdout);
                    }
                    (KeyCode::BackTab, _) if !show_help && total_vp > 0 => {
                        if focus_m > 0 { focus_m -= 1; }
                        else { focus_p = (focus_p + total_vp - 1) % total_vp; focus_m = visible_mods[focus_p].saturating_sub(1); }
                        redraw!(stdout);
                    }

                    (KeyCode::Char('s'), KeyModifiers::CONTROL) if !show_help => {
                        sync_status = Some((GRAY, "syncing…"));
                        redraw!(stdout);
                        let exe = std::env::current_exe().unwrap_or_else(|_| "banco".into());
                        let ok = std::process::Command::new(&exe)
                            .arg("sync")
                            .current_dir(root)
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .status()
                            .map(|s| s.success())
                            .unwrap_or(false);
                        sync_status = Some(if ok { (GREEN, "sync ok") } else { (RED, "sync failed") });
                        redraw!(stdout);
                    }

                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn render_main(stdout: &mut impl Write, root: &Path, ctx: &ContextOutput, focus_p: usize, focus_m: usize, sync_status: Option<(&str, &str)>) -> anyhow::Result<()> {
    let project_name = root.file_name().and_then(|s| s.to_str()).unwrap_or("?");

    let sync = last_sync(root);
    let providers_str = enabled_providers(root);
    let (check_color, check_str) = check_status(root);
    let status_val = format!("{GRAY}{sync}  ·  {check_color}{check_str}{RESET}");

    // "  banco: " (9) + project_name + "  " (2) = inner display width
    let inner = 11 + project_name.chars().count();
    let top = "━".repeat(inner + 4);

    write!(stdout, "{ORANGE}{top}{RESET}    {GRAY}Status: {RESET}{status_val}\r\n")?;
    write!(stdout, "{ORANGE} ┃  banco{RESET}: {WHITE}{project_name}  {ORANGE}┃ {RESET}    {GRAY}Providers: {RESET}{WHITE}{providers_str}{RESET}\r\n")?;

    write!(stdout, "\r\n")?;

    let (term_width, term_height) = terminal::size().unwrap_or((80, 24));

    let mut vp_idx = 0usize;
    for provider in &ctx.providers {
        let visible: Vec<&ModuleContext> = provider.modules.iter()
            .filter(|m| !m.items.is_empty())
            .collect();
        if visible.is_empty() { continue; }
        let focused_col = if vp_idx == focus_p { Some(focus_m) } else { None };
        render_provider_section(stdout, provider, &visible, term_width as usize, focused_col)?;
        write!(stdout, "\r\n")?;
        vp_idx += 1;
    }

    let bottom_row = term_height.saturating_sub(1);
    execute!(stdout, cursor::MoveTo(0, bottom_row))?;
    let (fg, text) = match sync_status {
        Some((color, msg)) => (color, msg.to_string()),
        None => (WHITE, root.to_string_lossy().into_owned()),
    };
    let display_len = 1 + text.chars().count(); // leading space + text
    let padding = " ".repeat((term_width as usize).saturating_sub(display_len));
    write!(stdout, "\x1b[48;2;40;40;40m {fg}{text}{RESET}\x1b[48;2;40;40;40m{padding}{RESET}")?;

    Ok(())
}

fn render_help_overlay(stdout: &mut impl Write) -> anyhow::Result<()> {
    const SHORTCUTS: &[(&str, &str)] = &[
        ("?",          "toggle this panel"),
        ("Esc",        "close this panel"),
        ("j / k",      "next / prev provider"),
        ("Tab",        "next module"),
        ("Shift+Tab",  "prev module"),
        ("Ctrl+S",     "sync"),
        ("q",          "quit"),
        ("Ctrl+C",     "quit"),
    ];

    let key_w  = SHORTCUTS.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    let desc_w = SHORTCUTS.iter().map(|(_, d)| d.len()).max().unwrap_or(0);
    let inner  = key_w + 3 + desc_w;
    let panel_w = (inner + 4) as u16; // │·content·│
    let panel_h = (SHORTCUTS.len() + 2) as u16;

    let (tw, th) = terminal::size().unwrap_or((80, 24));
    let col = tw.saturating_sub(panel_w) / 2;
    let row = th.saturating_sub(panel_h) / 2;

    let title  = " shortcuts ";
    let dashes = (inner + 2).saturating_sub(title.len());

    execute!(stdout, cursor::MoveTo(col, row))?;
    write!(stdout, "{GRAY}┌{title}{}{RESET}", "─".repeat(dashes) + "┐")?;

    for (i, (key, desc)) in SHORTCUTS.iter().enumerate() {
        let end_pad = " ".repeat(desc_w - desc.len());
        execute!(stdout, cursor::MoveTo(col, row + 1 + i as u16))?;
        write!(stdout, "{GRAY}│ {WHITE}{key:<key_w$}{GRAY}   {RESET}{WHITE}{desc}{end_pad}{GRAY} │{RESET}")?;
    }

    execute!(stdout, cursor::MoveTo(col, row + panel_h - 1))?;
    write!(stdout, "{GRAY}└{}┘{RESET}", "─".repeat(inner + 2))?;

    Ok(())
}

// ── header helpers ───────────────────────────────────────────────────────────

fn relative_time(ts: &str) -> Option<String> {
    use chrono::{NaiveDateTime, Utc};
    let dt = NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S").ok()?;
    let diff = Utc::now().naive_utc().signed_duration_since(dt);
    let secs = diff.num_seconds();
    Some(if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", diff.num_minutes())
    } else if secs < 86400 {
        format!("{}h ago", diff.num_hours())
    } else if secs < 604800 {
        format!("{}d ago", diff.num_days())
    } else {
        format!("{}w ago", diff.num_weeks())
    })
}

fn last_sync(root: &Path) -> String {
    let dir = root.join(".banco/sync-state");
    if !dir.exists() {
        return "never".to_string();
    }
    let ts = std::fs::read_dir(&dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .max();
    match ts {
        None => "never".to_string(),
        Some(ts) => match relative_time(&ts) {
            Some(rel) => format!("{ts} ({rel})"),
            None => ts,
        },
    }
}

fn check_status(root: &Path) -> (&'static str, String) {
    let local = LocalProvider::new();
    let findings = match local.check(root) {
        Ok(f) => f,
        Err(_) => return (RED, "error".to_string()),
    };
    let config_issues = crate::config::load(root)
        .map(|cfg| {
            let mut issues = 0usize;
            for entry in &cfg.providers {
                let schema = match entry.name.as_str() {
                    "github" => crate::providers::github::GitHubProvider::available_config_schema(),
                    "gitlab" => crate::providers::gitlab::GitLabProvider::available_config_schema(),
                    "jira"   => crate::providers::jira::JiraProvider::available_config_schema(),
                    _        => continue,
                };
                let known: std::collections::HashSet<&str> = schema.iter().map(|p| p.name).collect();
                for key in entry.config.keys() {
                    if !known.contains(key.as_str()) { issues += 1; }
                }
                for param in &schema {
                    if param.required && !entry.config.contains_key(param.name) { issues += 1; }
                }
            }
            issues
        })
        .unwrap_or(0);

    let issue_count = findings.extraneous_dirs.len()
        + findings.extraneous_module_paths.len()
        + config_issues;

    if issue_count == 0 {
        (GREEN, "ok".to_string())
    } else {
        (RED, format!("{issue_count} issue{}", if issue_count == 1 { "" } else { "s" }))
    }
}

fn enabled_providers(root: &Path) -> String {
    let mut names = vec!["local".to_string()];
    if let Ok(cfg) = crate::config::load(root) {
        for p in &cfg.providers {
            if p.enabled && p.name != "local" {
                names.push(p.display_name().to_string());
            }
        }
    }
    names.join(", ")
}

// ── provider section rendering ───────────────────────────────────────────────

enum ColumnLine {
    Header(String),
    Group(String),
    Item(String),
}

fn display_name(raw: &str) -> &str {
    raw.split_once(" - ").map(|(_, rest)| rest).unwrap_or(raw)
}

fn fit(s: &str, width: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > width {
        let truncated: String = chars.iter().take(width.saturating_sub(1)).collect();
        format!("{truncated}…")
    } else {
        format!("{s:<width$}")
    }
}

fn module_column_lines(module: &ModuleContext) -> Vec<ColumnLine> {
    let mut lines = vec![
        ColumnLine::Header(format!("{} ({})", module.name, module.items.len())),
    ];

    if module.name == "tasks" {
        const STATUS_ORDER: &[&str] = &[
            "backlog", "to do", "open",
            "doing", "in progress", "in review",
            "done", "closed",
        ];

        let mut seen = std::collections::HashSet::new();
        let mut statuses: Vec<String> = module
            .items
            .iter()
            .filter_map(|i| i.get("status").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string()))
            .filter(|s| seen.insert(s.clone()))
            .collect();

        statuses.sort_by(|a, b| {
            let pa = STATUS_ORDER.iter().position(|&p| p.eq_ignore_ascii_case(a)).unwrap_or(STATUS_ORDER.len());
            let pb = STATUS_ORDER.iter().position(|&p| p.eq_ignore_ascii_case(b)).unwrap_or(STATUS_ORDER.len());
            pa.cmp(&pb).then_with(|| a.cmp(b))
        });

        for status in &statuses {
            let items: Vec<_> = module
                .items
                .iter()
                .filter(|i| i.get("status").and_then(|v| v.as_str()) == Some(status.as_str()))
                .collect();
            lines.push(ColumnLine::Group(format!("  {} ({})", status, items.len())));
            for item in items.iter().take(5) {
                if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                    let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let prefix = if id.is_empty() { String::new() } else { format!("{} ", id) };
                    lines.push(ColumnLine::Item(format!("    {}{}", prefix, display_name(name))));
                }
            }
        }
    } else {
        for item in module.items.iter().take(5) {
            if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                lines.push(ColumnLine::Item(format!("  {name}")));
            }
        }
    }

    lines
}

fn render_provider_section(
    stdout: &mut impl Write,
    provider: &ProviderContext,
    modules: &[&ModuleContext],
    term_width: usize,
    focused_col: Option<usize>,
) -> anyhow::Result<()> {
    let n = modules.len();

    // Generate full column content, then decide whether we have room to show it.
    // Minimum useful column width = 16: enough for group labels + a few chars of
    // item text. If any column has items and the window is too narrow, drop to
    // headers-only (counts still visible, just no item list).
    let mut columns: Vec<Vec<ColumnLine>> =
        modules.iter().map(|m| module_column_lines(m)).collect();
    let has_items = columns
        .iter()
        .any(|col| col.iter().any(|l| matches!(l, ColumnLine::Group(_) | ColumnLine::Item(_))));
    if has_items && term_width < 16 * n + (n + 1) {
        columns = columns
            .into_iter()
            .map(|col| col.into_iter().filter(|l| matches!(l, ColumnLine::Header(_))).collect())
            .collect();
    }

    let inner_total = term_width.saturating_sub(n + 1);
    let base_col = inner_total / n;
    let col_widths: Vec<usize> = (0..n)
        .map(|i| {
            if i == n - 1 {
                inner_total - base_col * (n - 1)
            } else {
                base_col
            }
        })
        .collect();

    let max_rows = columns.iter().map(|c| c.len()).max().unwrap_or(0);

    let border = if focused_col.is_some() { ORANGE } else { GRAY };

    // Top border: provider name embedded in first column's segment
    let name_str = format!(" {} ", provider.name);
    let first_border = {
        let w = col_widths[0];
        let name_len = name_str.chars().count();
        if name_len < w {
            format!("{}{}", name_str, "─".repeat(w - name_len))
        } else {
            "─".repeat(w)
        }
    };
    let top_parts: Vec<String> = std::iter::once(first_border)
        .chain(col_widths[1..].iter().map(|&w| "─".repeat(w)))
        .collect();
    write!(stdout, "{border}┌{}┐{RESET}\r\n", top_parts.join("┬"))?;

    // Content rows
    for row in 0..max_rows {
        write!(stdout, "{border}│{RESET}")?;
        for (ci, col) in columns.iter().enumerate() {
            let w = col_widths[ci];
            let (color, text) = match col.get(row) {
                None => (GRAY, String::new()),
                Some(ColumnLine::Header(s)) => {
                    let c = if focused_col == Some(ci) { ORANGE } else { WHITE };
                    (c, s.clone())
                }
                Some(ColumnLine::Group(s)) => (ORANGE, s.clone()),
                Some(ColumnLine::Item(s)) => (GRAY, s.clone()),
            };
            write!(stdout, "{color}{}{RESET}{border}│{RESET}", fit(&text, w))?;
        }
        write!(stdout, "\r\n")?;
    }

    // Bottom border
    let bot: Vec<String> = col_widths.iter().map(|&w| "─".repeat(w)).collect();
    write!(stdout, "{border}└{}┘{RESET}\r\n", bot.join("┴"))?;

    Ok(())
}

// ── interactive prompts ───────────────────────────────────────────────────────

pub fn prompt(labels: &[Label]) -> anyhow::Result<(String, HashMap<String, String>)> {
    let theme = ColorfulTheme::default();

    let name: String = Input::with_theme(&theme)
        .with_prompt("Name")
        .interact_text()?;

    let mut params = HashMap::new();
    for param in labels {
        match param.kind.as_str() {
            "string" => {
                let value: String = Input::with_theme(&theme)
                    .with_prompt(&param.name)
                    .allow_empty(true)
                    .interact_text()?;
                if !value.is_empty() {
                    params.insert(param.name.clone(), value);
                }
            }
            "enum" => {
                if let Some(values) = &param.values {
                    let idx = Select::with_theme(&theme)
                        .with_prompt(&param.name)
                        .items(values)
                        .default(0)
                        .interact()?;
                    params.insert(param.name.clone(), values[idx].clone());
                }
            }
            _ => {}
        }
    }

    Ok((name, params))
}

pub fn confirm_open(editor: &str) -> anyhow::Result<bool> {
    Ok(dialoguer::Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Open in {}?", editor))
        .default(true)
        .interact()?)
}

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute, queue,
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use dialoguer::{Input, Select, theme::ColorfulTheme};

use crate::context::{ContextOutput, Label, ModuleContext, ProviderContext};
use crate::providers::local::LocalProvider;

struct CheckDetail {
    config_issues: Vec<(String, Vec<String>)>, // (provider_display_name, [issue descriptions])
    extraneous_dirs: Vec<PathBuf>,
    extraneous_module_paths: Vec<PathBuf>,
}

fn gather_check_details(root: &Path) -> anyhow::Result<CheckDetail> {
    let local = LocalProvider::new();
    let findings = local.check(root)?;

    let mut config_issues: Vec<(String, Vec<String>)> = Vec::new();
    if let Ok(cfg) = crate::config::load(root) {
        for entry in &cfg.providers {
            let schema = match entry.name.as_str() {
                "github" => crate::providers::github::GitHubProvider::available_config_schema(),
                "gitlab" => crate::providers::gitlab::GitLabProvider::available_config_schema(),
                "jira"   => crate::providers::jira::JiraProvider::available_config_schema(),
                _        => continue,
            };
            let known: std::collections::HashSet<&str> = schema.iter().map(|p| p.name).collect();
            let mut issues = Vec::new();
            for key in entry.config.keys() {
                if !known.contains(key.as_str()) {
                    issues.push(format!("unknown key: {}", key));
                }
            }
            for param in &schema {
                if param.required && !entry.config.contains_key(param.name) {
                    issues.push(format!("missing required: {}", param.name));
                }
            }
            if !issues.is_empty() {
                config_issues.push((entry.display_name().to_string(), issues));
            }
        }
    }

    Ok(CheckDetail {
        config_issues,
        extraneous_dirs: findings.extraneous_dirs,
        extraneous_module_paths: findings.extraneous_module_paths,
    })
}

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

    // Build a flat index: (visible_provider_idx) -> (provider_idx, [visible_module_idxs])
    let vp_map: Vec<(usize, Vec<usize>)> = ctx.providers.iter()
        .enumerate()
        .filter_map(|(pi, p)| {
            let vis: Vec<usize> = p.modules.iter()
                .enumerate()
                .filter(|(_, m)| !m.items.is_empty())
                .map(|(mi, _)| mi)
                .collect();
            if vis.is_empty() { None } else { Some((pi, vis)) }
        })
        .collect();

    macro_rules! redraw {
        ($stdout:expr) => {{
            queue!($stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
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

                    (KeyCode::Char(' '), KeyModifiers::NONE) if !show_help && total_vp > 0 => {
                        let (pi, ref vis_mods) = vp_map[focus_p];
                        let mi = vis_mods[focus_m];
                        let provider = &ctx.providers[pi];
                        let module = &provider.modules[mi];
                        show_item_list(stdout, root, &provider.name, module)?;
                        redraw!(stdout);
                    }

                    (KeyCode::Char('d'), KeyModifiers::NONE) if !show_help => {
                        show_check_panel(stdout, root)?;
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

// ── item list overlay ────────────────────────────────────────────────────────

fn fuzzy_match(query: &str, target: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let q = query.to_lowercase();
    let t = target.to_lowercase();
    let mut qi = q.chars();
    let mut next = qi.next();
    for c in t.chars() {
        if Some(c) == next {
            next = qi.next();
        }
        if next.is_none() {
            return true;
        }
    }
    false
}

fn resolve_item_path(root: &Path, provider_name: &str, module_name: &str, item: &serde_json::Value) -> Option<PathBuf> {
    match (provider_name, module_name) {
        ("local", "notes") => {
            let name = item.get("name")?.as_str()?;
            let label = item.get("label").and_then(|v| v.as_str()).unwrap_or("");
            let base = root.join("notes/local");
            let dir = if label.is_empty() { base } else { base.join(label) };
            Some(dir.join(format!("{}.md", name)))
        }
        ("local", "tasks") => {
            let name = item.get("name")?.as_str()?;
            let status = item.get("status").and_then(|v| v.as_str()).unwrap_or("backlog");
            let dir = root.join("tasks/local").join(status);
            Some(dir.join(format!("{}.md", name)))
        }
        (_, "tasks") => {
            let name = item.get("name").and_then(|v| v.as_str())?;
            if let (Some(owner), Some(repo)) = (
                item.get("owner").and_then(|v| v.as_str()),
                item.get("repo").and_then(|v| v.as_str()),
            ) {
                // GitHub: tasks/{provider}/{owner}/{repo}/{stem}.md
                let dir = root.join("tasks").join(provider_name).join(owner).join(repo);
                Some(dir.join(format!("{}.md", name)))
            } else if let Some(project) = item.get("project").and_then(|v| v.as_str()) {
                // GitLab: tasks/{provider}/{project}/{stem}.md
                let dir = root.join("tasks").join(provider_name).join(project);
                Some(dir.join(format!("{}.md", name)))
            } else if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                // JIRA: tasks/{provider}/{id} - {name}.md
                let dir = root.join("tasks").join(provider_name);
                Some(dir.join(format!("{} - {}.md", id, name)))
            } else {
                None
            }
        }
        _ => None,
    }
}

struct ListRow {
    display: String,
    item_idx: Option<usize>,
    // 2 = project header, 1 = status/label header, 0 = selectable item
    level: u8,
}

fn item_project_key(item: &serde_json::Value) -> Option<String> {
    if let Some(p) = item.get("project").and_then(|v| v.as_str()) {
        if !p.is_empty() { return Some(p.to_string()); }
    }
    if let (Some(owner), Some(repo)) = (
        item.get("owner").and_then(|v| v.as_str()),
        item.get("repo").and_then(|v| v.as_str()),
    ) {
        if !owner.is_empty() && !repo.is_empty() {
            return Some(format!("{}/{}", owner, repo));
        }
    }
    None
}

fn sorted_statuses(items: impl Iterator<Item = String>) -> Vec<String> {
    const STATUS_ORDER: &[&str] = &[
        "backlog", "to do", "open",
        "doing", "in progress", "in review",
        "done", "closed",
    ];
    let mut seen = std::collections::HashSet::new();
    let mut statuses: Vec<String> = items
        .filter(|s| seen.insert(s.clone()))
        .collect();
    statuses.sort_by(|a, b| {
        let pa = STATUS_ORDER.iter().position(|&p| p.eq_ignore_ascii_case(a)).unwrap_or(STATUS_ORDER.len());
        let pb = STATUS_ORDER.iter().position(|&p| p.eq_ignore_ascii_case(b)).unwrap_or(STATUS_ORDER.len());
        pa.cmp(&pb).then_with(|| a.cmp(b))
    });
    statuses
}

fn build_list_rows(module: &ModuleContext) -> Vec<ListRow> {
    let mut rows = Vec::new();
    if module.name == "tasks" {
        let has_projects = module.items.iter().any(|i| item_project_key(i).is_some());

        if has_projects {
            let mut seen = std::collections::HashSet::new();
            let mut projects: Vec<String> = module.items.iter()
                .filter_map(item_project_key)
                .filter(|p| seen.insert(p.clone()))
                .collect();
            projects.sort();

            for project in &projects {
                rows.push(ListRow { display: project.clone(), item_idx: None, level: 2 });

                let statuses = sorted_statuses(
                    module.items.iter()
                        .filter(|i| item_project_key(i).as_deref() == Some(project.as_str()))
                        .filter_map(|i| i.get("status").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string()))
                );

                for status in &statuses {
                    rows.push(ListRow { display: format!("  {status}"), item_idx: None, level: 1 });
                    for (idx, item) in module.items.iter().enumerate() {
                        if item_project_key(item).as_deref() != Some(project.as_str()) { continue; }
                        if item.get("status").and_then(|v| v.as_str()) != Some(status.as_str()) { continue; }
                        if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let prefix = if id.is_empty() { String::new() } else { format!("{} ", id) };
                            rows.push(ListRow {
                                display: format!("    {}{}", prefix, display_name(name)),
                                item_idx: Some(idx),
                                level: 0,
                            });
                        }
                    }
                }
            }
        } else {
            let statuses = sorted_statuses(
                module.items.iter()
                    .filter_map(|i| i.get("status").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string()))
            );

            for status in &statuses {
                rows.push(ListRow { display: status.clone(), item_idx: None, level: 1 });
                for (idx, item) in module.items.iter().enumerate() {
                    if item.get("status").and_then(|v| v.as_str()) != Some(status.as_str()) {
                        continue;
                    }
                    if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let prefix = if id.is_empty() { String::new() } else { format!("{} ", id) };
                        rows.push(ListRow {
                            display: format!("{}{}", prefix, display_name(name)),
                            item_idx: Some(idx),
                            level: 0,
                        });
                    }
                }
            }
        }
    } else {
        let label_key = match module.name.as_str() {
            "notes" => Some("label"),
            _ => None,
        };
        if let Some(lk) = label_key {
            let mut seen = std::collections::HashSet::new();
            let mut labels: Vec<String> = module.items.iter()
                .filter_map(|i| i.get(lk).and_then(|v| v.as_str()).map(|s| s.to_string()))
                .filter(|s| seen.insert(s.clone()))
                .collect();
            labels.sort();

            let unlabeled: Vec<(usize, &serde_json::Value)> = module.items.iter()
                .enumerate()
                .filter(|(_, i)| i.get(lk).and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()))
                .collect();

            for (idx, item) in &unlabeled {
                if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                    rows.push(ListRow { display: name.to_string(), item_idx: Some(*idx), level: 0 });
                }
            }

            for label in &labels {
                if label.is_empty() { continue; }
                rows.push(ListRow { display: label.clone(), item_idx: None, level: 1 });
                for (idx, item) in module.items.iter().enumerate() {
                    if item.get(lk).and_then(|v| v.as_str()) != Some(label.as_str()) {
                        continue;
                    }
                    if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                        rows.push(ListRow {
                            display: name.to_string(),
                            item_idx: Some(idx),
                            level: 0,
                        });
                    }
                }
            }
        } else {
            for (idx, item) in module.items.iter().enumerate() {
                if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                    rows.push(ListRow { display: name.to_string(), item_idx: Some(idx), level: 0 });
                }
            }
        }
    }
    rows
}

fn show_item_list(stdout: &mut impl Write, root: &Path, provider_name: &str, module: &ModuleContext) -> anyhow::Result<()> {
    let can_edit = module.name == "notes" || module.name == "tasks";
    let all_rows = build_list_rows(module);

    let mut filter = String::new();
    let mut cursor_pos: usize = 0; // index into filtered selectable rows

    macro_rules! filtered_rows {
        () => {{
            let matched: Vec<&ListRow> = all_rows.iter().filter(|r| {
                r.item_idx.is_none() || fuzzy_match(&filter, &r.display)
            }).collect();
            // Drop header rows that have no item rows in their scope.
            // A header at level L owns rows until the next header at level >= L.
            let mut out: Vec<&ListRow> = Vec::new();
            for i in 0..matched.len() {
                if matched[i].item_idx.is_none() {
                    let level = matched[i].level;
                    let has_items = matched[i+1..].iter()
                        .take_while(|r| r.item_idx.is_some() || r.level < level)
                        .any(|r| r.item_idx.is_some());
                    if has_items { out.push(matched[i]); }
                } else {
                    out.push(matched[i]);
                }
            }
            out
        }};
    }

    macro_rules! selectable_indices {
        ($rows:expr) => {{
            $rows.iter().enumerate()
                .filter(|(_, r)| r.item_idx.is_some())
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        }};
    }

    let mut dirty = true;
    loop {
        if dirty {
            let rows = filtered_rows!();
            let sel_indices = selectable_indices!(rows);
            let n_sel = sel_indices.len();
            if cursor_pos >= n_sel && n_sel > 0 {
                cursor_pos = n_sel - 1;
            }
            queue!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
            render_item_list(stdout, provider_name, module, &rows, &sel_indices, cursor_pos, &filter, can_edit)?;
            stdout.flush()?;
            dirty = false;
        }

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                let rows = filtered_rows!();
                let sel_indices = selectable_indices!(rows);
                let n_sel = sel_indices.len();
                if cursor_pos >= n_sel && n_sel > 0 {
                    cursor_pos = n_sel - 1;
                }

                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::NONE) => break,

                    (KeyCode::Up, _) | (KeyCode::BackTab, _) if n_sel > 0 => {
                        cursor_pos = if cursor_pos == 0 { n_sel - 1 } else { cursor_pos - 1 };
                        dirty = true;
                    }
                    (KeyCode::Down, _) | (KeyCode::Tab, KeyModifiers::NONE) if n_sel > 0 => {
                        cursor_pos = (cursor_pos + 1) % n_sel;
                        dirty = true;
                    }

                    (KeyCode::Backspace, _) => {
                        filter.pop();
                        cursor_pos = 0;
                        dirty = true;
                    }
                    (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                        filter.push(c);
                        cursor_pos = 0;
                        dirty = true;
                    }

                    (KeyCode::Enter, _) if n_sel > 0 => {
                        let row_idx = sel_indices[cursor_pos];
                        let row = rows[row_idx];
                        if let Some(item_idx) = row.item_idx {
                            let item = &module.items[item_idx];
                            if let Some(path) = resolve_item_path(root, provider_name, &module.name, item) {
                                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
                                execute!(stdout, cursor::Show, LeaveAlternateScreen)?;
                                terminal::disable_raw_mode()?;

                                let _ = std::process::Command::new(&editor)
                                    .arg(&path)
                                    .status();

                                terminal::enable_raw_mode()?;
                                execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
                                dirty = true;
                            }
                        }
                    }

                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn render_item_list(
    stdout: &mut impl Write,
    provider_name: &str,
    module: &ModuleContext,
    rows: &[&ListRow],
    sel_indices: &[usize],
    cursor_pos: usize,
    filter: &str,
    can_edit: bool,
) -> anyhow::Result<()> {
    let (term_width, term_height) = terminal::size().unwrap_or((80, 24));
    let w = term_width as usize;

    let selected_row = sel_indices.get(cursor_pos).copied();

    let title = format!(" {}: {} ({} items) ", provider_name, module.name, module.items.len());
    let title_len = title.chars().count();
    let bar = "─".repeat(w.saturating_sub(title_len + 2));
    write!(stdout, "{ORANGE}┌{title}{bar}┐{RESET}\r\n")?;

    // Filter bar
    let filter_display = format!(" filter: {}_", filter);
    let fpad = w.saturating_sub(filter_display.chars().count() + 2);
    write!(stdout, "{ORANGE}│{RESET}{WHITE}{}{}{ORANGE}│{RESET}\r\n",
        filter_display, " ".repeat(fpad))?;
    write!(stdout, "{ORANGE}├{}┤{RESET}\r\n", "─".repeat(w.saturating_sub(2)))?;

    let list_height = (term_height as usize).saturating_sub(6); // header(3) + footer(2) + bottom border(1)

    // Scroll so selected item is visible
    let scroll_offset = if let Some(si) = selected_row {
        let n_sel = sel_indices.len();
        if n_sel == 0 {
            0
        } else {
            let mid = list_height / 2;
            if cursor_pos > mid {
                let max_scroll = rows.len().saturating_sub(list_height);
                (si.saturating_sub(mid)).min(max_scroll)
            } else {
                0
            }
        }
    } else {
        0
    };

    let visible_rows = rows.iter().enumerate().skip(scroll_offset).take(list_height);
    let mut shown = 0usize;

    for (ri, row) in visible_rows {
        let is_selected = Some(ri) == selected_row;
        let inner_w = w.saturating_sub(4); // │·content·│

        if row.item_idx.is_none() {
            // Label/group header — always visible; project headers (level 2) get accent color
            let text = fit(&format!("  {}", row.display), inner_w);
            let color = if row.level >= 2 { ORANGE } else { WHITE };
            write!(stdout, "{ORANGE}│ {color}{text} {ORANGE}│{RESET}\r\n")?;
        } else if is_selected {
            let text = fit(&format!("  {}", row.display), inner_w);
            write!(stdout, "{ORANGE}│{RESET}\x1b[48;2;60;40;10m{ORANGE}{text} {RESET}{ORANGE}│{RESET}\r\n")?;
        } else {
            let text = fit(&format!("  {}", row.display), inner_w);
            write!(stdout, "{ORANGE}│ {GRAY}{text}{RESET} {ORANGE}│{RESET}\r\n")?;
        }
        shown += 1;
    }

    for _ in shown..list_height {
        write!(stdout, "{ORANGE}│{}│{RESET}\r\n", " ".repeat(w.saturating_sub(2)))?;
    }

    write!(stdout, "{ORANGE}└{}┘{RESET}\r\n", "─".repeat(w.saturating_sub(2)))?;

    // Footer hints
    let edit_hint = if can_edit { "  Enter edit" } else { "" };
    let hint = format!("  Esc/q close  ↑↓/Tab navigate  type to filter{}", edit_hint);
    let hpad = w.saturating_sub(hint.chars().count());
    execute!(stdout, cursor::MoveTo(0, term_height - 1))?;
    write!(stdout, "\x1b[48;2;40;40;40m{GRAY}{}{}{RESET}", hint, " ".repeat(hpad))?;

    Ok(())
}

// ── main dashboard rendering ─────────────────────────────────────────────────

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

    // Collect visible providers up front so the vertical budget can be split
    // across them. Each section costs 2 border rows + 1 blank separator row on
    // top of its content rows; the header above (3 rows) and the bottom status
    // bar (1 row) must always remain on screen.
    let sections: Vec<(&ProviderContext, Vec<&ModuleContext>)> = ctx.providers.iter()
        .map(|p| (p, p.modules.iter().filter(|m| !m.items.is_empty()).collect::<Vec<_>>()))
        .filter(|(_, v)| !v.is_empty())
        .collect();

    const HEADER_ROWS: usize = 3; // two header lines + one blank
    const STATUS_ROWS: usize = 1; // bottom status bar
    const SECTION_CHROME: usize = 3; // top border + bottom border + separator
    let mut avail = (term_height as usize)
        .saturating_sub(HEADER_ROWS + STATUS_ROWS)
        .saturating_sub(sections.len() * SECTION_CHROME);

    // Each section's natural height is its tallest column. Distribute the
    // available content rows by water-filling: sections that need fewer rows
    // than their fair share take only what they need and donate the rest to
    // the remaining sections, so a small panel never starves a large one.
    let natural: Vec<usize> = sections.iter()
        .map(|(_, mods)| mods.iter().map(|m| module_column_lines(m).len()).max().unwrap_or(0))
        .collect();
    let mut budgets = vec![0usize; sections.len()];
    let mut remaining: Vec<usize> = (0..sections.len()).collect();
    while !remaining.is_empty() && avail > 0 {
        let share = (avail / remaining.len()).max(1);
        let mut next = Vec::new();
        let mut progressed = false;
        for &i in &remaining {
            let want = natural[i].saturating_sub(budgets[i]);
            let give = want.min(share).min(avail);
            budgets[i] += give;
            avail -= give;
            if give > 0 { progressed = true; }
            if budgets[i] < natural[i] { next.push(i); }
        }
        // No section could grow this pass (avail < remaining count): stop.
        if !progressed { break; }
        remaining = next;
    }

    let mut vp_idx = 0usize;
    for (i, (provider, visible)) in sections.iter().enumerate() {
        let focused_col = if vp_idx == focus_p { Some(focus_m) } else { None };
        let budget = budgets[i].max(1); // always room for at least the header
        render_provider_section(stdout, provider, visible, term_width as usize, budget, focused_col)?;
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
        ("Space",      "browse module items"),
        ("d",          "check panel"),
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

// ── check panel ─────────────────────────────────────────────────────────────

fn show_check_panel(stdout: &mut impl Write, root: &Path) -> anyhow::Result<()> {
    let detail = gather_check_details(root).unwrap_or_else(|_| CheckDetail {
        config_issues: vec![("error".to_string(), vec!["failed to gather check details".to_string()])],
        extraneous_dirs: vec![],
        extraneous_module_paths: vec![],
    });

    render_check_panel(stdout, root, &detail)?;
    stdout.flush()?;

    loop {
        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::NONE) => break,
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn render_check_panel(stdout: &mut impl Write, root: &Path, detail: &CheckDetail) -> anyhow::Result<()> {
    let (tw, th) = terminal::size().unwrap_or((80, 24));

    let total_issues = detail.config_issues.iter().map(|(_, v)| v.len()).sum::<usize>()
        + detail.extraneous_dirs.len()
        + detail.extraneous_module_paths.len();

    // Build content lines: (color, text)
    let mut content: Vec<(&'static str, String)> = Vec::new();

    if total_issues == 0 {
        content.push((GREEN, "  ✓  No issues found".to_string()));
    } else {
        content.push((WHITE, format!("  {} issue{} found:", total_issues, if total_issues == 1 { "" } else { "s" })));

        if !detail.config_issues.is_empty() {
            content.push((WHITE, String::new()));
            content.push((ORANGE, "  Config".to_string()));
            for (provider, issues) in &detail.config_issues {
                content.push((WHITE, format!("    {}", provider)));
                for issue in issues {
                    content.push((RED, format!("      ✗  {}", issue)));
                }
            }
        }

        if !detail.extraneous_dirs.is_empty() {
            content.push((WHITE, String::new()));
            content.push((ORANGE, "  Extraneous directories".to_string()));
            for path in &detail.extraneous_dirs {
                let display = path.strip_prefix(root)
                    .map(|r| format!("./{}", r.display()))
                    .unwrap_or_else(|_| path.display().to_string());
                content.push((RED, format!("    ✗  {}", display)));
            }
        }

        if !detail.extraneous_module_paths.is_empty() {
            content.push((WHITE, String::new()));
            content.push((ORANGE, "  Extraneous module paths".to_string()));
            for path in &detail.extraneous_module_paths {
                let display = path.strip_prefix(root)
                    .map(|r| format!("./{}", r.display()))
                    .unwrap_or_else(|_| path.display().to_string());
                content.push((RED, format!("    ✗  {}", display)));
            }
        }
    }

    const HINT: &str = "Esc  close";

    // inner_w: widest content line or hint, capped to leave a margin
    let max_line = content.iter().map(|(_, s)| s.chars().count()).max().unwrap_or(0);
    let inner_w = max_line.max(HINT.len() + 2).min(tw as usize - 4);

    // rows: top + blank + content... + blank + hint + bottom
    let panel_h = (content.len() + 4) as u16;
    let panel_w = (inner_w + 4) as u16;

    let col = tw.saturating_sub(panel_w) / 2;
    let row = th.saturating_sub(panel_h) / 2;

    let title = " check ";
    let top_dashes = (inner_w + 2).saturating_sub(title.len());

    // top border
    execute!(stdout, cursor::MoveTo(col, row))?;
    write!(stdout, "{ORANGE}┌{title}{}┐{RESET}", "─".repeat(top_dashes))?;

    // blank
    execute!(stdout, cursor::MoveTo(col, row + 1))?;
    write!(stdout, "{ORANGE}│{}│{RESET}", " ".repeat(inner_w + 2))?;

    // content lines
    for (i, (color, text)) in content.iter().enumerate() {
        execute!(stdout, cursor::MoveTo(col, row + 2 + i as u16))?;
        let fitted = fit(text, inner_w);
        write!(stdout, "{ORANGE}│ {color}{fitted}{RESET} {ORANGE}│{RESET}")?;
    }

    // blank before hint
    let hint_row = row + 2 + content.len() as u16;
    execute!(stdout, cursor::MoveTo(col, hint_row))?;
    write!(stdout, "{ORANGE}│{}│{RESET}", " ".repeat(inner_w + 2))?;

    // hint (right-aligned)
    execute!(stdout, cursor::MoveTo(col, hint_row + 1))?;
    let pad = inner_w.saturating_sub(HINT.len());
    write!(stdout, "{ORANGE}│ {GRAY}{}{}{RESET} {ORANGE}│{RESET}", " ".repeat(pad), HINT)?;

    // bottom border
    execute!(stdout, cursor::MoveTo(col, hint_row + 2))?;
    write!(stdout, "{ORANGE}└{}┘{RESET}", "─".repeat(inner_w + 2))?;

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
    let detail = match gather_check_details(root) {
        Ok(d) => d,
        Err(_) => return (RED, "error".to_string()),
    };
    let issue_count = detail.config_issues.iter().map(|(_, v)| v.len()).sum::<usize>()
        + detail.extraneous_dirs.len()
        + detail.extraneous_module_paths.len();
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

#[derive(Clone)]
enum ColumnLine {
    Header(String),
    Group(String),
    Item(String),
    More(String),
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
        let has_projects = module.items.iter().any(|i| item_project_key(i).is_some());

        if has_projects {
            let mut seen = std::collections::HashSet::new();
            let mut projects: Vec<String> = module.items.iter()
                .filter_map(item_project_key)
                .filter(|p| seen.insert(p.clone()))
                .collect();
            projects.sort();

            for project in &projects {
                let project_items: Vec<_> = module.items.iter()
                    .filter(|i| item_project_key(i).as_deref() == Some(project.as_str()))
                    .collect();
                lines.push(ColumnLine::Group(format!("  {} ({})", project, project_items.len())));

                let statuses = sorted_statuses(
                    project_items.iter()
                        .filter_map(|i| i.get("status").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string()))
                );

                for status in &statuses {
                    let status_items: Vec<_> = project_items.iter()
                        .filter(|i| i.get("status").and_then(|v| v.as_str()) == Some(status.as_str()))
                        .collect();
                    lines.push(ColumnLine::Group(format!("    {} ({})", status, status_items.len())));
                    for item in &status_items {
                        if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let prefix = if id.is_empty() { String::new() } else { format!("{} ", id) };
                            lines.push(ColumnLine::Item(format!("      {}{}", prefix, display_name(name))));
                        }
                    }
                }
            }
        } else {
            let statuses = sorted_statuses(
                module.items.iter()
                    .filter_map(|i| i.get("status").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string()))
            );

            for status in &statuses {
                let items: Vec<_> = module
                    .items
                    .iter()
                    .filter(|i| i.get("status").and_then(|v| v.as_str()) == Some(status.as_str()))
                    .collect();
                lines.push(ColumnLine::Group(format!("  {} ({})", status, items.len())));
                for item in &items {
                    if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let prefix = if id.is_empty() { String::new() } else { format!("{} ", id) };
                        lines.push(ColumnLine::Item(format!("    {}{}", prefix, display_name(name))));
                    }
                }
            }
        }
    } else {
        for item in &module.items {
            if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                lines.push(ColumnLine::Item(format!("  {name}")));
            }
        }
    }

    lines
}

/// Cap a single column's content to `max_rows` lines. When the column would
/// overflow, the surplus item lines are dropped and replaced with a single
/// `+N more <noun>` summary line so the count stays visible. The header line
/// is always preserved. `noun` is the plural item label (e.g. "tasks").
fn truncate_column(lines: Vec<ColumnLine>, max_rows: usize, noun: &str) -> Vec<ColumnLine> {
    if lines.len() <= max_rows {
        return lines;
    }
    let item_total = lines.iter().filter(|l| matches!(l, ColumnLine::Item(_))).count();

    // The header (first line) is always kept; the last visible row is reserved
    // for the "+N more" summary. Whatever rows remain in between hold content.
    let header = lines.first().cloned();
    let body_budget = max_rows.saturating_sub(if header.is_some() { 2 } else { 1 });

    let mut out: Vec<ColumnLine> = Vec::with_capacity(max_rows);
    let mut items_shown = 0usize;
    if let Some(h) = header {
        out.push(h);
    }
    for line in lines.into_iter().skip(1).take(body_budget) {
        if matches!(line, ColumnLine::Item(_)) {
            items_shown += 1;
        }
        out.push(line);
    }
    let hidden = item_total.saturating_sub(items_shown);
    out.push(ColumnLine::More(format!("  +{hidden} more {noun}")));
    out
}

fn render_provider_section(
    stdout: &mut impl Write,
    provider: &ProviderContext,
    modules: &[&ModuleContext],
    term_width: usize,
    content_budget: usize,
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

    // Cap each column to the vertical budget so the header and status bar stay
    // on screen. Overflow collapses into a "+N more <noun>" summary line.
    columns = columns
        .iter()
        .zip(modules.iter())
        .map(|(col, m)| truncate_column(col.clone(), content_budget.max(1), &m.name))
        .collect();

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
                Some(ColumnLine::Group(s)) => (WHITE, s.clone()),
                Some(ColumnLine::Item(s)) => (GRAY, s.clone()),
                Some(ColumnLine::More(s)) => (GRAY, s.clone()),
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

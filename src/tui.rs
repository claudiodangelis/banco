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

use crate::config::{self, BrowseConfig, ProjectConfig, ProviderEntry};
use crate::context::{ContextOutput, Label, ModuleContext, ProviderContext};
use crate::provider::{ConfigParam, ConfigParamKind};
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

    let result = render_and_wait(&mut stdout, root, ctx);

    let _ = execute!(stdout, cursor::Show, LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();

    result
}

// Navigation indices derived from the context. Recomputed whenever the
// underlying data changes (e.g. after a sync).
struct NavIndex {
    // Count of visible modules per visible provider.
    visible_mods: Vec<usize>,
    total_vp: usize,
    // Flat index: visible_provider_idx -> (provider_idx, [visible_module_idxs]).
    vp_map: Vec<(usize, Vec<usize>)>,
}

fn build_nav_index(ctx: &ContextOutput) -> NavIndex {
    let visible_mods: Vec<usize> = ctx.providers.iter()
        .map(|p| p.modules.iter().filter(|m| !m.items.is_empty()).count())
        .filter(|&c| c > 0)
        .collect();
    let total_vp = visible_mods.len();

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

    NavIndex { visible_mods, total_vp, vp_map }
}

fn render_and_wait(stdout: &mut impl Write, root: &Path, mut ctx: ContextOutput) -> anyhow::Result<()> {
    let NavIndex { mut visible_mods, mut total_vp, mut vp_map } = build_nav_index(&ctx);

    let mut focus_p: usize = 0;
    let mut focus_m: usize = 0;
    let mut show_help = false;
    let mut view = crate::state::load(root);
    let mut sync_status: Option<(&'static str, &'static str)> = None; // (color, msg)

    macro_rules! redraw {
        ($stdout:expr) => {{
            queue!($stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
            render_main($stdout, root, &ctx, focus_p, focus_m, &view, sync_status)?;
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

                    (KeyCode::Char('c'), KeyModifiers::NONE) if !show_help => {
                        let changed = show_config_editor(stdout, root)?;
                        // Editing config can enable/disable providers or modules,
                        // which changes what the dashboard shows — rebuild as we
                        // do after a sync.
                        if changed {
                            if let Ok(fresh) = crate::build_context(root) {
                                ctx = fresh;
                                let nav = build_nav_index(&ctx);
                                visible_mods = nav.visible_mods;
                                total_vp = nav.total_vp;
                                vp_map = nav.vp_map;
                                focus_p = focus_p.min(total_vp.saturating_sub(1));
                                let mods_here = visible_mods.get(focus_p).copied().unwrap_or(0);
                                focus_m = focus_m.min(mods_here.saturating_sub(1));
                            }
                        }
                        redraw!(stdout);
                    }

                    (KeyCode::Char('v'), KeyModifiers::NONE) if !show_help && total_vp > 0 => {
                        let (pi, _) = vp_map[focus_p];
                        view.toggle_collapsed(&ctx.providers[pi].name);
                        crate::state::save(root, &view);
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
                        // Sync mutates files on disk; rebuild the context and
                        // navigation indices so the dashboard reflects the new items.
                        if ok {
                            if let Ok(fresh) = crate::build_context(root) {
                                ctx = fresh;
                                let nav = build_nav_index(&ctx);
                                visible_mods = nav.visible_mods;
                                total_vp = nav.total_vp;
                                vp_map = nav.vp_map;
                                // Keep focus within the (possibly changed) bounds.
                                focus_p = focus_p.min(total_vp.saturating_sub(1));
                                let mods_here = visible_mods.get(focus_p).copied().unwrap_or(0);
                                focus_m = focus_m.min(mods_here.saturating_sub(1));
                            }
                        }
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

fn render_main(stdout: &mut impl Write, root: &Path, ctx: &ContextOutput, focus_p: usize, focus_m: usize, view: &crate::state::ProjectState, sync_status: Option<(&str, &str)>) -> anyhow::Result<()> {
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
    // bar (1 row) must always remain on screen. A provider collapsed with `v`
    // renders as a single summary bar instead and is excluded from the budget.
    let sections: Vec<(&ProviderContext, Vec<&ModuleContext>)> = ctx.providers.iter()
        .map(|p| (p, p.modules.iter().filter(|m| !m.items.is_empty()).collect::<Vec<_>>()))
        .filter(|(_, v)| !v.is_empty())
        .collect();
    let collapsed: Vec<bool> = sections.iter().map(|(p, _)| view.is_collapsed(&p.name)).collect();

    const HEADER_ROWS: usize = 3; // two header lines + one blank
    const STATUS_ROWS: usize = 1; // bottom status bar
    const SECTION_CHROME: usize = 3; // top border + bottom border + separator
    const COLLAPSED_ROWS: usize = 2; // summary bar + separator
    let collapsed_cost: usize = collapsed.iter().filter(|&&c| c).count() * COLLAPSED_ROWS;
    let expanded_count = collapsed.iter().filter(|&&c| !c).count();
    let mut avail = (term_height as usize)
        .saturating_sub(HEADER_ROWS + STATUS_ROWS)
        .saturating_sub(collapsed_cost)
        .saturating_sub(expanded_count * SECTION_CHROME);

    // Each expanded section's natural height is its tallest column. Distribute the
    // available content rows by water-filling: sections that need fewer rows
    // than their fair share take only what they need and donate the rest to
    // the remaining sections, so a small panel never starves a large one.
    // Collapsed sections need no content rows.
    let natural: Vec<usize> = sections.iter().enumerate()
        .map(|(i, (_, mods))| {
            if collapsed[i] { 0 } else { mods.iter().map(|m| module_column_lines(m).len()).max().unwrap_or(0) }
        })
        .collect();
    let mut budgets = vec![0usize; sections.len()];
    let mut remaining: Vec<usize> = (0..sections.len()).filter(|&i| !collapsed[i]).collect();
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
        let focused = vp_idx == focus_p;
        if collapsed[i] {
            render_collapsed_section(stdout, provider, visible, term_width as usize, focused)?;
        } else {
            let focused_col = if focused { Some(focus_m) } else { None };
            let budget = budgets[i].max(1); // always room for at least the header
            render_provider_section(stdout, provider, visible, term_width as usize, budget, focused_col)?;
        }
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
        ("c",          "edit config"),
        ("d",          "check panel"),
        ("Esc",        "close this panel"),
        ("j / k",      "next / prev provider"),
        ("q",          "quit"),
        ("v",          "collapse / expand provider"),
        ("Ctrl+C",     "quit"),
        ("Ctrl+S",     "sync"),
        ("Tab",        "next module"),
        ("Shift+Tab",  "prev module"),
        ("Space",      "browse module items"),
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

// ── config editor ─────────────────────────────────────────────────────────────

const HILITE: &str = "\x1b[48;2;60;40;10m"; // selected-row background

/// Config schema for a provider name, or empty for unknown/local.
fn provider_schema(name: &str) -> Vec<ConfigParam> {
    use crate::providers::{github::GitHubProvider, gitlab::GitLabProvider, jira::JiraProvider};
    match name {
        "github" => GitHubProvider::available_config_schema(),
        "gitlab" => GitLabProvider::available_config_schema(),
        "jira"   => JiraProvider::available_config_schema(),
        _        => LocalProvider::available_config_schema(),
    }
}

/// Modules a provider implements (used for the per-module on/off toggles).
fn provider_modules(name: &str) -> Vec<&'static str> {
    use crate::providers::{github::GitHubProvider, gitlab::GitLabProvider, jira::JiraProvider};
    match name {
        "local"  => LocalProvider::available_modules(),
        "github" => GitHubProvider::available_modules(),
        "gitlab" => GitLabProvider::available_modules(),
        "jira"   => JiraProvider::available_modules(),
        _        => vec![],
    }
}

/// In-memory validation of a single entry, mirroring `check_config` in main.rs:
/// unknown/missing parameters and unknown disabled modules.
fn entry_issues(entry: &ProviderEntry) -> Vec<String> {
    let mut issues = Vec::new();
    let modules = provider_modules(&entry.name);
    if modules.is_empty() {
        issues.push(format!("unknown provider '{}'", entry.name));
        return issues;
    }
    let known_modules: std::collections::HashSet<&str> = modules.iter().copied().collect();
    for m in &entry.disabled_modules {
        if !known_modules.contains(m.as_str()) {
            issues.push(format!("unknown module '{m}' in disabled_modules"));
        }
    }
    let schema = provider_schema(&entry.name);
    let known: std::collections::HashSet<&str> = schema.iter().map(|p| p.name).collect();
    for key in entry.config.keys() {
        if !known.contains(key.as_str()) {
            issues.push(format!("unknown parameter '{key}'"));
        }
    }
    for param in &schema {
        if param.required && !entry.config.contains_key(param.name) {
            issues.push(format!("missing required parameter '{}'", param.name));
        }
    }
    issues.sort();
    issues
}

/// Draw a centered, bordered panel of content lines with a title and footer
/// hint. When `cursor` is `Some(i)`, content line `i` is drawn highlighted.
/// Clears the screen first so it reads as a modal page over the dashboard.
fn draw_panel(
    stdout: &mut impl Write,
    title: &str,
    lines: &[(&'static str, String)],
    hint: &str,
    cursor: Option<usize>,
) -> anyhow::Result<()> {
    let (tw, th) = terminal::size().unwrap_or((80, 24));

    let max_line = lines.iter().map(|(_, s)| s.chars().count()).max().unwrap_or(0);
    let inner_w = max_line
        .max(hint.chars().count() + 2)
        .max(title.chars().count() + 2)
        .min((tw as usize).saturating_sub(4))
        .max(20);

    // top + blank + content + blank + hint + bottom
    let panel_h = (lines.len() + 4) as u16;
    let panel_w = (inner_w + 4) as u16;
    let col = tw.saturating_sub(panel_w) / 2;
    let row = th.saturating_sub(panel_h) / 2;

    queue!(stdout, terminal::Clear(ClearType::All))?;

    let title_bar = format!(" {title} ");
    let top_dashes = (inner_w + 2).saturating_sub(title_bar.chars().count());
    execute!(stdout, cursor::MoveTo(col, row))?;
    write!(stdout, "{ORANGE}┌{title_bar}{}┐{RESET}", "─".repeat(top_dashes))?;

    execute!(stdout, cursor::MoveTo(col, row + 1))?;
    write!(stdout, "{ORANGE}│{}│{RESET}", " ".repeat(inner_w + 2))?;

    for (i, (color, text)) in lines.iter().enumerate() {
        execute!(stdout, cursor::MoveTo(col, row + 2 + i as u16))?;
        let fitted = fit(text, inner_w);
        if cursor == Some(i) {
            write!(stdout, "{ORANGE}│{RESET}{HILITE} {color}{fitted} {RESET}{ORANGE}│{RESET}")?;
        } else {
            write!(stdout, "{ORANGE}│ {color}{fitted}{RESET} {ORANGE}│{RESET}")?;
        }
    }

    let hint_row = row + 2 + lines.len() as u16;
    execute!(stdout, cursor::MoveTo(col, hint_row))?;
    write!(stdout, "{ORANGE}│{}│{RESET}", " ".repeat(inner_w + 2))?;
    execute!(stdout, cursor::MoveTo(col, hint_row + 1))?;
    let pad = inner_w.saturating_sub(hint.chars().count());
    write!(stdout, "{ORANGE}│ {GRAY}{hint}{}{RESET} {ORANGE}│{RESET}", " ".repeat(pad))?;
    execute!(stdout, cursor::MoveTo(col, hint_row + 2))?;
    write!(stdout, "{ORANGE}└{}┘{RESET}", "─".repeat(inner_w + 2))?;

    stdout.flush()?;
    Ok(())
}

/// A modal single-line text editor. Returns `Some(value)` on Enter (value may be
/// empty), `None` on Esc (cancelled — caller keeps the previous value).
fn prompt_line(stdout: &mut impl Write, label: &str, initial: &str) -> anyhow::Result<Option<String>> {
    let mut buf = initial.to_string();
    loop {
        let line = (WHITE, format!("{buf}_"));
        draw_panel(stdout, label, std::slice::from_ref(&line), "Enter save   Esc cancel", None)?;
        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) => return Ok(None),
                    (KeyCode::Enter, _) => return Ok(Some(buf)),
                    (KeyCode::Backspace, _) => { buf.pop(); }
                    (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                        buf.push(c);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// A modal list editor: add (`a`), remove the selected entry (`d`/Del),
/// navigate (↑↓). Esc commits and returns the edited list.
fn edit_string_list(stdout: &mut impl Write, label: &str, mut items: Vec<String>) -> anyhow::Result<Vec<String>> {
    let mut sel = 0usize;
    loop {
        let lines: Vec<(&'static str, String)> = if items.is_empty() {
            vec![(GRAY, "(empty)".to_string())]
        } else {
            items.iter().map(|s| (WHITE, s.clone())).collect()
        };
        let cursor = if items.is_empty() { None } else { Some(sel.min(items.len() - 1)) };
        draw_panel(stdout, label, &lines, "a add   d remove   ↑↓ move   Esc done", cursor)?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::NONE) => return Ok(items),
                    (KeyCode::Up, _) if !items.is_empty() => {
                        sel = if sel == 0 { items.len() - 1 } else { sel - 1 };
                    }
                    (KeyCode::Down, _) if !items.is_empty() => {
                        sel = (sel + 1) % items.len();
                    }
                    (KeyCode::Char('a'), KeyModifiers::NONE) => {
                        if let Some(v) = prompt_line(stdout, "New entry", "")? {
                            let v = v.trim().to_string();
                            if !v.is_empty() {
                                items.push(v);
                                sel = items.len() - 1;
                            }
                        }
                    }
                    (KeyCode::Char('d'), KeyModifiers::NONE) | (KeyCode::Delete, _) if !items.is_empty() => {
                        let i = sel.min(items.len() - 1);
                        items.remove(i);
                        if sel > 0 && sel >= items.len() { sel -= 1; }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Confirm dialog. Returns true if the user pressed `y`.
fn confirm_overlay(stdout: &mut impl Write, question: &str) -> anyhow::Result<bool> {
    let line = (WHITE, question.to_string());
    draw_panel(stdout, "confirm", std::slice::from_ref(&line), "y confirm   n / Esc cancel", None)?;
    loop {
        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(true),
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => return Ok(false),
                    _ => {}
                }
            }
        }
    }
}

/// Editable fields of a provider detail view, in display order.
enum Field {
    Alias,
    Enabled,
    Module(usize),
    Param(usize),
    BrowseCommand,
    BrowseArgs,
}

fn build_fields(schema: &[ConfigParam], modules: &[&str]) -> Vec<Field> {
    let mut fields = Vec::new();
    fields.push(Field::Alias);
    fields.push(Field::Enabled);
    for i in 0..modules.len() {
        fields.push(Field::Module(i));
    }
    for i in 0..schema.len() {
        fields.push(Field::Param(i));
    }
    fields.push(Field::BrowseCommand);
    fields.push(Field::BrowseArgs);
    fields
}

/// One rendered line per field: (color, "label: value"). Required string params
/// that are unset render red. The raw stored value is shown (no env expansion),
/// so `$TOKEN` stays `$TOKEN` while editing.
fn field_line(entry: &ProviderEntry, schema: &[ConfigParam], modules: &[&str], field: &Field) -> (&'static str, String) {
    let pad = "browse args".len(); // widest label, for alignment
    let row = |label: &str, value: String| format!("{label:<pad$}  {value}");
    match field {
        Field::Alias => {
            let v = entry.alias.clone().unwrap_or_else(|| "(none)".to_string());
            (WHITE, row("alias", v))
        }
        Field::Enabled => {
            let mark = if entry.enabled { "[x]" } else { "[ ]" };
            (WHITE, row("enabled", mark.to_string()))
        }
        Field::Module(i) => {
            let name = modules[*i];
            let on = entry.is_module_enabled(name);
            let mark = if on { "[x]" } else { "[ ]" };
            (WHITE, row(&format!("module {name}"), mark.to_string()))
        }
        Field::Param(i) => {
            let p = &schema[*i];
            let raw = entry.config.get(p.name).and_then(|v| v.as_str()).map(str::to_string);
            match p.kind {
                ConfigParamKind::String => match raw {
                    Some(v) => (WHITE, row(p.name, v)),
                    None if p.required => (RED, row(p.name, "✗ required".to_string())),
                    None => (GRAY, row(p.name, "(unset)".to_string())),
                },
                ConfigParamKind::List => {
                    let n = entry.get_list(p.name).len();
                    let color = if p.required && n == 0 { RED } else { WHITE };
                    (color, row(p.name, format!("[{n} item{}]", if n == 1 { "" } else { "s" })))
                }
            }
        }
        Field::BrowseCommand => {
            let v = entry.browse.as_ref().map(|b| b.command.clone()).unwrap_or_else(|| "(none)".to_string());
            (WHITE, row("browse cmd", v))
        }
        Field::BrowseArgs => {
            let n = entry.browse.as_ref().map(|b| b.args.len()).unwrap_or(0);
            (WHITE, row("browse args", format!("[{n} item{}]", if n == 1 { "" } else { "s" })))
        }
    }
}

/// Edit one provider entry in `cfg`. Saves to disk after each mutation and
/// returns whether anything changed.
fn edit_provider_detail(stdout: &mut impl Write, root: &Path, cfg: &mut ProjectConfig, idx: usize) -> anyhow::Result<bool> {
    let name = cfg.providers[idx].name.clone();
    let schema = provider_schema(&name);
    let modules = provider_modules(&name);
    let fields = build_fields(&schema, &modules);

    let mut sel = 0usize;
    let mut changed = false;

    loop {
        let entry = &cfg.providers[idx];
        let mut lines: Vec<(&'static str, String)> =
            fields.iter().map(|f| field_line(entry, &schema, &modules, f)).collect();

        let issues = entry_issues(entry);
        if !issues.is_empty() {
            lines.push((GRAY, String::new()));
            lines.push((RED, format!("⚠ {} issue{}", issues.len(), if issues.len() == 1 { "" } else { "s" })));
        }

        let title = match &entry.alias {
            Some(a) => format!("{name} ({a})"),
            None => name.clone(),
        };
        draw_panel(stdout, &title, &lines, "↑↓ move   Enter edit/toggle   Esc back", Some(sel))?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::NONE) => break,
                    (KeyCode::Up, _) => { sel = if sel == 0 { fields.len() - 1 } else { sel - 1 }; }
                    (KeyCode::Down, _) => { sel = (sel + 1) % fields.len(); }
                    (KeyCode::Enter, _) => {
                        if apply_field(stdout, &mut cfg.providers[idx], &schema, &modules, &fields[sel])? {
                            config::save(root, cfg)?;
                            changed = true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(changed)
}

/// Run the edit action for the selected field, mutating `entry`. Returns whether
/// the entry was modified.
fn apply_field(
    stdout: &mut impl Write,
    entry: &mut ProviderEntry,
    schema: &[ConfigParam],
    modules: &[&str],
    field: &Field,
) -> anyhow::Result<bool> {
    match field {
        Field::Alias => {
            let cur = entry.alias.clone().unwrap_or_default();
            if let Some(v) = prompt_line(stdout, "alias", &cur)? {
                let v = v.trim().to_string();
                entry.alias = if v.is_empty() { None } else { Some(v) };
                return Ok(true);
            }
        }
        Field::Enabled => {
            entry.enabled = !entry.enabled;
            return Ok(true);
        }
        Field::Module(i) => {
            let name = modules[*i].to_string();
            if let Some(pos) = entry.disabled_modules.iter().position(|m| *m == name) {
                entry.disabled_modules.remove(pos);
            } else {
                entry.disabled_modules.push(name);
            }
            return Ok(true);
        }
        Field::Param(i) => {
            let p = &schema[*i];
            match p.kind {
                ConfigParamKind::String => {
                    let cur = entry.config.get(p.name).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if let Some(v) = prompt_line(stdout, p.name, &cur)? {
                        let v = v.trim().to_string();
                        if v.is_empty() {
                            entry.config.remove(p.name);
                        } else {
                            entry.config.insert(p.name.to_string(), serde_yaml::Value::String(v));
                        }
                        return Ok(true);
                    }
                }
                ConfigParamKind::List => {
                    let cur = entry.get_list(p.name);
                    let updated = edit_string_list(stdout, p.name, cur)?;
                    if updated.is_empty() {
                        entry.config.remove(p.name);
                    } else {
                        let seq = updated.into_iter().map(serde_yaml::Value::String).collect();
                        entry.config.insert(p.name.to_string(), serde_yaml::Value::Sequence(seq));
                    }
                    return Ok(true);
                }
            }
        }
        Field::BrowseCommand => {
            let cur = entry.browse.as_ref().map(|b| b.command.clone()).unwrap_or_default();
            if let Some(v) = prompt_line(stdout, "browse cmd", &cur)? {
                let v = v.trim().to_string();
                if v.is_empty() {
                    entry.browse = None;
                } else {
                    match &mut entry.browse {
                        Some(b) => b.command = v,
                        None => entry.browse = Some(BrowseConfig { command: v, args: Vec::new() }),
                    }
                }
                return Ok(true);
            }
        }
        Field::BrowseArgs => {
            // Args are meaningless without a command, so only editable once a
            // browse command is set.
            let Some(b) = &mut entry.browse else { return Ok(false) };
            b.args = edit_string_list(stdout, "browse args", b.args.clone())?;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Edit the project-level `browse` block (the "General" detail view).
fn edit_general_detail(stdout: &mut impl Write, root: &Path, cfg: &mut ProjectConfig) -> anyhow::Result<bool> {
    let mut sel = 0usize;
    let mut changed = false;
    loop {
        let cmd = cfg.browse.as_ref().map(|b| b.command.clone()).unwrap_or_else(|| "(none)".to_string());
        let nargs = cfg.browse.as_ref().map(|b| b.args.len()).unwrap_or(0);
        let lines = vec![
            (WHITE, format!("{:<11}  {}", "browse cmd", cmd)),
            (WHITE, format!("{:<11}  [{} item{}]", "browse args", nargs, if nargs == 1 { "" } else { "s" })),
        ];
        draw_panel(stdout, "General", &lines, "↑↓ move   Enter edit   Esc back", Some(sel))?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::NONE) => break,
                    (KeyCode::Up, _) | (KeyCode::Down, _) => { sel = 1 - sel; }
                    (KeyCode::Enter, _) if sel == 0 => {
                        let cur = cfg.browse.as_ref().map(|b| b.command.clone()).unwrap_or_default();
                        if let Some(v) = prompt_line(stdout, "browse cmd", &cur)? {
                            let v = v.trim().to_string();
                            if v.is_empty() {
                                cfg.browse = None;
                            } else {
                                match &mut cfg.browse {
                                    Some(b) => b.command = v,
                                    None => cfg.browse = Some(BrowseConfig { command: v, args: Vec::new() }),
                                }
                            }
                            config::save(root, cfg)?;
                            changed = true;
                        }
                    }
                    (KeyCode::Enter, _) => {
                        let cur = cfg.browse.as_ref().map(|b| b.args.clone()).unwrap_or_default();
                        let updated = edit_string_list(stdout, "browse args", cur)?;
                        if let Some(b) = &mut cfg.browse {
                            b.args = updated;
                            config::save(root, cfg)?;
                            changed = true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(changed)
}

/// Master list of the config editor: "General" plus one row per provider.
/// Returns whether the config changed (so the caller can rebuild context).
fn show_config_editor(stdout: &mut impl Write, root: &Path) -> anyhow::Result<bool> {
    let mut cfg = config::load(root)?;
    let mut changed = false;
    let mut sel = 0usize; // 0 = General, 1.. = providers[sel-1]

    loop {
        let mut lines: Vec<(&'static str, String)> = Vec::new();
        lines.push((WHITE, "General".to_string()));
        for entry in &cfg.providers {
            let label = match &entry.alias {
                Some(a) => format!("{} ({})", entry.name, a),
                None => entry.name.clone(),
            };
            let mut status = String::new();
            if !entry.enabled {
                status.push_str("  ✗ off");
            }
            let issues = entry_issues(entry);
            if !issues.is_empty() {
                status.push_str(&format!("  ⚠ {}", issues.len()));
            }
            let color = if !issues.is_empty() { RED } else { WHITE };
            lines.push((color, format!("{label}{status}")));
        }

        let n = lines.len();
        draw_panel(stdout, "config", &lines, "↑↓ move   Enter open   a add   x delete   Esc close", Some(sel))?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::NONE) => break,
                    (KeyCode::Up, _) => { sel = if sel == 0 { n - 1 } else { sel - 1 }; }
                    (KeyCode::Down, _) => { sel = (sel + 1) % n; }
                    (KeyCode::Enter, _) => {
                        if sel == 0 {
                            changed |= edit_general_detail(stdout, root, &mut cfg)?;
                        } else {
                            changed |= edit_provider_detail(stdout, root, &mut cfg, sel - 1)?;
                        }
                    }
                    (KeyCode::Char('a'), KeyModifiers::NONE) => {
                        // `provider_add` uses dialoguer (cooked mode) and writes
                        // to disk itself, so drop out of the alternate screen
                        // around it, then reload to pick up the new entry.
                        execute!(stdout, cursor::Show, LeaveAlternateScreen)?;
                        terminal::disable_raw_mode()?;
                        let res = crate::provider_add(root);
                        terminal::enable_raw_mode()?;
                        execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
                        if res.is_ok() {
                            cfg = config::load(root)?;
                            changed = true;
                            sel = sel.min(cfg.providers.len());
                        }
                    }
                    (KeyCode::Char('x'), KeyModifiers::NONE) | (KeyCode::Delete, _) if sel > 0 => {
                        let entry = &cfg.providers[sel - 1];
                        if entry.name == "local" {
                            // The local provider is the on-disk base; never removable.
                        } else {
                            let q = format!("Remove provider '{}'? (config only; files stay on disk)", entry.display_name());
                            if confirm_overlay(stdout, &q)? {
                                cfg.providers.remove(sel - 1);
                                config::save(root, &cfg)?;
                                changed = true;
                                sel = sel.min(cfg.providers.len());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(changed)
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
    let cfg = crate::config::load(root).unwrap_or_default();
    // `local` is listed first when present and enabled. It's enabled unless an
    // explicit local entry sets `enabled: false`; absence of any entry (a fresh
    // or unconfigured project) still means local is on.
    let local_on = cfg
        .providers
        .iter()
        .find(|p| p.name == "local")
        .map_or(true, |p| p.enabled);

    let mut names = Vec::new();
    if local_on {
        names.push("local".to_string());
    }
    for p in &cfg.providers {
        if p.enabled && p.name != "local" {
            names.push(p.display_name().to_string());
        }
    }
    if names.is_empty() {
        names.push("(none)".to_string());
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

/// Build a repos item's display label: `name (branch) *↟N`, where `*` flags an
/// uncommitted working copy and `↟N` counts branches not merged into HEAD. Items
/// without a branch (non-repo modules, detached HEAD) render as the bare name.
fn repo_item_label(item: &serde_json::Value, name: &str) -> String {
    let Some(branch) = item.get("branch").and_then(|v| v.as_str()).filter(|b| !b.is_empty()) else {
        return name.to_string();
    };
    let dirty = item.get("dirty").and_then(|v| v.as_bool()).unwrap_or(false);
    let unmerged = item.get("unmerged_branches").and_then(|v| v.as_u64()).unwrap_or(0);

    let mut flags = String::new();
    if dirty {
        flags.push('*');
    }
    if unmerged > 0 {
        flags.push_str(&format!("↟{unmerged}"));
    }
    if flags.is_empty() {
        format!("{name} ({branch})")
    } else {
        format!("{name} ({branch}) {flags}")
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
                lines.push(ColumnLine::Item(format!("  {}", repo_item_label(item, name))));
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

/// A provider collapsed with `v`: a single bordered bar showing the provider
/// name and a per-module item-count summary, e.g. `▸ github  tasks (4) · repos (2)`.
fn render_collapsed_section(
    stdout: &mut impl Write,
    provider: &ProviderContext,
    modules: &[&ModuleContext],
    term_width: usize,
    focused: bool,
) -> anyhow::Result<()> {
    let border = if focused { ORANGE } else { GRAY };
    let name_color = if focused { ORANGE } else { WHITE };

    let summary = modules.iter()
        .map(|m| format!("{} ({})", m.name, m.items.len()))
        .collect::<Vec<_>>()
        .join(" · ");

    let inner = term_width.saturating_sub(2); // borders
    // "▸ " + name + "  " + summary, then padded/truncated to the inner width.
    let label = format!("▸ {}  ", provider.name);
    let label_len = label.chars().count();
    let summary_room = inner.saturating_sub(label_len);
    let body = format!("{name_color}{label}{GRAY}{}{RESET}", fit(&summary, summary_room));

    write!(stdout, "{border}│{RESET}{body}{border}│{RESET}\r\n")?;
    Ok(())
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

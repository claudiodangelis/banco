use std::path::{Path, PathBuf};

use tiny_http::{Header, Response, Server};

use crate::context::ModuleContext;
use crate::tui::{display_name, item_project_key, repo_item_label, resolve_item_path, sorted_statuses};

const ORANGE: &str = "#f97316";

pub fn serve(root: &Path, bind: &str, port: u16, open: bool) -> anyhow::Result<()> {
    let addr = format!("{bind}:{port}");
    let server = Server::http(&addr)
        .map_err(|e| anyhow::anyhow!("failed to bind {addr}: {e}"))?;

    let url = format!("http://{addr}");
    println!("banco serving {} on {}", root.display(), url);
    println!("Press Ctrl-C to stop.");

    if open {
        if let Err(e) = crate::open_browser(&url, None) {
            eprintln!("warning: could not open browser: {e:#}");
        }
    }

    for request in server.incoming_requests() {
        let raw = request.url().to_string();
        let (path, query) = match raw.split_once('?') {
            Some((p, q)) => (p, q),
            None => (raw.as_str(), ""),
        };

        let response = match path {
            "/" => html_response(render_dashboard(root)),
            "/search" => html_response(render_search_page()),
            "/item" => match handle_item(root, query) {
                Ok(body) => html_response(body),
                Err(code) => error_response(code),
            },
            "/mtime" => match handle_mtime(root, query) {
                Ok(body) => text_response(body),
                Err(code) => error_response(code),
            },
            "/log" => match handle_log(root, query) {
                Ok(body) => html_response(body),
                Err(code) => error_response(code),
            },
            "/api/search" => match handle_search(root, query) {
                Ok(body) => json_response(body),
                Err(code) => error_response(code),
            },
            _ => error_response(404),
        };

        let _ = request.respond(response);
    }

    Ok(())
}

// ── routes ─────────────────────────────────────────────────────────────────

fn handle_item(root: &Path, query: &str) -> Result<String, u16> {
    let rel = query_param(query, "path").ok_or(400u16)?;
    let path = safe_md_path(root, &rel).ok_or(404u16)?;
    let content = std::fs::read_to_string(&path).map_err(|_| 404u16)?;
    let (fm, body) = crate::providers::frontmatter::parse(&content);

    let title = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();

    let mut meta = String::new();
    if let Some(fm) = fm {
        meta.push_str(&format!("<span class=\"badge\">{}</span>", esc(&fm.status)));
        for tag in &fm.tags {
            meta.push_str(&format!("<span class=\"tag\">{}</span>", esc(tag)));
        }
    }

    let mut opts = pulldown_cmark::Options::empty();
    opts.insert(pulldown_cmark::Options::ENABLE_TABLES);
    opts.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    opts.insert(pulldown_cmark::Options::ENABLE_TASKLISTS);
    opts.insert(pulldown_cmark::Options::ENABLE_FOOTNOTES);
    let mut rendered = String::new();
    pulldown_cmark::html::push_html(&mut rendered, pulldown_cmark::Parser::new_ext(&body, opts));

    let rel_attr = esc(&rel);
    let inner = format!(
        r##"<a href="/" class="back">← dashboard</a>
<h1>{title}</h1>
<div class="meta">{meta}</div>
<article class="markdown">{rendered}</article>
<div class="goto">
  <div class="goto-panel" id="goto-panel" hidden>
    <a href="#" data-goto="top">↑ Go to top</a>
    <a href="#" data-goto="bottom">↓ Go to bottom</a>
    <div class="goto-toc" id="goto-toc"></div>
  </div>
  <button class="goto-btn" id="goto-btn" aria-expanded="false">Go to</button>
</div>
<script>
(function () {{
  const btn = document.getElementById("goto-btn");
  const panel = document.getElementById("goto-panel");
  const toc = document.getElementById("goto-toc");
  const heads = document.querySelectorAll(".markdown h1, .markdown h2, .markdown h3, .markdown h4, .markdown h5, .markdown h6");
  const used = {{}};
  heads.forEach(function (h, i) {{
    let id = h.id;
    if (!id) {{
      id = (h.textContent || "h").toLowerCase().trim().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "h";
      if (used[id] != null) {{ used[id]++; id = id + "-" + used[id]; }} else {{ used[id] = 0; }}
      h.id = id;
    }}
    const a = document.createElement("a");
    a.href = "#" + id;
    a.textContent = h.textContent;
    a.className = "lvl-" + h.tagName[1];
    a.addEventListener("click", function () {{ hide(); }});
    toc.appendChild(a);
  }});
  if (!heads.length) {{
    const e = document.createElement("span");
    e.className = "goto-empty";
    e.textContent = "No headings";
    toc.appendChild(e);
  }}
  function show() {{ panel.hidden = false; btn.setAttribute("aria-expanded", "true"); }}
  function hide() {{ panel.hidden = true; btn.setAttribute("aria-expanded", "false"); }}
  btn.addEventListener("click", function (e) {{ e.stopPropagation(); panel.hidden ? show() : hide(); }});
  panel.addEventListener("click", function (e) {{
    const t = e.target.closest("[data-goto]");
    if (!t) return;
    e.preventDefault();
    window.scrollTo({{ top: t.dataset.goto === "top" ? 0 : document.body.scrollHeight, behavior: "smooth" }});
    hide();
  }});
  document.addEventListener("click", function (e) {{ if (!panel.hidden && !panel.contains(e.target) && e.target !== btn) hide(); }});
  document.addEventListener("keydown", function (e) {{
    if (e.key === "Escape") {{ hide(); return; }}
    if (e.key !== "g" || e.ctrlKey || e.metaKey || e.altKey) return;
    const el = document.activeElement;
    const tag = el && el.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || (el && el.isContentEditable)) return;
    e.preventDefault();
    panel.hidden ? show() : hide();
  }});
}})();
</script>
<script>
const REL = "{rel_attr}";
let last = null;
async function poll() {{
  if (document.hidden) return;
  try {{
    const r = await fetch("/mtime?path=" + encodeURIComponent(REL));
    if (!r.ok) return;
    const v = await r.text();
    if (last === null) last = v;
    else if (v !== last) location.reload();
  }} catch (e) {{}}
}}
setInterval(poll, 2000);
document.addEventListener("visibilitychange", () => {{ if (!document.hidden) poll(); }});
poll();
</script>"##,
        title = esc(&title),
    );
    Ok(page(&title, &inner))
}

fn handle_mtime(root: &Path, query: &str) -> Result<String, u16> {
    let rel = query_param(query, "path").ok_or(400u16)?;
    let path = safe_md_path(root, &rel).ok_or(404u16)?;
    let meta = std::fs::metadata(&path).map_err(|_| 404u16)?;
    let modified = meta.modified().map_err(|_| 500u16)?;
    let secs = modified
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| 500u16)?
        .as_secs();
    Ok(secs.to_string())
}

// ── git history ──────────────────────────────────────────────────────────────

/// Maximum number of commits shown in the read-only history view.
const LOG_LIMIT: usize = 250;

fn handle_log(root: &Path, query: &str) -> Result<String, u16> {
    let rel = query_param(query, "path").ok_or(400u16)?;
    let repo = safe_repo_path(root, &rel).ok_or(404u16)?;
    let name = repo.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();

    let commits = crate::providers::git::git_log(&repo, LOG_LIMIT).ok_or(404u16)?;

    let mut rows = String::new();
    if commits.is_empty() {
        rows.push_str("<p class=\"muted\">No commits yet.</p>");
    } else {
        rows.push_str("<ul class=\"log\">");
        for c in &commits {
            rows.push_str(&format!(
                "<li><div class=\"log-subject\">{subject}</div>\
                 <div class=\"log-meta\"><code>{hash}</code> · {author} · {date}</div></li>",
                subject = esc(&c.subject),
                hash = esc(&c.short_hash),
                author = esc(&c.author),
                date = esc(&c.date),
            ));
        }
        rows.push_str("</ul>");
    }

    let capped = if commits.len() >= LOG_LIMIT {
        format!("<p class=\"muted\">Showing the most recent {LOG_LIMIT} commits.</p>")
    } else {
        String::new()
    };

    let inner = format!(
        "<a href=\"/\" class=\"back\">← dashboard</a>\
         <h1>{title} <span class=\"count\">history</span></h1>{capped}{rows}",
        title = esc(&name),
    );
    Ok(page(&name, &inner))
}

/// Resolve `rel` against `root`, ensuring the result stays inside `root`, is an
/// existing directory, and lives under a top-level `repos/` tree. Returns `None`
/// on traversal attempts or misses.
fn safe_repo_path(root: &Path, rel: &str) -> Option<PathBuf> {
    let root = root.canonicalize().ok()?;
    let canonical = root.join(rel).canonicalize().ok()?;
    if !canonical.starts_with(root.join("repos")) {
        return None;
    }
    if !canonical.is_dir() {
        return None;
    }
    Some(canonical)
}

// ── search ───────────────────────────────────────────────────────────────────

/// The module root directories whose `.md` files are searchable, mapped to the
/// module name used for display. Bookmarks are excluded — they aren't standalone
/// markdown documents. Repos are searched separately by directory name.
const SEARCHABLE: &[(&str, &str)] = &[("notes", "notes"), ("tasks", "tasks")];

struct SearchHit {
    rel: String,
    title: String,
    module: String,
    provider: String,
    snippet: String,
    /// `"doc"` for markdown files (linked via `/item`), `"repo"` for repositories
    /// (linked via `/log`).
    kind: &'static str,
}

fn handle_search(root: &Path, query: &str) -> Result<String, u16> {
    let q = query_param(query, "q").unwrap_or_default();
    let q = q.trim();
    if q.is_empty() {
        return Ok("[]".to_string());
    }
    let needle = q.to_lowercase();

    let mut hits = Vec::new();
    let root_canon = root.canonicalize().map_err(|_| 500u16)?;

    for (dir, module) in SEARCHABLE {
        let base = root.join(dir);
        if !base.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&base).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() || path.extension().map_or(true, |e| e != "md") {
                continue;
            }
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let title = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
            let (_, body) = crate::providers::frontmatter::parse(&content);

            let title_match = title.to_lowercase().contains(&needle);
            let body_lower = body.to_lowercase();
            let body_pos = body_lower.find(&needle);
            if !title_match && body_pos.is_none() {
                continue;
            }

            let rel = match path.strip_prefix(&root_canon).ok().or_else(|| path.strip_prefix(root).ok()) {
                Some(r) => r.to_string_lossy().replace('\\', "/"),
                None => continue,
            };
            // Provider is the first path segment under the module dir
            // (notes/local/… → local, tasks/github/owner/… → github).
            let provider = rel
                .strip_prefix(&format!("{dir}/"))
                .and_then(|r| r.split('/').next())
                .unwrap_or("")
                .to_string();

            hits.push(SearchHit {
                rel,
                title,
                module: module.to_string(),
                provider,
                snippet: make_snippet(&body, body_pos, &needle),
                kind: "doc",
            });
        }
    }

    // Repos are directories, not documents: match on the repo name only.
    let repos_base = root.join("repos");
    if repos_base.exists() {
        for provider_entry in std::fs::read_dir(&repos_base).into_iter().flatten().filter_map(|e| e.ok()) {
            let provider_path = provider_entry.path();
            if !provider_path.is_dir() {
                continue;
            }
            let provider = provider_path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
            for repo_entry in std::fs::read_dir(&provider_path).into_iter().flatten().filter_map(|e| e.ok()) {
                let repo_path = repo_entry.path();
                if !repo_path.is_dir() {
                    continue;
                }
                let name = repo_path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
                if !name.to_lowercase().contains(&needle) {
                    continue;
                }
                hits.push(SearchHit {
                    rel: format!("repos/{provider}/{name}"),
                    title: name,
                    module: "repos".to_string(),
                    provider: provider.clone(),
                    snippet: String::new(),
                    kind: "repo",
                });
            }
        }
    }

    hits.sort_by(|a, b| a.provider.cmp(&b.provider).then(a.module.cmp(&b.module)).then(a.title.cmp(&b.title)));

    let items: Vec<String> = hits
        .iter()
        .map(|h| {
            format!(
                "{{\"path\":{},\"title\":{},\"module\":{},\"provider\":{},\"snippet\":{},\"kind\":{}}}",
                json_str(&h.rel),
                json_str(&h.title),
                json_str(&h.module),
                json_str(&h.provider),
                json_str(&h.snippet),
                json_str(h.kind),
            )
        })
        .collect();
    Ok(format!("[{}]", items.join(",")))
}

/// A short excerpt around the first body match (or the document start when the
/// hit was title-only). Whitespace is collapsed so the snippet stays one line.
fn make_snippet(body: &str, pos: Option<usize>, needle: &str) -> String {
    let flat: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.is_empty() {
        return String::new();
    }
    let flat_lower = flat.to_lowercase();
    let at = pos.and(flat_lower.find(needle)).unwrap_or(0);

    let start = flat[..at].char_indices().rev().nth(40).map(|(i, _)| i).unwrap_or(0);
    let end = flat[at..]
        .char_indices()
        .nth(120)
        .map(|(i, _)| at + i)
        .unwrap_or(flat.len());

    let mut s = String::new();
    if start > 0 {
        s.push('…');
    }
    s.push_str(flat[start..end].trim());
    if end < flat.len() {
        s.push('…');
    }
    s
}

fn render_search_page() -> String {
    let inner = r#"<a href="/" class="back">← dashboard</a>
<h1>Search</h1>
<input id="q" class="search-input" type="search" placeholder="Search notes, tasks, and repos…" autofocus autocomplete="off">
<div id="results"></div>
<script>
const input = document.getElementById("q");
const results = document.getElementById("results");
let timer = null;

function esc(s) {
  return s.replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;","\"":"&quot;","'":"&#39;"}[c]));
}

async function run() {
  const q = input.value.trim();
  history.replaceState(null, "", q ? "/search?q=" + encodeURIComponent(q) : "/search");
  if (!q) { results.innerHTML = ""; return; }
  try {
    const r = await fetch("/api/search?q=" + encodeURIComponent(q));
    const hits = await r.json();
    if (!hits.length) { results.innerHTML = '<p class="muted">No matches.</p>'; return; }
    results.innerHTML = hits.map(h => {
      const route = h.kind === "repo" ? "/log?path=" : "/item?path=";
      const snippet = h.snippet ? '<div class="snippet">' + esc(h.snippet) + '</div>' : '';
      return '<div class="hit"><a href="' + route + encodeURIComponent(h.path) + '">' +
        esc(h.title) + '</a> <span class="hit-meta">' + esc(h.provider) + ' · ' + esc(h.module) +
        '</span>' + snippet + '</div>';
    }).join("");
  } catch (e) {
    results.innerHTML = '<p class="err">search failed</p>';
  }
}

input.addEventListener("input", () => { clearTimeout(timer); timer = setTimeout(run, 150); });

const initial = new URLSearchParams(location.search).get("q");
if (initial) { input.value = initial; run(); }
</script>"#;
    page("Search", inner)
}

// ── dashboard ────────────────────────────────────────────────────────────────

fn render_dashboard(root: &Path) -> String {
    let project = root.file_name().and_then(|s| s.to_str()).unwrap_or("?");
    let ctx = match crate::build_context(root) {
        Ok(c) => c,
        Err(e) => return page("banco", &format!("<p class=\"err\">failed to build context: {}</p>", esc(&format!("{e:#}")))),
    };

    let mut body = format!(
        "<h1><span class=\"brand\">banco</span> {}</h1><p><a class=\"nav\" href=\"/search\">Search →</a></p>",
        esc(project)
    );

    let providers: Vec<_> = ctx
        .providers
        .iter()
        .filter(|p| p.modules.iter().any(|m| !m.items.is_empty()))
        .collect();

    if providers.is_empty() {
        body.push_str("<p class=\"muted\">No items yet.</p>");
        return page("banco", &body);
    }

    for provider in providers {
        body.push_str(&format!(
            "<details class=\"provider\" data-collapse=\"provider:{key}\" open><summary><h2>{name}</h2></summary>",
            key = esc(&provider.name),
            name = esc(&provider.name),
        ));
        for module in &provider.modules {
            if module.items.is_empty() {
                continue;
            }
            body.push_str(&format!(
                "<details class=\"module\" data-collapse=\"module:{pkey}:{mkey}\" open><summary><h3>{name} <span class=\"count\">{count}</span></h3></summary>",
                pkey = esc(&provider.name),
                mkey = esc(&module.name),
                name = esc(&module.name),
                count = module.items.len(),
            ));
            body.push_str(&render_module(root, &provider.name, module));
            body.push_str("</details>");
        }
        body.push_str("</details>");
    }

    page("banco", &body)
}

fn render_module(root: &Path, provider_name: &str, module: &ModuleContext) -> String {
    match module.name.as_str() {
        "tasks" => render_tasks(root, provider_name, module),
        "bookmarks" => render_bookmarks(module),
        "notes" => render_notes(root, provider_name, module),
        _ => render_repos(provider_name, module),
    }
}

fn item_link(root: &Path, provider_name: &str, module: &ModuleContext, item: &serde_json::Value, label: &str) -> String {
    match resolve_item_path(root, provider_name, &module.name, item) {
        Some(path) => match path.strip_prefix(root).ok().and_then(|p| p.to_str()) {
            Some(rel) => format!(
                "<li><a href=\"/item?path={}\">{}</a></li>",
                url_encode(rel),
                esc(label)
            ),
            None => format!("<li>{}</li>", esc(label)),
        },
        None => format!("<li>{}</li>", esc(label)),
    }
}

fn render_tasks(root: &Path, provider_name: &str, module: &ModuleContext) -> String {
    let mut out = String::new();
    let has_projects = module.items.iter().any(|i| item_project_key(i).is_some());

    let render_status_group = |out: &mut String, items: &[&serde_json::Value]| {
        let statuses = sorted_statuses(
            items
                .iter()
                .filter_map(|i| i.get("status").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string())),
        );
        for status in &statuses {
            out.push_str(&format!("<h4 class=\"group\">{}</h4><ul>", esc(status)));
            for item in items {
                if item.get("status").and_then(|v| v.as_str()) != Some(status.as_str()) {
                    continue;
                }
                if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                    let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let prefix = if id.is_empty() { String::new() } else { format!("{id} ") };
                    let label = format!("{prefix}{}", display_name(name));
                    out.push_str(&item_link(root, provider_name, module, item, &label));
                }
            }
            out.push_str("</ul>");
        }
    };

    if has_projects {
        let mut seen = std::collections::HashSet::new();
        let mut projects: Vec<String> = module
            .items
            .iter()
            .filter_map(item_project_key)
            .filter(|p| seen.insert(p.clone()))
            .collect();
        projects.sort();
        for project in &projects {
            out.push_str(&format!("<h4 class=\"project\">{}</h4>", esc(project)));
            let items: Vec<&serde_json::Value> = module
                .items
                .iter()
                .filter(|i| item_project_key(i).as_deref() == Some(project.as_str()))
                .collect();
            render_status_group(&mut out, &items);
        }
    } else {
        let items: Vec<&serde_json::Value> = module.items.iter().collect();
        render_status_group(&mut out, &items);
    }
    out
}

fn render_notes(root: &Path, provider_name: &str, module: &ModuleContext) -> String {
    let mut out = String::new();

    let mut seen = std::collections::HashSet::new();
    let mut labels: Vec<String> = module
        .items
        .iter()
        .filter_map(|i| i.get("label").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string()))
        .filter(|s| seen.insert(s.clone()))
        .collect();
    labels.sort();

    out.push_str("<ul>");
    for item in &module.items {
        let label_empty = item.get("label").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty());
        if !label_empty {
            continue;
        }
        if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
            out.push_str(&item_link(root, provider_name, module, item, name));
        }
    }
    out.push_str("</ul>");

    for label in &labels {
        out.push_str(&format!("<h4 class=\"group\">{}</h4><ul>", esc(label)));
        for item in &module.items {
            if item.get("label").and_then(|v| v.as_str()) != Some(label.as_str()) {
                continue;
            }
            if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                out.push_str(&item_link(root, provider_name, module, item, name));
            }
        }
        out.push_str("</ul>");
    }
    out
}

fn render_bookmarks(module: &ModuleContext) -> String {
    let mut out = String::from("<ul>");
    for item in &module.items {
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let url = item.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let group = item.get("group").and_then(|v| v.as_str()).unwrap_or("");
        let label = if group.is_empty() || group == "default" {
            name.to_string()
        } else {
            format!("{group}/{name}")
        };
        if url.is_empty() {
            out.push_str(&format!("<li>{}</li>", esc(&label)));
        } else {
            out.push_str(&format!(
                "<li><a href=\"{}\" rel=\"noreferrer\" target=\"_blank\">{}</a></li>",
                esc(url),
                esc(&label)
            ));
        }
    }
    out.push_str("</ul>");
    out
}

fn render_repos(provider_name: &str, module: &ModuleContext) -> String {
    let mut out = String::from("<ul>");
    for item in &module.items {
        if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
            let label = repo_item_label(item, name);
            let rel = format!("repos/{provider_name}/{name}");
            out.push_str(&format!(
                "<li><a href=\"/log?path={}\">{}</a></li>",
                url_encode(&rel),
                esc(&label)
            ));
        }
    }
    out.push_str("</ul>");
    out
}

// ── path safety ──────────────────────────────────────────────────────────────

/// Resolve `rel` against `root`, ensuring the result stays inside `root` and is
/// an existing `.md` file. Returns `None` on traversal attempts or misses.
fn safe_md_path(root: &Path, rel: &str) -> Option<PathBuf> {
    let root = root.canonicalize().ok()?;
    let candidate = root.join(rel);
    let canonical = candidate.canonicalize().ok()?;
    if !canonical.starts_with(&root) {
        return None;
    }
    if !canonical.is_file() {
        return None;
    }
    if canonical.extension().map_or(true, |e| e != "md") {
        return None;
    }
    Some(canonical)
}

// ── query / encoding helpers ───────────────────────────────────────────────

fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k == key {
            Some(url_decode(v))
        } else {
            None
        }
    })
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hi = hex_val(bytes[i + 1]);
                let lo = hex_val(bytes[i + 2]);
                match (hi, lo) {
                    (Some(h), Some(l)) => {
                        out.push(h << 4 | l);
                        i += 3;
                    }
                    _ => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// A JSON-encoded string literal (with surrounding quotes), for hand-built JSON.
fn json_str(s: &str) -> String {
    serde_json::Value::String(s.to_string()).to_string()
}

// ── HTML shell / responses ─────────────────────────────────────────────────

fn page(title: &str, body: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · banco</title>
<script>
(function () {{
  try {{
    var t = localStorage.getItem("banco-theme");
    if (t !== "light" && t !== "dark") {{
      t = matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
    }}
    document.documentElement.setAttribute("data-theme", t);
  }} catch (e) {{
    document.documentElement.setAttribute("data-theme", "dark");
  }}
}})();
</script>
<style>
:root {{ --accent: {ORANGE}; }}
[data-theme="dark"] {{ --bg: #0f1115; --fg: #e6e6e6; --muted: #8a8f98; --card: #171a21; --border: #262b34; --code-bg: #0f1115; --on-accent: #0f1115; }}
[data-theme="light"] {{ --bg: #fbfbfa; --fg: #1f2328; --muted: #6b7280; --card: #ffffff; --border: #e2e4e8; --code-bg: #f3f4f6; --on-accent: #ffffff; }}
* {{ box-sizing: border-box; }}
body {{ margin: 0; background: var(--bg); color: var(--fg); font: 15px/1.6 -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }}
main {{ max-width: 900px; margin: 0 auto; padding: 2rem 1.25rem 4rem; }}
h1 {{ font-size: 1.5rem; margin: 0 0 1.5rem; }}
.brand {{ color: var(--accent); }}
h2 {{ font-size: 1.05rem; color: var(--accent); margin: 0; display: inline; }}
h3 {{ font-size: .95rem; margin: 0; display: inline; }}
h4 {{ font-size: .8rem; text-transform: uppercase; letter-spacing: .05em; color: var(--muted); margin: .9rem 0 .3rem; }}
h4.project {{ color: var(--accent); text-transform: none; letter-spacing: 0; font-size: .9rem; }}
.count {{ color: var(--muted); font-weight: 400; font-size: .8rem; }}
details.provider {{ margin: 1.25rem 0; }}
details.provider > summary {{ border-bottom: 1px solid var(--border); padding-bottom: .3rem; margin-bottom: 1rem; }}
details.module {{ margin: .5rem 0 .5rem .25rem; }}
details.module > summary {{ margin: .5rem 0 .4rem; }}
summary {{ list-style: none; cursor: pointer; user-select: none; }}
summary::-webkit-details-marker {{ display: none; }}
summary::before {{ content: "▾"; display: inline-block; width: 1em; color: var(--muted); transition: transform .15s; }}
details:not([open]) > summary::before {{ transform: rotate(-90deg); }}
summary:hover::before {{ color: var(--accent); }}
ul {{ list-style: none; margin: 0 0 .5rem; padding: 0; }}
li {{ padding: .15rem 0; }}
li a {{ color: var(--fg); text-decoration: none; border-bottom: 1px solid transparent; }}
li a:hover {{ color: var(--accent); border-color: var(--accent); }}
.muted, .err {{ color: var(--muted); }}
.err {{ color: #ef4444; }}
.back {{ color: var(--muted); text-decoration: none; font-size: .85rem; }}
.back:hover {{ color: var(--accent); }}
.meta {{ margin: -.75rem 0 1.5rem; }}
.badge {{ background: var(--accent); color: var(--on-accent); border-radius: 4px; padding: .05rem .5rem; font-size: .75rem; font-weight: 600; }}
.tag {{ background: var(--card); border: 1px solid var(--border); color: var(--muted); border-radius: 4px; padding: .05rem .5rem; font-size: .75rem; margin-left: .35rem; }}
.markdown {{ background: var(--card); border: 1px solid var(--border); border-radius: 8px; padding: 1.25rem 1.5rem; }}
.markdown :first-child {{ margin-top: 0; }}
.markdown h1, .markdown h2 {{ border: 0; color: var(--fg); }}
.markdown a {{ color: var(--accent); }}
.markdown code {{ background: var(--code-bg); border: 1px solid var(--border); border-radius: 4px; padding: .1rem .35rem; font-size: .85em; }}
.markdown pre {{ background: var(--code-bg); border: 1px solid var(--border); border-radius: 6px; padding: 1rem; overflow: auto; }}
.markdown pre code {{ border: 0; padding: 0; }}
.markdown ul {{ list-style: disc; margin: .5rem 0; padding-left: 1.5rem; }}
.markdown ol {{ list-style: decimal; margin: .5rem 0; padding-left: 1.5rem; }}
.markdown li {{ padding: .1rem 0; }}
.markdown li::marker {{ color: var(--muted); }}
.markdown li:has(> input[type="checkbox"]) {{ list-style: none; margin-left: -1.2rem; }}
.markdown li > input[type="checkbox"] {{ margin-right: .4rem; }}
.markdown blockquote {{ border-left: 3px solid var(--accent); margin: 1rem 0; padding: .1rem 1rem; color: var(--muted); }}
.markdown table {{ border-collapse: collapse; }}
.markdown th, .markdown td {{ border: 1px solid var(--border); padding: .35rem .6rem; }}
.theme-toggle {{ position: fixed; top: 1rem; right: 1rem; background: var(--card); color: var(--fg); border: 1px solid var(--border); border-radius: 6px; padding: .35rem .6rem; font-size: .85rem; cursor: pointer; line-height: 1; }}
.theme-toggle:hover {{ border-color: var(--accent); color: var(--accent); }}
.nav {{ color: var(--accent); text-decoration: none; font-size: .9rem; }}
.nav:hover {{ text-decoration: underline; }}
.search-input {{ width: 100%; background: var(--card); color: var(--fg); border: 1px solid var(--border); border-radius: 8px; padding: .6rem .8rem; font-size: 1rem; margin: .5rem 0 1.5rem; }}
.search-input:focus {{ outline: none; border-color: var(--accent); }}
.hit {{ padding: .6rem 0; border-bottom: 1px solid var(--border); }}
.hit > a {{ color: var(--fg); text-decoration: none; font-weight: 600; }}
.hit > a:hover {{ color: var(--accent); }}
.hit-meta {{ color: var(--muted); font-size: .78rem; margin-left: .5rem; }}
.snippet {{ color: var(--muted); font-size: .85rem; margin-top: .2rem; }}
.log li {{ padding: .5rem 0; border-bottom: 1px solid var(--border); }}
.log-subject {{ color: var(--fg); }}
.log-meta {{ color: var(--muted); font-size: .8rem; margin-top: .15rem; }}
.log-meta code {{ background: var(--code-bg); border: 1px solid var(--border); border-radius: 4px; padding: .02rem .3rem; font-size: .95em; }}
.goto {{ position: fixed; bottom: 1.25rem; right: 1.25rem; z-index: 50; }}
.goto-btn {{ background: var(--accent); color: var(--on-accent); border: 0; border-radius: 6px; padding: .5rem .8rem; font-size: .85rem; font-weight: 600; cursor: pointer; box-shadow: 0 2px 8px rgba(0,0,0,.3); }}
.goto-panel {{ position: absolute; bottom: 2.6rem; right: 0; background: var(--card); border: 1px solid var(--border); border-radius: 8px; padding: .4rem; min-width: 200px; max-width: 340px; max-height: 60vh; overflow: auto; box-shadow: 0 4px 16px rgba(0,0,0,.35); }}
.goto-panel a {{ display: block; color: var(--fg); text-decoration: none; padding: .25rem .5rem; border-radius: 4px; font-size: .85rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }}
.goto-panel a:hover {{ background: var(--code-bg); color: var(--accent); }}
.goto-toc {{ border-top: 1px solid var(--border); margin-top: .3rem; padding-top: .3rem; }}
.goto-toc a.lvl-1 {{ padding-left: .5rem; font-weight: 600; }}
.goto-toc a.lvl-2 {{ padding-left: 1.1rem; }}
.goto-toc a.lvl-3 {{ padding-left: 1.7rem; }}
.goto-toc a.lvl-4 {{ padding-left: 2.3rem; }}
.goto-toc a.lvl-5 {{ padding-left: 2.9rem; }}
.goto-toc a.lvl-6 {{ padding-left: 3.5rem; }}
.goto-empty {{ display: block; color: var(--muted); font-size: .8rem; padding: .25rem .5rem; }}
</style>
</head>
<body>
<button class="theme-toggle" onclick="bancoToggleTheme()" title="Toggle theme" aria-label="Toggle theme"></button>
<main>{body}</main>
<script>
function bancoApplyThemeLabel() {{
  var t = document.documentElement.getAttribute("data-theme");
  var b = document.querySelector(".theme-toggle");
  if (b) b.textContent = t === "light" ? "◐ dark" : "◑ light";
}}
function bancoToggleTheme() {{
  var t = document.documentElement.getAttribute("data-theme") === "light" ? "dark" : "light";
  document.documentElement.setAttribute("data-theme", t);
  try {{ localStorage.setItem("banco-theme", t); }} catch (e) {{}}
  bancoApplyThemeLabel();
}}
bancoApplyThemeLabel();

(function () {{
  var KEY = "banco-collapsed";
  var collapsed;
  try {{ collapsed = new Set(JSON.parse(localStorage.getItem(KEY) || "[]")); }}
  catch (e) {{ collapsed = new Set(); }}
  function save() {{
    try {{ localStorage.setItem(KEY, JSON.stringify([...collapsed])); }} catch (e) {{}}
  }}
  document.querySelectorAll("details[data-collapse]").forEach(function (d) {{
    var id = d.getAttribute("data-collapse");
    if (collapsed.has(id)) d.open = false;
    d.addEventListener("toggle", function () {{
      if (d.open) collapsed.delete(id); else collapsed.add(id);
      save();
    }});
  }});
}})();

document.addEventListener("keydown", function (e) {{
  if (e.key !== "/" || e.ctrlKey || e.metaKey || e.altKey) return;
  var el = document.activeElement;
  var tag = el && el.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || (el && el.isContentEditable)) return;
  var input = document.getElementById("q");
  if (input) {{
    e.preventDefault();
    input.focus();
    input.select();
  }} else {{
    e.preventDefault();
    location.href = "/search";
  }}
}});
</script>
</body>
</html>"#
    )
}

fn html_response(body: String) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(body).with_header(
        Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap(),
    )
}

fn text_response(body: String) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(body).with_header(
        Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..]).unwrap(),
    )
}

fn json_response(body: String) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(body).with_header(
        Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap(),
    )
}

fn error_response(code: u16) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(format!("{code}")).with_status_code(code)
}

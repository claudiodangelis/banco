use std::path::{Path, PathBuf};

use tiny_http::{Header, Response, Server};

use crate::context::ModuleContext;
use crate::tui::{display_name, item_project_key, repo_item_label, resolve_item_path, sorted_statuses};

const ORANGE: &str = "#f97316";

/// Square workbench glyph from the logo, used as the browser favicon.
const FAVICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <g fill="#F97316">
    <rect x="6" y="20" width="88" height="10" rx="2"/>
    <rect x="24" y="32" width="10" height="54" rx="1"/>
    <rect x="66" y="32" width="10" height="54" rx="1"/>
    <rect x="34" y="66" width="32" height="8" rx="1"/>
  </g>
</svg>"##;

pub fn serve(root: &Path, bind: &str, port: u16, open: bool) -> anyhow::Result<()> {
    crate::providers::git::skip_repo_status();
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
            "/search" => html_response(render_search_page(root)),
            "/config" => {
                // `partial=1` returns just the fragment, for the gear dialog.
                if query_param(query, "partial").as_deref() == Some("1") {
                    html_response(render_config(root))
                } else {
                    let body = format!("<h1>Configuration</h1>{}", render_config(root));
                    html_response(page(root, "Configuration", &body))
                }
            }
            "/favicon.svg" => svg_response(FAVICON),
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
            p if p.starts_with("/misc/") => match handle_misc(root, p) {
                Ok((body, content_type)) => bytes_response(body, content_type),
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
        r##"<h1>{title}</h1>
<div class="meta">{meta}</div>
<article class="markdown">{rendered}</article>
<div class="goto">
  <button class="copy-path" data-path="{rel_attr}" onclick="bancoCopyPath(this)">Copy path</button>
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
    Ok(page(root, &title, &inner))
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

// ── misc assets ──────────────────────────────────────────────────────────────

/// Serve a file from the provider-agnostic top-level `misc/` directory, so that
/// images and other assets referenced from rendered markdown resolve.
fn handle_misc(root: &Path, path: &str) -> Result<(Vec<u8>, &'static str), u16> {
    let rel = url_decode(path.trim_start_matches('/'));
    let file = safe_misc_path(root, &rel).ok_or(404u16)?;
    let bytes = std::fs::read(&file).map_err(|_| 404u16)?;
    Ok((bytes, content_type(&file)))
}

/// Resolve `rel` against `root`, ensuring the result stays inside `root/misc`
/// and is an existing file. Returns `None` on traversal attempts or misses.
fn safe_misc_path(root: &Path, rel: &str) -> Option<PathBuf> {
    let root = root.canonicalize().ok()?;
    let canonical = root.join(rel).canonicalize().ok()?;
    if !canonical.starts_with(root.join("misc")) {
        return None;
    }
    if !canonical.is_file() {
        return None;
    }
    Some(canonical)
}

fn content_type(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "json" => "application/json",
        "css" => "text/css; charset=utf-8",
        "txt" | "md" | "log" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
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
        "<h1>{title} <span class=\"count\">history</span></h1>\
         {capped}{rows}\
         <div class=\"goto\">\
           <button class=\"copy-path\" data-path=\"{rel}\" onclick=\"bancoCopyPath(this)\">Copy path</button>\
         </div>",
        title = esc(&name),
        rel = esc(&rel),
    );
    Ok(page(root, &name, &inner))
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
            // Discover working copies at any depth: flat providers nest one level
            // (`repos/github/<repo>`), Gerrit nests by project path
            // (`repos/gerrit/<owner>/<repo>`).
            for repo_path in crate::providers::git::discover_repos(&provider_path) {
                let name = repo_path
                    .strip_prefix(&provider_path)
                    .unwrap_or(&repo_path)
                    .to_string_lossy()
                    .replace('\\', "/");
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

fn render_search_page(root: &Path) -> String {
    let inner = r#"<h1>Search</h1>
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
    page(root, "Search", inner)
}

// ── dashboard ────────────────────────────────────────────────────────────────

fn render_dashboard(root: &Path) -> String {
    let project = project_name(root);
    let ctx = match crate::build_context(root) {
        Ok(c) => c,
        Err(e) => return page(root, project, &format!("<p class=\"err\">failed to build context: {}</p>", esc(&format!("{e:#}")))),
    };

    let mut body = format!(
        "<h1><span class=\"brand\">banco</span> {}</h1>",
        esc(project)
    );

    let providers: Vec<_> = ctx
        .providers
        .iter()
        .filter(|p| p.modules.iter().any(|m| !m.items.is_empty()))
        .collect();

    if providers.is_empty() {
        body.push_str("<p class=\"muted\">No items yet.</p>");
        return page(root, project, &body);
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

    page(root, project, &body)
}

fn render_module(root: &Path, provider_name: &str, module: &ModuleContext) -> String {
    match module.name.as_str() {
        "tasks" => render_tasks(root, provider_name, module),
        "bookmarks" => render_bookmarks(module),
        "notes" => render_notes(root, provider_name, module),
        _ => render_repos(provider_name, module),
    }
}

/// The `/item` URL for a markdown-backed item, or `None` when it has no file.
fn item_href(root: &Path, provider_name: &str, module_name: &str, item: &serde_json::Value) -> Option<String> {
    let path = resolve_item_path(root, provider_name, module_name, item)?;
    let rel = path.strip_prefix(root).ok()?.to_str()?;
    Some(format!("/item?path={}", url_encode(rel)))
}

fn repo_href(provider_name: &str, name: &str) -> String {
    format!("/log?path={}", url_encode(&format!("repos/{provider_name}/{name}")))
}

fn item_link(root: &Path, provider_name: &str, module: &ModuleContext, item: &serde_json::Value, label: &str) -> String {
    match item_href(root, provider_name, &module.name, item) {
        Some(href) => format!("<li><a href=\"{href}\">{}</a></li>", esc(label)),
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
            out.push_str(&format!(
                "<li><a href=\"{}\">{}</a></li>",
                repo_href(provider_name, name),
                esc(&label)
            ));
        }
    }
    out.push_str("</ul>");
    out
}

// ── sidebar ──────────────────────────────────────────────────────────────────

/// Top-level module order in the sidebar; modules not listed follow, in the
/// order they first appear in the context.
const MODULE_ORDER: &[&str] = &["repos", "tasks", "notes", "bookmarks"];

/// Display names of remote providers paused with `enabled: false`. Their data
/// stays in the context; the sidebar just flags them.
fn paused_providers(root: &Path) -> std::collections::HashSet<String> {
    crate::config::load(root)
        .map(|c| {
            c.providers
                .iter()
                .filter(|e| !e.enabled && e.name != "local")
                .map(|e| e.display_name().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn project_name(root: &Path) -> &str {
    root.file_name().and_then(|s| s.to_str()).unwrap_or("?")
}

/// The whole sidebar: project header with the config/theme/hide buttons, the
/// search field, and the scrollable module tree.
fn render_sidebar(root: &Path) -> String {
    let mut out = format!(
        r#"<div class="sb-head">
  <a class="sb-project" href="/" title="Dashboard"><img src="/favicon.svg" alt=""><span>{project}</span></a>
  <button class="icon-btn" onclick="bancoShowConfig()" title="Configuration" aria-label="Configuration">{GEAR_SVG}</button>
  <button class="icon-btn theme-toggle" onclick="bancoToggleTheme()" title="Toggle theme" aria-label="Toggle theme"></button>
  <button class="icon-btn" onclick="bancoToggleSidebar()" title="Hide sidebar" aria-label="Hide sidebar" aria-controls="sidebar">{PANEL_SVG}</button>
</div>
<form class="sb-search" action="/search" method="get" role="search">
  <input id="sb-q" name="q" type="search" placeholder="Search…" autocomplete="off" aria-label="Search notes, tasks, and repos">
  <kbd>/</kbd>
</form>
<div class="sb-scroll" id="sb-scroll">"#,
        project = esc(project_name(root)),
    );
    out.push_str(&render_tree(root));
    out.push_str("</div>");
    out
}

/// Gear icon for the configuration button.
const GEAR_SVG: &str = r#"<svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>"#;

/// Panel icon for the sidebar show/hide buttons.
const PANEL_SVG: &str = r#"<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><rect x="1.5" y="2.5" width="13" height="11" rx="2" fill="none" stroke="currentColor" stroke-width="1.3"/><line x1="6" y1="2.5" x2="6" y2="13.5" stroke="currentColor" stroke-width="1.3"/></svg>"#;

/// Module → provider → items tree. Built from the context, so disabled modules
/// and a fully-disabled local provider are already absent.
fn render_tree(root: &Path) -> String {
    let mut out = String::new();
    let ctx = match crate::build_context(root) {
        Ok(c) => c,
        Err(e) => {
            out.push_str(&format!("<p class=\"err sb-pad\">{}</p>", esc(&format!("{e:#}"))));
            return out;
        }
    };
    let paused = paused_providers(root);

    let mut modules: Vec<&str> = Vec::new();
    for p in &ctx.providers {
        for m in &p.modules {
            if !modules.contains(&m.name.as_str()) {
                modules.push(&m.name);
            }
        }
    }
    modules.sort_by_key(|m| MODULE_ORDER.iter().position(|o| o == m).unwrap_or(MODULE_ORDER.len()));

    if modules.is_empty() {
        out.push_str("<p class=\"muted sb-pad\">No modules enabled.</p>");
        return out;
    }

    out.push_str("<nav class=\"tree\">");
    for module in modules {
        let entries: Vec<(&str, &ModuleContext)> = ctx
            .providers
            .iter()
            .filter_map(|p| p.modules.iter().find(|m| m.name == module).map(|m| (p.name.as_str(), m)))
            .collect();
        let total: usize = entries.iter().map(|(_, m)| m.items.len()).sum();
        out.push_str(&format!(
            "<details class=\"t-module\" data-tree=\"m:{key}\" open><summary><span class=\"t-name\">{key}/</span><span class=\"count\">{total}</span></summary>",
            key = esc(module),
        ));
        for (provider, m) in entries {
            let badge = if paused.contains(provider) {
                "<span class=\"pill warn\" title=\"enabled: false — skipped by a bare banco sync\">paused</span>"
            } else {
                ""
            };
            out.push_str(&format!(
                "<details class=\"t-provider\" data-tree=\"p:{mkey}:{pkey}\" open><summary><span class=\"t-name\">{pkey}</span>{badge}<span class=\"count\">{n}</span></summary>",
                mkey = esc(module),
                pkey = esc(provider),
                n = m.items.len(),
            ));
            if m.items.is_empty() {
                out.push_str("<div class=\"t-empty\">empty</div>");
            } else {
                out.push_str(&match module {
                    "tasks" => tree_tasks(root, provider, m),
                    "notes" => tree_notes(root, provider, m),
                    "bookmarks" => tree_bookmarks(provider, m),
                    _ => tree_repos(provider, m),
                });
            }
            out.push_str("</details>");
        }
        out.push_str("</details>");
    }
    out.push_str("</nav>");
    out
}

fn is_closed_status(status: &str) -> bool {
    status.eq_ignore_ascii_case("done") || status.eq_ignore_ascii_case("closed")
}

fn tree_leaf(href: Option<String>, label: &str, class: &str) -> String {
    match href {
        Some(h) => format!("<li><a href=\"{}\" class=\"{class}\" title=\"{t}\">{t}</a></li>", esc(&h), t = esc(label)),
        None => format!("<li><span class=\"t-leaf {class}\" title=\"{t}\">{t}</span></li>", t = esc(label)),
    }
}

fn tree_group(id: &str, title: &str, open: bool, count: usize, leaves: &str) -> String {
    format!(
        "<details class=\"t-group\" data-tree=\"{id}\"{open}><summary><span class=\"t-name\">{title}</span><span class=\"count\">{count}</span></summary><ul>{leaves}</ul></details>",
        id = esc(id),
        title = esc(title),
        open = if open { " open" } else { "" },
    )
}

/// Tasks grouped by project for remote providers (which carry one), otherwise
/// by status. Done/closed status groups start collapsed; closed items inside a
/// project group are dimmed.
fn tree_tasks(root: &Path, provider: &str, module: &ModuleContext) -> String {
    let status_of = |i: &serde_json::Value| i.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let leaf = |i: &serde_json::Value| -> String {
        let Some(name) = i.get("name").and_then(|v| v.as_str()) else { return String::new() };
        let id = i.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let label = if id.is_empty() { display_name(name).to_string() } else { format!("{id} {}", display_name(name)) };
        let class = if is_closed_status(&status_of(i)) { "dim" } else { "" };
        tree_leaf(item_href(root, provider, "tasks", i), &label, class)
    };

    let mut out = String::new();
    let has_projects = module.items.iter().any(|i| item_project_key(i).is_some());
    let (keys, key_of): (Vec<String>, Box<dyn Fn(&serde_json::Value) -> String>) = if has_projects {
        let mut seen = std::collections::HashSet::new();
        let mut projects: Vec<String> = module.items.iter().filter_map(item_project_key).filter(|p| seen.insert(p.clone())).collect();
        projects.sort();
        (projects, Box::new(|i| item_project_key(i).unwrap_or_default()))
    } else {
        let statuses = sorted_statuses(module.items.iter().map(status_of).filter(|s| !s.is_empty()));
        (statuses, Box::new(status_of))
    };

    // Items with no group key sit directly under the provider.
    let loose: String = module.items.iter().filter(|i| key_of(i).is_empty()).map(leaf).collect();
    if !loose.is_empty() {
        out.push_str(&format!("<ul>{loose}</ul>"));
    }
    for key in &keys {
        let items: Vec<&serde_json::Value> = module.items.iter().filter(|i| &key_of(i) == key).collect();
        let leaves: String = items.iter().map(|i| leaf(i)).collect();
        let open = has_projects || !is_closed_status(key);
        out.push_str(&tree_group(&format!("g:tasks:{provider}:{key}"), key, open, items.len(), &leaves));
    }
    out
}

/// Unlabeled notes first, then one group per label (subdirectory).
fn tree_notes(root: &Path, provider: &str, module: &ModuleContext) -> String {
    let label_of = |i: &serde_json::Value| i.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let leaf = |i: &serde_json::Value| -> String {
        match i.get("name").and_then(|v| v.as_str()) {
            Some(name) => tree_leaf(item_href(root, provider, "notes", i), name, ""),
            None => String::new(),
        }
    };

    let mut out = String::new();
    let loose: String = module.items.iter().filter(|i| label_of(i).is_empty()).map(leaf).collect();
    if !loose.is_empty() {
        out.push_str(&format!("<ul>{loose}</ul>"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut labels: Vec<String> = module.items.iter().map(label_of).filter(|l| !l.is_empty() && seen.insert(l.clone())).collect();
    labels.sort();
    for label in &labels {
        let items: Vec<&serde_json::Value> = module.items.iter().filter(|i| &label_of(i) == label).collect();
        let leaves: String = items.iter().map(|i| leaf(i)).collect();
        out.push_str(&tree_group(&format!("g:notes:{provider}:{label}"), label, true, items.len(), &leaves));
    }
    out
}

/// Bookmarks in the `default` group sit directly under the provider; other
/// group files become groups. Links open in a new tab.
fn tree_bookmarks(provider: &str, module: &ModuleContext) -> String {
    let group_of = |i: &serde_json::Value| {
        let g = i.get("group").and_then(|v| v.as_str()).unwrap_or("");
        if g == "default" { String::new() } else { g.to_string() }
    };
    let leaf = |i: &serde_json::Value| -> String {
        let name = i.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let url = i.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if url.is_empty() {
            tree_leaf(None, name, "")
        } else {
            format!(
                "<li><a href=\"{u}\" rel=\"noreferrer\" target=\"_blank\" class=\"ext\" title=\"{u}\">{n}</a></li>",
                u = esc(url),
                n = esc(name),
            )
        }
    };

    let mut out = String::new();
    let loose: String = module.items.iter().filter(|i| group_of(i).is_empty()).map(leaf).collect();
    if !loose.is_empty() {
        out.push_str(&format!("<ul>{loose}</ul>"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut groups: Vec<String> = module.items.iter().map(group_of).filter(|g| !g.is_empty() && seen.insert(g.clone())).collect();
    groups.sort();
    for group in &groups {
        let items: Vec<&serde_json::Value> = module.items.iter().filter(|i| &group_of(i) == group).collect();
        let leaves: String = items.iter().map(|i| leaf(i)).collect();
        out.push_str(&tree_group(&format!("g:bookmarks:{provider}:{group}"), group, true, items.len(), &leaves));
    }
    out
}

fn tree_repos(provider: &str, module: &ModuleContext) -> String {
    let leaves: String = module
        .items
        .iter()
        .filter_map(|i| {
            let name = i.get("name").and_then(|v| v.as_str())?;
            Some(tree_leaf(Some(repo_href(provider, name)), &repo_item_label(i, name), ""))
        })
        .collect();
    format!("<ul>{leaves}</ul>")
}

// ── configuration ────────────────────────────────────────────────────────────

/// Config keys whose literal values are never shown. Env-var references
/// (`$VAR`) are safe to show and are displayed with whether the var is set.
fn is_secret_key(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    ["key", "token", "secret", "password", "passwd"].iter().any(|s| k.contains(s))
}

fn env_ref(s: &str) -> Option<&str> {
    s.strip_prefix("${").and_then(|v| v.strip_suffix('}')).or_else(|| s.strip_prefix('$'))
}

fn config_value_html(key: &str, value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(s) => {
            if let Some(var) = env_ref(s) {
                let set = std::env::var_os(var).is_some();
                format!(
                    "<code>{}</code><span class=\"env {cls}\">{label}</span>",
                    esc(s),
                    cls = if set { "ok" } else { "missing" },
                    label = if set { "set" } else { "unset" },
                )
            } else if is_secret_key(key) {
                "<span class=\"redacted\">redacted</span>".to_string()
            } else {
                format!("<code>{}</code>", esc(s))
            }
        }
        serde_yaml::Value::Sequence(seq) => seq
            .iter()
            .map(|v| match v {
                serde_yaml::Value::String(s) => format!("<code>{}</code>", esc(s)),
                other => format!("<code>{}</code>", esc(serde_yaml::to_string(other).unwrap_or_default().trim())),
            })
            .collect(),
        other => format!("<code>{}</code>", esc(serde_yaml::to_string(other).unwrap_or_default().trim())),
    }
}

fn browse_html(b: &crate::config::BrowseConfig) -> String {
    let mut s = format!("<code>{}</code>", esc(&b.command));
    for a in &b.args {
        s.push_str(&format!("<code>{}</code>", esc(a)));
    }
    s
}

/// Read-only view of `.banco/config.yml`: one card per provider with its
/// state, modules (disabled ones struck through), and config values with
/// secrets redacted. Returned as a fragment for both `/config` and the dialog.
fn render_config(root: &Path) -> String {
    let path = root.join(".banco/config.yml");
    let mut out = String::from("<div class=\"cfg\">");
    out.push_str("<div class=\"cfg-path\"><code>.banco/config.yml</code> · read-only</div>");

    let mut cfg = match crate::config::load(root) {
        Ok(c) => c,
        Err(e) => {
            out.push_str(&format!("<p class=\"err\">{}</p></div>", esc(&format!("{e:#}"))));
            return out;
        }
    };
    if !path.exists() {
        out.push_str("<p class=\"muted\">No config file found: defaults apply (local provider, every module on).</p></div>");
        return out;
    }

    if let Some(b) = &cfg.browse {
        out.push_str(&format!(
            "<section class=\"cfg-provider\"><header><strong>project</strong></header><div class=\"cfg-row\"><span class=\"k\">browse</span><span class=\"v\">{}</span></div></section>",
            browse_html(b)
        ));
    }

    for entry in &cfg.providers {
        let is_local = entry.name == "local";
        let (state, cls) = match (is_local, entry.enabled) {
            (true, true) => ("enabled", "ok"),
            (true, false) => ("disabled", "off"),
            (false, true) => ("active", "ok"),
            (false, false) => ("paused", "warn"),
        };
        out.push_str("<section class=\"cfg-provider\"><header>");
        out.push_str(&format!("<strong>{}</strong>", esc(entry.display_name())));
        if entry.alias.is_some() {
            out.push_str(&format!("<span class=\"cfg-kind\">{}</span>", esc(&entry.name)));
        }
        out.push_str(&format!("<span class=\"pill {cls}\">{state}</span></header>"));

        let modules = crate::provider_kind_modules(&entry.name);
        if !modules.is_empty() {
            let chips: String = modules
                .iter()
                .map(|m| {
                    // Pausing a remote keeps its modules; disabling local turns them all off.
                    let on = (entry.enabled || !is_local) && entry.is_module_enabled(m);
                    format!(
                        "<span class=\"chip{}\" title=\"{}\">{}</span>",
                        if on { "" } else { " off" },
                        if on { "enabled" } else { "disabled" },
                        esc(m)
                    )
                })
                .collect();
            out.push_str(&format!("<div class=\"cfg-row\"><span class=\"k\">modules</span><span class=\"v\">{chips}</span></div>"));
        }

        let mut keys: Vec<&String> = entry.config.keys().collect();
        keys.sort();
        for key in keys {
            out.push_str(&format!(
                "<div class=\"cfg-row\"><span class=\"k\">{}</span><span class=\"v\">{}</span></div>",
                esc(key),
                config_value_html(key, &entry.config[key])
            ));
        }
        if let Some(b) = &entry.browse {
            out.push_str(&format!("<div class=\"cfg-row\"><span class=\"k\">browse</span><span class=\"v\">{}</span></div>", browse_html(b)));
        }
        out.push_str("</section>");
    }

    // Effective config re-serialized (comments are not preserved), with
    // literal secrets replaced so it is safe to display.
    for entry in &mut cfg.providers {
        for (key, value) in entry.config.iter_mut() {
            if let serde_yaml::Value::String(s) = value {
                if is_secret_key(key) && env_ref(s).is_none() {
                    *s = "<redacted>".to_string();
                }
            }
        }
    }
    let yaml = serde_yaml::to_string(&cfg).unwrap_or_default();
    out.push_str(&format!(
        "<details class=\"cfg-raw\"><summary>Effective YAML</summary><pre><code>{}</code></pre></details>",
        esc(&yaml)
    ));
    out.push_str("</div>");
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

fn page(root: &Path, title: &str, body: &str) -> String {
    let title = esc(title);
    let sidebar = render_sidebar(root);
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="icon" href="/favicon.svg" type="image/svg+xml">
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
  // Sidebar visibility is set before first paint to avoid a flash. Narrow
  // screens always start hidden (the sidebar overlays content there).
  var sb = "shown";
  try {{
    if (matchMedia("(max-width: 800px)").matches || localStorage.getItem("banco-sidebar") === "hidden") sb = "hidden";
  }} catch (e) {{}}
  document.documentElement.setAttribute("data-sidebar", sb);
  try {{
    var w = parseInt(localStorage.getItem("banco-sidebar-width"), 10);
    if (w >= 180 && w <= 640) document.documentElement.style.setProperty("--sb-w", w + "px");
  }} catch (e) {{}}
}})();
</script>
<style>
:root {{ --accent: {ORANGE}; --sb-w: 280px; --mono: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 14px; }}
[data-theme="dark"] {{ --bg: #0f1115; --fg: #e6e6e6; --muted: #8a8f98; --card: #171a21; --border: #262b34; --code-bg: #0f1115; --on-accent: #0f1115; --side: #12151b; --hover: #1c2129; }}
[data-theme="light"] {{ --bg: #fbfbfa; --fg: #1f2328; --muted: #6b7280; --card: #ffffff; --border: #e2e4e8; --code-bg: #f3f4f6; --on-accent: #ffffff; --side: #f4f4f2; --hover: #e9e9e6; }}
* {{ box-sizing: border-box; }}
body {{ margin: 0; background: var(--bg); color: var(--fg); font: 1rem/1.6 -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }}
main {{ max-width: 900px; margin: 0 auto; padding: 2rem 1.75rem 4rem; }}
.icon-btn {{ display: inline-flex; align-items: center; justify-content: center; flex: none; height: 28px; min-width: 28px; padding: 0 .35rem; background: transparent; color: var(--muted); border: 0; border-radius: 6px; font-size: .95rem; line-height: 1; cursor: pointer; }}
.icon-btn:hover {{ color: var(--accent); background: var(--hover); }}
kbd {{ font: 11px/1 var(--mono); color: var(--muted); background: var(--bg); border: 1px solid var(--border); border-bottom-width: 2px; border-radius: 4px; padding: .15rem .35rem; }}

/* sidebar: header + search stay pinned, the tree scrolls */
.sidebar {{ position: fixed; top: 0; bottom: 0; left: 0; width: var(--sb-w); z-index: 65; display: flex; flex-direction: column; background: var(--side); border-right: 1px solid var(--border); font-size: .875rem; transition: transform .2s ease; }}
[data-sidebar="hidden"] .sidebar {{ transform: translateX(-100%); }}
.sb-scroll {{ flex: 1; min-height: 0; overflow-y: auto; padding: .25rem .5rem 3rem; }}
.content {{ margin-left: var(--sb-w); transition: margin-left .2s ease; }}
[data-sidebar="hidden"] .content {{ margin-left: 0; }}
.sb-open {{ position: fixed; top: .6rem; left: .6rem; z-index: 60; display: none; background: var(--card); border: 1px solid var(--border); }}
[data-sidebar="hidden"] .sb-open {{ display: inline-flex; }}
.sb-resizer {{ position: fixed; top: 0; bottom: 0; left: calc(var(--sb-w) - 3px); width: 6px; z-index: 66; cursor: col-resize; }}
.sb-resizer::after {{ content: ""; position: absolute; top: 0; bottom: 0; left: 2px; width: 2px; background: transparent; transition: background .15s; }}
.sb-resizer:hover::after, .resizing .sb-resizer::after {{ background: var(--accent); }}
[data-sidebar="hidden"] .sb-resizer {{ display: none; }}
.resizing, .resizing * {{ cursor: col-resize !important; user-select: none !important; }}
.resizing .sidebar, .resizing .content {{ transition: none; }}
.scrim {{ display: none; }}
@media (max-width: 800px) {{
  .content {{ margin-left: 0; }}
  main {{ padding-top: 3.25rem; }}
  .sb-resizer {{ display: none; }}
  [data-sidebar="shown"] .sidebar {{ box-shadow: 6px 0 24px rgba(0,0,0,.35); }}
  [data-sidebar="shown"] .scrim {{ display: block; position: fixed; inset: 0; z-index: 64; background: rgba(0,0,0,.35); }}
}}
.sb-head {{ display: flex; align-items: center; gap: .1rem; padding: .55rem .5rem .35rem .75rem; }}
.sb-project {{ display: flex; align-items: center; gap: .5rem; flex: 1; min-width: 0; color: var(--fg); font-weight: 600; font-size: .95rem; text-decoration: none; }}
.sb-project span {{ overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
.sb-project:hover {{ color: var(--accent); }}
.sb-project img {{ width: 18px; height: 18px; flex: none; }}
.sb-search {{ position: relative; margin: .25rem .6rem .6rem; }}
.sb-search input {{ width: 100%; padding: .35rem 2rem .35rem .6rem; color: var(--fg); background: var(--card); border: 1px solid var(--border); border-radius: 6px; font: inherit; font-size: .85rem; }}
.sb-search input:focus {{ outline: none; border-color: var(--accent); }}
.sb-search input::-webkit-search-cancel-button {{ display: none; }}
.sb-search kbd {{ position: absolute; right: .4rem; top: 50%; transform: translateY(-50%); pointer-events: none; }}
.sb-search input:focus + kbd {{ display: none; }}
.sb-pad {{ padding: 0 .5rem; }}
.tree summary {{ display: flex; align-items: center; gap: .35rem; padding: .15rem .4rem; border-radius: 5px; white-space: nowrap; }}
.tree summary:hover {{ background: var(--hover); }}
.tree summary::before {{ flex: none; width: .9em; font-size: .8em; }}
.tree .t-name {{ overflow: hidden; text-overflow: ellipsis; }}
.tree .count {{ margin-left: auto; padding-left: .5rem; font-size: .72rem; }}
.t-module {{ margin-top: .3rem; }}
.t-module > summary {{ font-weight: 600; }}
.t-module > summary .t-name {{ font-family: var(--mono); font-size: .84rem; }}
.t-provider, .t-group {{ margin-left: .8rem; }}
.t-provider > summary .t-name {{ font-weight: 500; }}
.t-group > summary {{ color: var(--muted); font-size: .8rem; }}
.tree ul {{ margin: 0 0 .1rem 1.05rem; padding: 0 0 0 .1rem; border-left: 1px solid var(--border); }}
.tree li {{ padding: 0; }}
.tree li a, .tree .t-leaf {{ display: block; padding: .12rem .5rem; border: 0; border-radius: 5px; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }}
.tree li a:hover {{ color: var(--accent); background: var(--hover); }}
.tree li a.active {{ color: var(--accent); background: color-mix(in srgb, var(--accent) 15%, transparent); font-weight: 600; }}
.tree li a.dim, .tree .t-leaf {{ color: var(--muted); }}
.tree li a.ext::after {{ content: " ↗"; color: var(--muted); font-size: .75em; }}
.t-empty {{ margin: 0 0 .1rem 2.2rem; color: var(--muted); font-size: .78rem; font-style: italic; }}
.pill {{ flex: none; font-size: .62rem; font-weight: 700; letter-spacing: .05em; text-transform: uppercase; line-height: 1.5; padding: 0 .45rem; border: 1px solid currentColor; border-radius: 999px; }}
.pill.ok {{ color: #22c55e; }}
.pill.warn {{ color: #eab308; }}
.pill.off {{ color: var(--muted); }}

/* configuration */
.cfg-dialog {{ width: min(720px, calc(100vw - 2rem)); max-height: calc(100vh - 4rem); padding: 0; color: var(--fg); background: var(--bg); border: 1px solid var(--border); border-radius: 10px; box-shadow: 0 24px 64px rgba(0,0,0,.45); }}
.cfg-dialog::backdrop {{ background: rgba(0,0,0,.45); backdrop-filter: blur(2px); }}
.dlg-bar {{ position: sticky; top: 0; z-index: 1; display: flex; align-items: center; justify-content: space-between; padding: .45rem .5rem .45rem 1rem; background: var(--bg); border-bottom: 1px solid var(--border); font-weight: 600; }}
#cfg-body {{ padding: 1rem; }}
.cfg code {{ font: .8rem var(--mono); background: var(--code-bg); border: 1px solid var(--border); border-radius: 4px; padding: .05rem .35rem; word-break: break-all; }}
.cfg-path {{ color: var(--muted); font-size: .8rem; margin-bottom: .8rem; }}
.cfg-provider {{ background: var(--card); border: 1px solid var(--border); border-radius: 8px; margin: 0 0 .75rem; overflow: hidden; }}
.cfg-provider > header {{ display: flex; align-items: center; gap: .5rem; padding: .5rem .8rem; }}
.cfg-provider > header .pill {{ margin-left: auto; }}
.cfg-kind {{ color: var(--muted); font-size: .8rem; font-family: var(--mono); }}
.cfg-row {{ display: grid; grid-template-columns: 150px 1fr; gap: .75rem; padding: .4rem .8rem; border-top: 1px solid var(--border); font-size: .85rem; }}
.cfg-row .k {{ color: var(--muted); font-family: var(--mono); font-size: .8rem; padding-top: .1rem; }}
.cfg-row .v {{ display: flex; flex-wrap: wrap; align-items: center; gap: .3rem; min-width: 0; }}
.chip {{ font-size: .78rem; padding: 0 .45rem; border: 1px solid var(--border); border-radius: 4px; }}
.chip.off {{ color: var(--muted); text-decoration: line-through; opacity: .7; }}
.env {{ font-size: .68rem; font-weight: 700; text-transform: uppercase; letter-spacing: .04em; }}
.env.ok {{ color: #22c55e; }}
.env.missing {{ color: #ef4444; }}
.redacted {{ color: var(--muted); font-style: italic; }}
.cfg-raw > summary {{ color: var(--muted); font-size: .85rem; margin-top: .25rem; }}
.cfg-raw pre {{ background: var(--code-bg); border: 1px solid var(--border); border-radius: 6px; padding: .8rem 1rem; overflow: auto; }}
.cfg-raw pre code {{ border: 0; padding: 0; background: none; word-break: normal; }}
h1 {{ font-size: 1.5rem; margin: 0 0 1.5rem; }}
.brand {{ color: var(--accent); }}
h2 {{ font-size: 1.05rem; color: var(--accent); margin: 0; display: inline; }}
h3 {{ font-size: .95rem; margin: 0; display: inline; }}
h4 {{ font-size: .8rem; text-transform: uppercase; letter-spacing: .05em; color: var(--muted); margin: .9rem 0 .3rem; }}
h4.project {{ color: var(--accent); text-transform: none; letter-spacing: 0; font-size: .9rem; }}
.count {{ color: var(--muted); font-weight: 400; font-size: .8rem; }}
details.provider {{ margin: 1.25rem 0; padding: .75rem 1.1rem; background: var(--card); border: 1px solid var(--border); border-radius: 10px; }}
details.provider[open] > summary {{ border-bottom: 1px solid var(--border); padding-bottom: .5rem; margin-bottom: .75rem; }}
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
.markdown img {{ max-width: 100%; height: auto; }}
.markdown blockquote {{ border-left: 3px solid var(--accent); margin: 1rem 0; padding: .1rem 1rem; color: var(--muted); }}
.markdown table {{ border-collapse: collapse; }}
.markdown th, .markdown td {{ border: 1px solid var(--border); padding: .35rem .6rem; }}
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
.goto {{ position: fixed; bottom: 1.25rem; right: 1.25rem; z-index: 50; display: flex; flex-direction: column; align-items: flex-end; gap: .5rem; }}
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
.copy-path {{ background: var(--card); color: var(--fg); border: 1px solid var(--border); border-radius: 6px; padding: .5rem .8rem; font-size: .85rem; font-weight: 600; cursor: pointer; line-height: 1; box-shadow: 0 2px 8px rgba(0,0,0,.3); }}
.copy-path:hover {{ border-color: var(--accent); color: var(--accent); }}
.copy-path.copied {{ border-color: var(--accent); color: var(--accent); }}
</style>
</head>
<body>
<button class="icon-btn sb-open" onclick="bancoToggleSidebar()" title="Show sidebar" aria-label="Show sidebar" aria-controls="sidebar">{PANEL_SVG}</button>
<aside class="sidebar" id="sidebar">{sidebar}</aside>
<div class="sb-resizer" id="sb-resizer" title="Drag to resize · double-click to reset" role="separator" aria-orientation="vertical"></div>
<div class="scrim" onclick="bancoToggleSidebar()"></div>
<div class="content"><main>{body}</main></div>
<dialog id="cfg-dialog" class="cfg-dialog">
  <div class="dlg-bar"><span>Configuration</span><button class="icon-btn" onclick="this.closest('dialog').close()" aria-label="Close">✕</button></div>
  <div id="cfg-body"></div>
</dialog>
<script>
function bancoToggleSidebar() {{
  var el = document.documentElement;
  var next = el.getAttribute("data-sidebar") === "hidden" ? "shown" : "hidden";
  el.setAttribute("data-sidebar", next);
  // Only remember the choice on wide screens, where the sidebar docks.
  if (!matchMedia("(max-width: 800px)").matches) {{
    try {{ localStorage.setItem("banco-sidebar", next); }} catch (e) {{}}
  }}
}}

// Sidebar width: drag the right edge; double-click resets to the default.
(function () {{
  var handle = document.getElementById("sb-resizer");
  var root = document.documentElement;
  var KEY = "banco-sidebar-width";
  function clamp(w) {{ return Math.max(180, Math.min(640, w, innerWidth - 320)); }}
  handle.addEventListener("pointerdown", function (e) {{
    e.preventDefault();
    handle.setPointerCapture(e.pointerId);
    root.classList.add("resizing");
    function move(ev) {{ root.style.setProperty("--sb-w", clamp(ev.clientX) + "px"); }}
    function up(ev) {{
      handle.removeEventListener("pointermove", move);
      handle.removeEventListener("pointerup", up);
      handle.removeEventListener("pointercancel", up);
      root.classList.remove("resizing");
      try {{ localStorage.setItem(KEY, String(parseInt(getComputedStyle(root).getPropertyValue("--sb-w"), 10))); }} catch (e) {{}}
    }}
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", up);
    handle.addEventListener("pointercancel", up);
  }});
  handle.addEventListener("dblclick", function () {{
    root.style.removeProperty("--sb-w");
    try {{ localStorage.removeItem(KEY); }} catch (e) {{}}
  }});
}})();

function bancoShowConfig() {{
  var dlg = document.getElementById("cfg-dialog");
  var body = document.getElementById("cfg-body");
  body.innerHTML = '<p class="muted">Loading…</p>';
  dlg.showModal();
  fetch("/config?partial=1")
    .then(function (r) {{ if (!r.ok) throw new Error(r.status); return r.text(); }})
    .then(function (html) {{ body.innerHTML = html; }})
    .catch(function () {{ body.innerHTML = '<p class="err">Could not load configuration.</p>'; }});
}}
document.getElementById("cfg-dialog").addEventListener("click", function (e) {{
  if (e.target === this) this.close();  // click on the backdrop
}});

// Sidebar tree: restore per-node open state, highlight the current page and
// expand its ancestors, and keep the scroll position across navigation.
(function () {{
  var sb = document.getElementById("sidebar");
  var scroller = document.getElementById("sb-scroll") || sb;
  var KEY = "banco-tree";
  var state;
  try {{ state = JSON.parse(localStorage.getItem(KEY) || "{{}}"); }} catch (e) {{ state = {{}}; }}
  sb.querySelectorAll("details[data-tree]").forEach(function (d) {{
    var id = d.getAttribute("data-tree");
    if (Object.prototype.hasOwnProperty.call(state, id)) d.open = !!state[id];
  }});
  // Persist on user clicks only, so auto-expanding the active path isn't saved.
  sb.addEventListener("click", function (e) {{
    var s = e.target.closest("summary");
    if (!s || !sb.contains(s)) return;
    var d = s.parentElement;
    state[d.getAttribute("data-tree")] = !d.open;  // runs before the toggle
    try {{ localStorage.setItem(KEY, JSON.stringify(state)); }} catch (e) {{}}
  }});

  function norm(h) {{ try {{ return decodeURIComponent(h); }} catch (e) {{ return h; }} }}
  var here = norm(location.pathname + location.search);
  var active = null;
  sb.querySelectorAll(".tree a[href]").forEach(function (a) {{
    if (norm(a.getAttribute("href")) === here) {{ a.classList.add("active"); active = a; }}
  }});
  if (active) {{
    for (var p = active.parentElement; p && p !== sb; p = p.parentElement) {{
      if (p.tagName === "DETAILS") p.open = true;
    }}
  }}

  try {{
    var y = sessionStorage.getItem("banco-sb-scroll");
    if (y) scroller.scrollTop = +y;
  }} catch (e) {{}}
  if (active) {{
    var r = active.getBoundingClientRect(), sr = scroller.getBoundingClientRect();
    if (r.top < sr.top || r.bottom > sr.bottom) active.scrollIntoView({{ block: "center" }});
  }}
  addEventListener("pagehide", function () {{
    try {{ sessionStorage.setItem("banco-sb-scroll", String(scroller.scrollTop)); }} catch (e) {{}}
  }});
}})();
</script>
<script>
function bancoApplyThemeLabel() {{
  var t = document.documentElement.getAttribute("data-theme");
  var b = document.querySelector(".theme-toggle");
  if (b) {{
    b.textContent = t === "light" ? "☾" : "☀";
    b.title = t === "light" ? "Switch to dark theme" : "Switch to light theme";
  }}
}}
function bancoToggleTheme() {{
  var t = document.documentElement.getAttribute("data-theme") === "light" ? "dark" : "light";
  document.documentElement.setAttribute("data-theme", t);
  try {{ localStorage.setItem("banco-theme", t); }} catch (e) {{}}
  bancoApplyThemeLabel();
}}
bancoApplyThemeLabel();

function bancoCopyPath(btn) {{
  var path = btn.getAttribute("data-path");
  function done() {{
    var original = btn.textContent;
    btn.textContent = "Copied";
    btn.classList.add("copied");
    setTimeout(function () {{ btn.textContent = original; btn.classList.remove("copied"); }}, 1200);
  }}
  if (navigator.clipboard && navigator.clipboard.writeText) {{
    navigator.clipboard.writeText(path).then(done, function () {{ bancoFallbackCopy(path, done); }});
  }} else {{
    bancoFallbackCopy(path, done);
  }}
}}
function bancoFallbackCopy(text, done) {{
  var ta = document.createElement("textarea");
  ta.value = text;
  ta.style.position = "fixed";
  ta.style.opacity = "0";
  document.body.appendChild(ta);
  ta.select();
  try {{ document.execCommand("copy"); done(); }} catch (e) {{}}
  document.body.removeChild(ta);
}}

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
  // The search page has its own live input; elsewhere use the sidebar field,
  // revealing the sidebar first if it is hidden.
  var input = document.getElementById("q") || document.getElementById("sb-q");
  if (!input) return;
  e.preventDefault();
  if (input.id === "sb-q" && document.documentElement.getAttribute("data-sidebar") === "hidden") bancoToggleSidebar();
  input.focus();
  input.select();
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

fn svg_response(body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(body.to_string()).with_header(
        Header::from_bytes(&b"Content-Type"[..], &b"image/svg+xml"[..]).unwrap(),
    )
}

fn bytes_response(body: Vec<u8>, content_type: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(body).with_header(
        Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap(),
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

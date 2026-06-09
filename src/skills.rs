use std::path::{Path, PathBuf};

/// A skill bundled into the banco binary. The `SKILL.md` body is embedded at
/// compile time (like `banco_template.md`) and written out on install.
pub struct BundledSkill {
    pub name: &'static str,
    pub skill_md: &'static str,
}

/// All skills shipped with banco.
pub const BUNDLED: &[BundledSkill] = &[
    BundledSkill {
        name: "hello-world",
        skill_md: include_str!("skills/hello-world/SKILL.md"),
    },
    BundledSkill {
        name: "tidy",
        skill_md: include_str!("skills/tidy/SKILL.md"),
    },
];

/// Agents banco can install skills for, mapped to the directory each one
/// discovers skills from (relative to the project root).
pub fn skills_dir(agent: &str) -> Option<&'static str> {
    match agent {
        "claude" => Some(".claude/skills"),
        _ => None,
    }
}

pub const SUPPORTED_AGENTS: &[&str] = &["claude"];

/// Install every bundled skill into the given agent's skills directory.
/// Idempotent: re-running overwrites the `SKILL.md` files so they stay current.
pub fn install(root: &Path, agent: &str) -> anyhow::Result<Vec<String>> {
    let dir = skills_dir(agent).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown agent '{}'; supported: {}",
            agent,
            SUPPORTED_AGENTS.join(", ")
        )
    })?;
    let base = root.join(dir);

    let mut installed = Vec::new();
    for skill in BUNDLED {
        let skill_dir = base.join(skill.name);
        std::fs::create_dir_all(&skill_dir)?;
        std::fs::write(skill_dir.join("SKILL.md"), skill.skill_md)?;
        installed.push(skill.name.to_string());
    }
    Ok(installed)
}

/// Names of skills currently installed for the given agent (directories under
/// the agent's skills dir that contain a `SKILL.md`).
pub fn installed(root: &Path, agent: &str) -> Vec<String> {
    let Some(dir) = skills_dir(agent) else {
        return Vec::new();
    };
    let base: PathBuf = root.join(dir);
    let Ok(entries) = std::fs::read_dir(&base) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.join("SKILL.md").is_file())
        .filter_map(|p| p.file_name().and_then(|s| s.to_str()).map(String::from))
        .collect();
    names.sort();
    names
}

//! Phase 2c: curate gentle/Pi after installing it (Option A — by files).
//!
//! devlab owns the environment and the agent (gentle-ai + Pi). Skills are
//! universal content, owned by the `bit-crew/prim-skills` repo and installed by
//! its own binary `prim-skills-install` — devlab does NOT copy skills anymore;
//! it delegates to that installer.
//!
//! What devlab still curates here (specific to devlab, not universal skills):
//!   - concatenate OUR instructions into Pi's AGENTS.md (steering),
//!   - keep Engram LOCAL only (no Engram Cloud),
//!   - refresh the skill registry.
//!
//! The instructions live in prim-skills (cloned to a temp dir for this), but the
//! act of injecting them into Pi's AGENTS.md is devlab's own steering.
//!
//! Non-fatal: logs and continues, so a partial install does not abort.

use std::path::{Path, PathBuf};
use std::process::Command;

const BLUE: &str = "\x1b[1;34m";
const RESET: &str = "\x1b[0m";

fn note(msg: &str) {
    println!("{BLUE}[gentle]{RESET} {msg}");
}

/// Content repo cloned at install time (for the instructions / steering).
const SKILLS_REPO: &str = "https://github.com/bit-crew/prim-skills.git";

/// Pi's agent config dir, where AGENTS.md lives.
fn pi_agent_dir(home: &Path) -> PathBuf {
    home.join(".pi").join("agent")
}

/// Run the full curation. Called after gentle-ai is installed (VPS profile).
pub fn curate() {
    let Some(home) = dirs::home_dir() else {
        note("sin HOME; se omite la curaduria.");
        return;
    };

    // Skills are installed by prim-skills' own binary (unified in ~/.agents/skills).
    install_skills_via_prim();

    // Clone the content repo to a temp dir for the instructions (steering).
    if let Some(content) = clone_content() {
        append_our_instructions(&content, &pi_agent_dir(&home));
        let _ = std::fs::remove_dir_all(&content);
    } else {
        note("no se pudo clonar prim-skills para instrucciones (continuo).");
    }

    ensure_engram_local();
    refresh_index();

    note("Curaduria de Pi/gentle completa.");
}

/// Delegate skill installation to prim-skills-install (own + external skills,
/// unified in ~/.agents/skills). Downloads and runs its bootstrap; non-fatal.
fn install_skills_via_prim() {
    note("Instalando skills via prim-skills-install...");
    let Some(script) =
        curl_text("https://raw.githubusercontent.com/bit-crew/prim-skills/main/install.sh")
    else {
        note("no se pudo descargar install.sh de prim-skills (continuo)");
        return;
    };
    if sh_stdin(&script, &[]) {
        note("Skills instaladas via prim-skills.");
    } else {
        note("fallo prim-skills-install (continuo)");
    }
}

/// Clone prim-skills into a fresh temp dir. Returns the clone path on success.
fn clone_content() -> Option<PathBuf> {
    let base = std::env::temp_dir().join(format!("prim-skills-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    note("Clonando prim-skills (contenido agnostico)...");
    let ok = Command::new("git")
        .args(["clone", "--depth", "1", SKILLS_REPO])
        .arg(&base)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Some(base)
    } else {
        None
    }
}

/// Append our always-on instructions to Pi's AGENTS.md. Strips the YAML
/// frontmatter of each file and resolves the ODD memory marker (Engram is on).
fn append_our_instructions(content: &Path, agent_dir: &Path) {
    // Order matters; mirror manifest.toml always-on instructions.
    let files = ["tech.md", "code-conventions.md", "odd.md"];
    let mut body = String::from("\n<!-- Managed by devlab-install. Source: prim-skills -->\n\n");
    for f in files {
        match std::fs::read_to_string(content.join("instructions").join(f)) {
            Ok(raw) => {
                let text = strip_frontmatter(&raw);
                let text = text.replace("<!-- ODD_MEMORY -->\n", ODD_MEMORY_STEPS);
                body.push_str(&text);
                body.push_str("\n\n");
            }
            Err(_) => note(&format!("instruccion {f} no encontrada (continuo)")),
        }
    }
    // visual-style is conditional (html/md); append under a labeled section.
    if let Ok(raw) = std::fs::read_to_string(content.join("instructions").join("visual-style.md")) {
        body.push_str("---\n\n# Conditional guidance (applies to *.html, *.md)\n\n");
        body.push_str(&strip_frontmatter(&raw));
        body.push_str("\n\n");
    }

    let path = agent_dir.join("AGENTS.md");
    let _ = std::fs::create_dir_all(agent_dir);
    // Append to whatever gentle already wrote, don't clobber it.
    let mut existing = std::fs::read_to_string(&path).unwrap_or_default();
    existing.push_str(&body);
    if std::fs::write(&path, existing).is_ok() {
        note(&format!(
            "instrucciones propias anexadas a {}",
            path.display()
        ));
    }
}

/// Memory steps injected into ODD (Engram local is on).
const ODD_MEMORY_STEPS: &str = "\
   Before exploring, query your persistent memory (Engram) for relevant prior\n\
   decisions, conventions and bugs, so context accumulates across sessions\n\
   instead of resetting. After significant work, save the decision or learning.\n\n";

/// Keep Engram LOCAL only. We deliberately do NOT set ENGRAM_CLOUD_* or enable
/// autosync; local is Engram's default. This is an intent assertion.
fn ensure_engram_local() {
    note("Engram en modo local (sin Engram Cloud).");
}

/// Regenerate gentle's skill index so it reflects the curated set.
fn refresh_index() {
    if Command::new("gentle-ai")
        .args(["--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        note("Regenerando indice de skills (gentle-ai skill-registry refresh)...");
        let _ = Command::new("gentle-ai")
            .args(["skill-registry", "refresh", "--force"])
            .status();
    }
}

// --- helpers ----------------------------------------------------------------

/// Strip a leading YAML frontmatter block (--- ... ---) if present.
fn strip_frontmatter(s: &str) -> String {
    let t = s.trim_start();
    if let Some(rest) = t.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            return rest[end + 4..].trim_start().to_string();
        }
    }
    s.to_string()
}

/// Recursively copy a directory tree.
/// Fetch a URL's body as text (install.sh of prim-skills). IPv4 forced to avoid
/// hanging on hosts with broken IPv6 (same issue hit for Go downloads).
fn curl_text(url: &str) -> Option<String> {
    let out = Command::new("curl")
        .args(["-4", "-fsSL", url])
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}

/// Run `sh` feeding `script` on stdin. Returns success as bool.
fn sh_stdin(script: &str, args: &[&str]) -> bool {
    use std::io::Write as _;
    use std::process::Stdio;
    let child = Command::new("sh").args(args).stdin(Stdio::piped()).spawn();
    match child {
        Ok(mut c) => {
            if let Some(mut stdin) = c.stdin.take() {
                let _ = stdin.write_all(script.as_bytes());
            }
            c.wait().map(|s| s.success()).unwrap_or(false)
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::strip_frontmatter;

    #[test]
    fn strips_leading_frontmatter() {
        let input = "---\nname: tech\ninclusion: always\n---\n\n# Technology Stack\n\nPolars.";
        let got = strip_frontmatter(input);
        assert!(got.starts_with("# Technology Stack"));
        assert!(!got.contains("inclusion"));
    }

    #[test]
    fn leaves_body_without_frontmatter_untouched() {
        let input = "# Just a heading\n\nno frontmatter here";
        assert_eq!(strip_frontmatter(input), input);
    }
}

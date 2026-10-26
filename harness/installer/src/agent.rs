//! Agent runtime install + curation (moved here from rig).
//!
//! Harness owns the agent: it installs the runtime (Google ADK, Pi, gentle-ai)
//! and configures it (Pi settings fix, instructions into AGENTS.md, Engram
//! local, skill-registry refresh). Skills themselves are installed by the
//! harness skill path (install_own_skills / install_external_skills in main.rs),
//! which the curation here calls IN-PROCESS — no `curl | sh` of install.sh.
//!
//! Non-fatal throughout: logs and continues so a partial install does not abort.

use std::path::{Path, PathBuf};
use std::process::Command;

const BLUE: &str = "\x1b[1;34m";
const RESET: &str = "\x1b[0m";

fn note(msg: &str) {
    println!("{BLUE}[agent]{RESET} {msg}");
}

/// The monorepo, cloned (sparse) for the instructions / steering.
const CONTENT_REPO: &str = "https://github.com/bit-crew/prim.git";
const INSTRUCTIONS_SUBPATH: &str = "harness/instructions";

/// Install the agent runtime: Google ADK (uv), Pi, gentle-ai. uv is ensured by
/// the caller (main.rs) before this runs.
pub fn install_runtime() {
    install_adk();
    install_pi();
    install_gentle();
    configure_pi_settings();
}

/// Google Agent Development Kit (ADK) — installed globally via uv (a2a extra).
fn install_adk() {
    if !which("uv") || which("adk") {
        return;
    }
    note("Instalando google-adk (uv tool, con extra a2a)...");
    if !run_ok("uv", &["tool", "install", "google-adk[a2a]"]) {
        note("fallo uv tool install google-adk (continuo)");
    }
}

/// Pi — the base coding agent (gentle-ai refuses to run without it), so Pi
/// installs before gentle-ai. Official installer drops `pi` into ~/.local/bin.
fn install_pi() {
    if which("pi") {
        return;
    }
    note("Instalando Pi (install.sh oficial)...");
    let Some(script) = curl_text("https://pi.dev/install.sh") else {
        note("no se pudo descargar install.sh de Pi (continuo)");
        return;
    };
    if sh_stdin(&script) {
        note("Pi instalado.");
    } else {
        note("fallo instalacion de Pi (continuo)");
    }
}

/// gentle-ai — the agent configurator for the Gentleman ecosystem. Installed via
/// its bash-specific install script, then configured non-interactively for Pi.
fn install_gentle() {
    if !which("gentle-ai") {
        note("Instalando gentle-ai (install.sh oficial)...");
        let Some(script) = curl_text(
            "https://raw.githubusercontent.com/Gentleman-Programming/gentle-ai/main/scripts/install.sh",
        ) else {
            note("no se pudo descargar install.sh de gentle-ai (continuo)");
            return;
        };
        // gentle-ai's installer is bash-specific; pipe it to bash.
        if !bash_stdin(&script) {
            note("fallo instalacion de gentle-ai (continuo)");
            return;
        }
    }
    if !which("gentle-ai") {
        note("gentle-ai no quedo en PATH; se omite setup (continuo)");
        return;
    }
    // Non-interactive setup for the Pi agent (gentle-shell).
    note("Configurando gentle-ai para Pi (gentle-shell)...");
    let ok = Command::new("gentle-ai")
        .args([
            "install",
            "--agent",
            "pi",
            "--pi-background-subagents",
            "on",
            "--channel",
            "stable",
        ])
        .env("GENTLE_AI_INSTALL_SCOPE", "global")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        note("gentle-ai configurado para Pi.");
    } else {
        note("fallo gentle-ai install --agent pi (continuo)");
    }
}

/// Post-install Pi settings fix. gentle-ai installs `gentle-pi`, whose `codemode`
/// tool collides with Pi's built-in `codemode`; write `-builtin:codemode` into
/// ~/.pi/agent/settings.json ahead of time. Non-fatal.
fn configure_pi_settings() {
    let Some(home) = dirs::home_dir() else { return };
    let settings = pi_agent_dir(&home).join("settings.json");
    let Ok(raw) = std::fs::read_to_string(&settings) else {
        return; // no settings yet; Pi will resolve on first launch
    };
    let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };
    let obj = match json.as_object_mut() {
        Some(o) => o,
        None => return,
    };
    let disabled = obj
        .entry("disabledExtensions")
        .or_insert_with(|| serde_json::Value::Array(vec![]));
    if let Some(arr) = disabled.as_array_mut() {
        let marker = serde_json::Value::String("-builtin:codemode".into());
        if !arr.contains(&marker) {
            arr.push(marker);
            if let Ok(out) = serde_json::to_string_pretty(&json) {
                if std::fs::write(&settings, out).is_ok() {
                    note(
                        "Pi settings: codemode builtin desactivado (evita colision con gentle-pi).",
                    );
                }
            }
        }
    }
}

// --- curation ---------------------------------------------------------------

/// Pi's agent config dir, where AGENTS.md lives.
fn pi_agent_dir(home: &Path) -> PathBuf {
    home.join(".pi").join("agent")
}

/// Curate the agent after installing it: inject our instructions into AGENTS.md,
/// keep Engram local, refresh the skill index. `install_skills` is a callback so
/// the skills are installed IN-PROCESS by the harness (no curl|sh recursion).
pub fn curate(install_skills: impl FnOnce()) {
    let Some(home) = dirs::home_dir() else {
        note("sin HOME; se omite la curaduria.");
        return;
    };

    // Skills are installed in-process by the harness skill path.
    install_skills();

    if let Some(content) = clone_content() {
        append_our_instructions(&content, &pi_agent_dir(&home));
        let _ = std::fs::remove_dir_all(&content);
    } else {
        note("no se pudo clonar prim para instrucciones (continuo).");
    }

    ensure_engram_local();
    refresh_index();

    note("Curaduria del agente completa.");
}

/// Sparse-clone only the instructions subpath of the monorepo into a temp dir.
fn clone_content() -> Option<PathBuf> {
    let base = std::env::temp_dir().join(format!("prim-content-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    note("Clonando prim (instrucciones, sparse)...");
    let cloned = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--filter=blob:none",
            "--sparse",
            CONTENT_REPO,
        ])
        .arg(&base)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !cloned {
        return None;
    }
    let sparse = Command::new("git")
        .args(["sparse-checkout", "set", INSTRUCTIONS_SUBPATH])
        .current_dir(&base)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if sparse {
        Some(base)
    } else {
        None
    }
}

/// Append our always-on instructions to Pi's AGENTS.md (strip frontmatter,
/// resolve the ODD memory marker). Appends; never clobbers gentle's content.
fn append_our_instructions(content: &Path, agent_dir: &Path) {
    let files = ["tech.md", "code-conventions.md", "odd.md"];
    let mut body = String::from("\n<!-- Managed by harness-install. Source: prim/harness -->\n\n");
    let instr = content.join(INSTRUCTIONS_SUBPATH);
    for f in files {
        match std::fs::read_to_string(instr.join(f)) {
            Ok(raw) => {
                let text = strip_frontmatter(&raw);
                let text = text.replace("<!-- ODD_MEMORY -->\n", ODD_MEMORY_STEPS);
                body.push_str(&text);
                body.push_str("\n\n");
            }
            Err(_) => note(&format!("instruccion {f} no encontrada (continuo)")),
        }
    }
    if let Ok(raw) = std::fs::read_to_string(instr.join("visual-style.md")) {
        body.push_str("---\n\n# Conditional guidance (applies to *.html, *.md)\n\n");
        body.push_str(&strip_frontmatter(&raw));
        body.push_str("\n\n");
    }

    let path = agent_dir.join("AGENTS.md");
    let _ = std::fs::create_dir_all(agent_dir);
    let mut existing = std::fs::read_to_string(&path).unwrap_or_default();
    existing.push_str(&body);
    if std::fs::write(&path, existing).is_ok() {
        note(&format!(
            "instrucciones propias anexadas a {}",
            path.display()
        ));
    }
}

const ODD_MEMORY_STEPS: &str = "\
   Before exploring, query your persistent memory (Engram) for relevant prior\n\
   decisions, conventions and bugs, so context accumulates across sessions\n\
   instead of resetting. After significant work, save the decision or learning.\n\n";

/// Keep Engram LOCAL only (intent assertion; local is the default).
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

fn strip_frontmatter(s: &str) -> String {
    let t = s.trim_start();
    if let Some(rest) = t.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            return rest[end + 4..].trim_start().to_string();
        }
    }
    s.to_string()
}

/// Is `bin` on PATH? (adds .exe/.cmd on Windows for local dev/testing).
fn which(bin: &str) -> bool {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let exts: &[&str] = if cfg!(windows) {
        &["", ".exe", ".cmd", ".bat"]
    } else {
        &[""]
    };
    std::env::split_paths(&path).any(|dir| {
        exts.iter().any(|ext| {
            let mut p = dir.join(bin);
            if !ext.is_empty() {
                p.set_extension(&ext[1..]);
            }
            p.is_file()
        })
    })
}

fn run_ok(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Fetch a URL's body as text. IPv4 forced to avoid hanging on broken IPv6.
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

fn sh_stdin(script: &str) -> bool {
    pipe_to("sh", script)
}

fn bash_stdin(script: &str) -> bool {
    pipe_to("bash", script)
}

fn pipe_to(program: &str, script: &str) -> bool {
    use std::io::Write as _;
    use std::process::Stdio;
    match Command::new(program).stdin(Stdio::piped()).spawn() {
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

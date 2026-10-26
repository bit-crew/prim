//! prim — the umbrella orchestrator.
//!
//! A thin console selector: pick which components of the ecosystem to install
//! and it downloads and runs each component's own `install.sh` from the
//! monorepo. It does NOT bundle their logic — each component keeps its own
//! lightweight installer, invoked on demand. This honors the install matrix:
//! harness alone / rig+harness / core+harness / all.
//!
//! Each selected component's install.sh is fetched from raw.githubusercontent
//! and piped to `sh`. Components installed in order: rig (environment) first,
//! then harness (agent), then core — so the agent lands on a ready environment.

use std::io::{self, Write};
use std::process::{Command, Stdio};

/// Base raw URL of the monorepo's main branch.
const RAW_BASE: &str = "https://raw.githubusercontent.com/bit-crew/prim/main";

/// A selectable component.
struct Component {
    key: &'static str,
    name: &'static str,
    description: &'static str,
    /// Path to the component's install.sh inside the repo. None = not shippable yet.
    install_path: Option<&'static str>,
}

/// Order here is also the install order (rig → harness → core).
const COMPONENTS: &[Component] = &[
    Component {
        key: "rig",
        name: "Rig (dev environment)",
        description: "toolchains, Node, herdr, editor, dotfiles, VPS infra. No agent.",
        install_path: Some("rig/install.sh"),
    },
    Component {
        key: "harness",
        name: "Harness (agent)",
        description:
            "agent runtime (ADK/Pi/gentle-ai) + skills. Use --skills-only for skills alone.",
        install_path: Some("harness/install.sh"),
    },
    Component {
        key: "core",
        name: "Core (operating core)",
        description: "data, services, identity in your own accounts (not shippable yet).",
        install_path: None,
    },
];

fn main() {
    // Non-interactive shortcut: `prim <keys...>` (e.g. `prim rig harness`) or
    // `prim all` skips the menu. Otherwise show the interactive selector.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let chosen = if args.is_empty() {
        interactive_select()
    } else {
        resolve_selection(&args.join(" "))
    };

    if chosen.is_empty() {
        eprintln!("[prim] nothing selected");
        std::process::exit(1);
    }

    // Ask the harness install mode once, up front, if harness is selected.
    let harness_skills_only = if chosen.contains(&"harness") {
        ask_skills_only()
    } else {
        false
    };

    let mut failures = 0;
    for comp in COMPONENTS.iter().filter(|c| chosen.contains(&c.key)) {
        match comp.install_path {
            None => {
                eprintln!("[prim] {} aun no es instalable; se omite.", comp.key);
            }
            Some(path) => {
                let extra = if comp.key == "harness" && harness_skills_only {
                    vec!["--skills-only"]
                } else {
                    vec![]
                };
                println!("[prim] instalando {}...", comp.key);
                if !run_component(path, &extra) {
                    eprintln!("[prim] fallo instalando {} (continuo).", comp.key);
                    failures += 1;
                }
            }
        }
    }

    if failures > 0 {
        std::process::exit(1);
    }
    println!("[prim] listo.");
}

/// Show the menu and read the user's selection from stdin.
fn interactive_select() -> Vec<&'static str> {
    println!("prim — selector de instalacion\n");
    for (i, c) in COMPONENTS.iter().enumerate() {
        let tag = if c.install_path.is_none() {
            " (proximamente)"
        } else {
            ""
        };
        println!("  {}) {}{} — {}", i + 1, c.name, tag, c.description);
    }
    println!("  a) todo");
    print!("\nElige componentes (ej. 1 2, o 'a'): ");
    let _ = io::stdout().flush();

    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return vec![];
    }
    resolve_selection(line.trim())
}

/// Ask whether to install harness as agent runtime+skills or skills only.
fn ask_skills_only() -> bool {
    print!("[prim] harness: instalar el agente completo (ADK/Pi/gentle-ai) o solo skills? [completo/skills] (completo): ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return false;
    }
    matches!(
        line.trim().to_lowercase().as_str(),
        "skills" | "s" | "solo" | "skills-only"
    )
}

/// Map a raw selection string to component keys. 'a'/'all'/'todo' = everything
/// that is shippable; otherwise 1-based indices or component keys by name.
fn resolve_selection(input: &str) -> Vec<&'static str> {
    let lower = input.to_lowercase();
    if matches!(lower.as_str(), "a" | "all" | "todo") {
        return COMPONENTS.iter().map(|c| c.key).collect();
    }
    let mut out = Vec::new();
    for tok in lower.split([' ', ',']).filter(|s| !s.is_empty()) {
        if let Ok(n) = tok.parse::<usize>() {
            if n >= 1 && n <= COMPONENTS.len() {
                push_unique(&mut out, COMPONENTS[n - 1].key);
            }
        } else if let Some(c) = COMPONENTS.iter().find(|c| c.key == tok) {
            push_unique(&mut out, c.key);
        }
    }
    out
}

fn push_unique(v: &mut Vec<&'static str>, key: &'static str) {
    if !v.contains(&key) {
        v.push(key);
    }
}

/// Download a component's install.sh and run it with `sh`, passing `extra` args.
/// The script is fetched via curl and piped to `sh -s -- <extra>`.
fn run_component(install_path: &str, extra: &[&str]) -> bool {
    let url = format!("{RAW_BASE}/{install_path}");
    let script = match curl_text(&url) {
        Some(s) => s,
        None => {
            eprintln!("[prim] no se pudo descargar {url}");
            return false;
        }
    };
    let mut args = vec!["-s", "--"];
    args.extend_from_slice(extra);
    pipe_to_sh(&script, &args)
}

/// Fetch a URL body as text (IPv4 forced to avoid broken-IPv6 hangs).
fn curl_text(url: &str) -> Option<String> {
    let out = Command::new("curl")
        .args(["-4", "-fsSL", url])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Pipe `script` to `sh` with the given args, inheriting stdout/stderr/stdin so
/// the component installer's own prompts (e.g. rig's dc1/vps question) work.
fn pipe_to_sh(script: &str, args: &[&str]) -> bool {
    let child = Command::new("sh").args(args).stdin(Stdio::piped()).spawn();
    match child {
        Ok(mut c) => {
            if let Some(mut stdin) = c.stdin.take() {
                let _ = stdin.write_all(script.as_bytes());
                drop(stdin); // close so sh starts reading
            }
            c.wait().map(|s| s.success()).unwrap_or(false)
        }
        Err(_) => false,
    }
}

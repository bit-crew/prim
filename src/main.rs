//! prim — the umbrella orchestrator.
//!
//! A thin console selector turned lifecycle manager: `prim` picks which
//! components of the ecosystem to act on and invokes each component's own
//! installer, forwarding the verb (`install|update|uninstall`, default
//! install). Since P3 the split is: rig is activated over Nix directly
//! (src/nix.rs: home-manager against the global flake, prim-owned receipt),
//! while harness keeps its install.sh path. This honors the install
//! matrix: harness alone / rig+harness / core+harness / all. prim also
//! self-owns its binary (~/.local/bin/prim): the root install.sh puts it
//! there and `prim update` self-updates it from the repo's "latest"
//! release before updating components.
//!
//! S8 self-sufficiency: before installing rig (explicit `prim install`,
//! bare keys or the interactive selector), prim bootstraps the machine
//! itself — apt update/upgrade, official Nix single-user install and the
//! static identity file. `prim bootstrap` runs the same flow standalone.
//!
//! Each selected component's install.sh is fetched from raw.githubusercontent
//! and piped to `sh`. Components installed in order: rig (environment) first,
//! then harness (agent), then core — so the agent lands on a ready environment.
//! Optional components (S11b, e.g. files) never ride along 'all' installs:
//! they opt in per machine via explicit key or menu number.
//!
//! `prim doctor [--prune]` (P5) is read-only diagnostics of the Nix stack:
//! nix, rig receipt, flake reachability, store size, direnv GC roots and
//! pre-Nix leftovers (advisory only); `--prune` adds nix-collect-garbage.

use std::io::{self, Write};
use std::process::{Command, Stdio};

mod bootstrap;
mod doctor;
mod dotfiles;
mod lifecycle;
mod nix;
mod org;

/// Base raw URL of the monorepo's main branch.
const RAW_BASE: &str = "https://raw.githubusercontent.com/bit-crew/prim/main";

/// A selectable component.
#[derive(Clone, Copy)]
struct Component {
    key: &'static str,
    name: &'static str,
    description: &'static str,
    /// Path to the component's install.sh inside the repo. None = not shippable yet.
    install_path: Option<&'static str>,
    /// False = excluded from 'all'/'todo' selections (files: the rig vps
    /// flow chains it instead — S1/S2 of odd/tasks/rig-workstation-files.md;
    /// an explicit key or menu number still installs/repairs it).
    in_all: bool,
}

/// Order here is also the install order (rig → harness → core → files).
const COMPONENTS: &[Component] = &[
    Component {
        key: "rig",
        name: "Rig (dev environment)",
        description: "dev workstation: toolchains, herdr, editor, dotfiles, host infra + the ~/work SMB share (vps profile). No agent.",
        install_path: Some("install.sh"),
        in_all: true,
    },
    Component {
        key: "harness",
        name: "Harness (agent)",
        description:
            "agent runtime (ADK/Pi/gentle-ai) + skills. Use --skills-only for skills alone.",
        install_path: Some("harness/install.sh"),
        in_all: true,
    },
    Component {
        key: "core",
        name: "Core (operating core)",
        description: "data, services, identity in your own accounts (not shippable yet).",
        install_path: None,
        in_all: true,
    },
    Component {
        key: "files",
        name: "Files (SMB over Tailscale)",
        description: "workstation share of ~/work over SMB, tailnet-only. Installed with the rig (vps profile); explicit install repairs it.",
        install_path: Some("files/install.sh"),
        in_all: false,
    },
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // `prim org ...` — organization management, not an install component.
    if args.first().map(|a| a == "org").unwrap_or(false) {
        org::run(&args[1..]);
        return;
    }

    // Explicit verbs; anything else is the back-compat path: bare component
    // keys (`prim rig harness`) or the no-arg interactive selector.
    match args.first().map(String::as_str) {
        Some("bootstrap") => {
            if args.len() != 1 {
                eprintln!("uso: prim bootstrap  (sin argumentos)");
                std::process::exit(2);
            }
            if let Err(e) = bootstrap::run() {
                eprintln!("[prim] bootstrap fallo: {e}");
                std::process::exit(1);
            }
        }
        Some("install") => install_components(&args[1..]),
        Some("update") => {
            if args.len() != 1 {
                eprintln!("uso: prim update  (sin argumentos; actualiza todo lo instalado)");
                std::process::exit(2);
            }
            lifecycle::update();
        }
        Some("uninstall") => lifecycle::uninstall(&args[1..]),
        Some("status") => {
            if args.len() != 1 {
                eprintln!("uso: prim status");
                std::process::exit(2);
            }
            lifecycle::status();
        }
        // `prim doctor [--prune]` — read-only diagnostics; --prune adds
        // nix-collect-garbage (P5). Not a component: handled like org.
        Some("doctor") => match doctor::parse_args(&args[1..]) {
            Some(prune) => {
                if !doctor::run(prune) {
                    std::process::exit(1);
                }
            }
            None => {
                eprintln!("uso: prim doctor [--prune]");
                std::process::exit(2);
            }
        },
        _ => install_components(&args),
    }
}

/// Install flow (also the back-compat bare-keys path): resolve the selection
/// (accepting `--profile <p>` / `--profile=<p>` anywhere for rig), ask the
/// harness mode once, then run each selected component — rig over Nix, the
/// rest through their own install.sh.
fn install_components(args: &[String]) {
    let (args, profile_arg) = strip_profile_arg(args);
    let chosen = if args.is_empty() {
        interactive_select()
    } else {
        select_from(COMPONENTS, &args.join(" "))
    };

    if chosen.is_empty() {
        eprintln!("[prim] nothing selected");
        std::process::exit(1);
    }

    // S8: prim is self-sufficient — when rig is selected, bootstrap the
    // machine (apt, Nix, identity) once per invocation, before components.
    if chosen.contains(&"rig") {
        if let Err(e) = bootstrap::run() {
            eprintln!("[prim] bootstrap fallo: {e}");
            std::process::exit(1);
        }
    }

    // Ask the harness install mode once, up front, if harness is selected.
    let harness_skills_only = if chosen.contains(&"harness") {
        ask_skills_only()
    } else {
        false
    };

    let mut failures = 0;
    let mut chained_files = false;
    for comp in COMPONENTS.iter().filter(|c| chosen.contains(&c.key)) {
        // rig: prim speaks Nix directly (P3) — activate the flake and write
        // prim's own receipt; rig/install.sh is no longer invoked.
        if comp.key == "rig" {
            println!("[prim] instalando rig (nix)...");
            let profile = nix::resolve_profile(profile_arg.as_deref());
            if !nix::activate(&profile) || nix::write_receipt(&profile).is_err() {
                eprintln!("[prim] fallo instalando rig (continuo).");
                failures += 1;
                continue;
            }
            // Workstation share (S1/S2): the vps profile IS the workstation,
            // so the rig install configures the ~/work SMB share right after
            // the activation. Client profiles never serve SMB.
            if profile == "vps" {
                if nix::dry_run() {
                    println!("[prim] PRIM_DRY_RUN=1: no se ejecuta files; encadenaria 'files/install.sh' (share de ~/work).");
                } else {
                    println!("[prim] instalando files (share ~/work del workstation)...");
                    if !run_component("files/install.sh", &[]) {
                        eprintln!("[prim] fallo instalando files (continuo).");
                        failures += 1;
                    } else {
                        chained_files = true;
                    }
                }
            }
            continue;
        }
        // The rig chain already installed files; skip the explicit pass.
        if comp.key == "files" && chained_files {
            continue;
        }
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
    // Only the primary components are menu entries: files rides with the rig
    // (vps profile) and stays reachable by explicit key for repair.
    for (i, c) in COMPONENTS.iter().filter(|c| c.in_all).enumerate() {
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
    select_from(COMPONENTS, line.trim())
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

/// Map a raw selection string to keys from `list`. 'a'/'all'/'todo' = every
/// in_all component (optional ones like files are skipped, S11b); otherwise
/// 1-based indices (positions in `list`) or component keys by name. Used for
/// both the install menu (all components) and the uninstall picker
/// (installed components only).
fn select_from(list: &[Component], input: &str) -> Vec<&'static str> {
    let lower = input.to_lowercase();
    if matches!(lower.as_str(), "a" | "all" | "todo") {
        return list.iter().filter(|c| c.in_all).map(|c| c.key).collect();
    }
    let mut out = Vec::new();
    for tok in lower.split([' ', ',']).filter(|s| !s.is_empty()) {
        if let Ok(n) = tok.parse::<usize>() {
            if n >= 1 && n <= list.len() {
                push_unique(&mut out, list[n - 1].key);
            }
        } else if let Some(c) = list.iter().find(|c| c.key == tok) {
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

/// Remove `--profile <p>` (also `--profile=<p>`) from install args and
/// return the remaining args plus the profile value, if given.
fn strip_profile_arg(args: &[String]) -> (Vec<String>, Option<String>) {
    let mut rest = Vec::new();
    let mut profile = None;
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            profile = Some(arg.clone());
            skip_next = false;
        } else if arg == "--profile" {
            skip_next = true;
        } else if let Some(v) = arg.strip_prefix("--profile=") {
            profile = Some(v.to_string());
        } else {
            rest.push(arg.clone());
        }
    }
    (rest, profile)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Selection accepts indices, keys, 'all', mixed separators, and drops
    /// unknown tokens (the back-compat bare-keys contract).
    #[test]
    fn select_from_maps_tokens_to_keys() {
        assert_eq!(select_from(COMPONENTS, "1 2"), vec!["rig", "harness"]);
        assert_eq!(
            select_from(COMPONENTS, "rig,harness"),
            vec!["rig", "harness"]
        );
        assert_eq!(select_from(COMPONENTS, "RIG"), vec!["rig"]);
        assert_eq!(select_from(COMPONENTS, "a"), vec!["rig", "harness", "core"]);
        assert_eq!(select_from(COMPONENTS, "2 2"), vec!["harness"]);
        assert!(select_from(COMPONENTS, "zzz").is_empty());
        assert!(select_from(COMPONENTS, "9").is_empty());
    }

    /// Optional components (files, S11b): 'all'/'todo' skip them — a
    /// default install never grows the Samba share — while the bare key
    /// and the menu index still select them explicitly.
    #[test]
    fn select_from_all_excludes_optional_components() {
        assert_eq!(
            select_from(COMPONENTS, "all"),
            vec!["rig", "harness", "core"]
        );
        assert_eq!(
            select_from(COMPONENTS, "todo"),
            vec!["rig", "harness", "core"]
        );
        assert_eq!(select_from(COMPONENTS, "files"), vec!["files"]);
        assert_eq!(select_from(COMPONENTS, "4"), vec!["files"]);
    }

    /// --profile is accepted and stripped anywhere in install args, in both
    /// `--profile <p>` and `--profile=<p>` forms; everything else passes
    /// through untouched.
    #[test]
    fn strip_profile_arg_matrix() {
        fn s(v: &[&str]) -> Vec<String> {
            v.iter().map(|x| x.to_string()).collect()
        }
        let cases: Vec<(&str, Vec<&str>, Vec<&str>, Option<&str>)> = vec![
            (
                "split form after key",
                vec!["rig", "--profile", "dc1"],
                vec!["rig"],
                Some("dc1"),
            ),
            (
                "split form before keys",
                vec!["--profile", "vps", "rig", "harness"],
                vec!["rig", "harness"],
                Some("vps"),
            ),
            (
                "equals form",
                vec!["--profile=dc1", "rig"],
                vec!["rig"],
                Some("dc1"),
            ),
            (
                "no profile at all",
                vec!["rig", "harness"],
                vec!["rig", "harness"],
                None,
            ),
            (
                "dangling flag keeps rest",
                vec!["rig", "--profile"],
                vec!["rig"],
                None,
            ),
        ];
        for (name, input, rest, profile) in cases {
            let (got_rest, got_profile) = strip_profile_arg(&s(&input));
            assert_eq!(got_rest, s(&rest), "{name}");
            assert_eq!(got_profile.as_deref(), profile, "{name}");
        }
    }
}

//! prim — the umbrella orchestrator.
//!
//! A thin console selector: pick which components of the ecosystem to install
//! (core, rig = dev environment, harness = agent config) and it downloads and
//! runs each component's own installer. It does NOT bundle their logic — each
//! component keeps its own lightweight binary, invoked on demand. This honors
//! the install matrix: harness alone / rig+harness / core+harness / all.
//!
//! Status: Phase 0 skeleton — the selector prints the plan. Download + invoke
//! wiring lands in Phase 3.

use std::io::{self, Write};

/// A selectable component. `installer` is the bootstrap each one already ships.
struct Component {
    key: &'static str,
    name: &'static str,
    description: &'static str,
}

const COMPONENTS: &[Component] = &[
    Component {
        key: "rig",
        name: "Rig (dev environment)",
        description: "toolchains, herdr, editor, dotfiles. Agent (Pi + gentle-ai) optional.",
    },
    Component {
        key: "harness",
        name: "Harness (agent config)",
        description: "agent-agnostic skills + instructions, installed into ~/.agents/skills.",
    },
    Component {
        key: "core",
        name: "Core (operating core)",
        description: "data, services, identity provisioned into your own accounts (future).",
    },
];

fn main() {
    println!("prim — install selector\n");
    for (i, c) in COMPONENTS.iter().enumerate() {
        println!("  {}) {} — {}", i + 1, c.name, c.description);
    }
    println!("  a) all");
    print!("\nSelect components (e.g. 1 2, or 'a'): ");
    let _ = io::stdout().flush();

    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        eprintln!("no input");
        std::process::exit(1);
    }

    let chosen = resolve_selection(line.trim());
    if chosen.is_empty() {
        eprintln!("nothing selected");
        std::process::exit(1);
    }

    for key in &chosen {
        // Phase 3: download and run <key>-install here.
        println!("[prim] would install: {key}");
    }
}

/// Map a raw selection string to component keys. 'a'/'all' selects everything;
/// otherwise 1-based indices separated by spaces or commas.
fn resolve_selection(input: &str) -> Vec<&'static str> {
    let lower = input.to_lowercase();
    if lower == "a" || lower == "all" {
        return COMPONENTS.iter().map(|c| c.key).collect();
    }
    lower
        .split([' ', ','])
        .filter(|s| !s.is_empty())
        .filter_map(|tok| tok.parse::<usize>().ok())
        .filter(|&n| n >= 1 && n <= COMPONENTS.len())
        .map(|n| COMPONENTS[n - 1].key)
        .collect()
}

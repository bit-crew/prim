//! prim-skills-install — installs the agent-agnostic skills into the agent.
//!
//! Two kinds of skill, one destination:
//!   - OWN skills (this repo's `skills/*`): copied into ~/.agents/skills.
//!   - EXTERNAL skills (Archify, grill-me): installed from source with the
//!     `skills` CLI (`npx skills add ... --agent pi`), which also writes to
//!     ~/.agents/skills. Installing from source keeps them updatable.
//!
//! ~/.agents/skills is a scan root for Pi (verified in gentle-pi's
//! skill-registry), so everything lands in one place Pi reads.
//!
//! Target agent: Pi (for now). The set of skills is universal (no profiles).
//!
//! Subcommands:
//!   (none)   install/refresh all skills (idempotent)
//!   update   update external skills to latest (`skills update`)
//!
//! Dependencies: Node/npx (for the `skills` CLI). If missing, external skills
//! are skipped with a note — the environment installer (devlab) owns Node.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const BLUE: &str = "\x1b[1;34m";
const GREEN: &str = "\x1b[1;32m";
const RED: &str = "\x1b[1;31m";
const RESET: &str = "\x1b[0m";

/// This content repo, cloned at install time to get the OWN skills.
const SKILLS_REPO: &str = "https://github.com/bit-crew/prim-skills.git";

/// External skills installed from source via the `skills` CLI.
/// (repo, skill-name-or-"*")
const EXTERNAL_SKILLS: &[(&str, &str)] = &[
    ("tt-a1i/archify", "archify"),
    ("mattpocock/skills", "grilling"),
    ("DietrichGebert/ponytail", "ponytail"),
];

fn log(msg: &str) {
    println!("{BLUE}[prim-skills]{RESET} {msg}");
}
fn ok(msg: &str) {
    println!("{GREEN}[prim-skills]{RESET} {msg}");
}
fn err(msg: &str) {
    eprintln!("{RED}[prim-skills]{RESET} {msg}");
}

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_default();
    let result = match arg.as_str() {
        "update" => run_update(),
        "uninstall" => run_uninstall(),
        _ => run_install(),
    };
    if let Err(e) = result {
        err(&format!("fallo: {e}"));
        std::process::exit(1);
    }
}

/// Install/refresh all skills into ~/.agents/skills.
fn run_install() -> io::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| io::Error::other("sin HOME"))?;
    let dest = agents_skills_dir(&home);
    std::fs::create_dir_all(&dest)?;

    install_own_skills(&dest)?;
    install_external_skills();

    ok("Skills instaladas/actualizadas en ~/.agents/skills.");
    Ok(())
}

/// Update external skills to their latest version (`skills update`).
fn run_update() -> io::Result<()> {
    if !has_npx() {
        err("npx no disponible; no se pueden actualizar skills externas.");
        return Ok(());
    }
    log("Actualizando skills externas (skills update)...");
    let _ = run_npx(&["-y", "skills", "update", "--global", "--yes"]);
    ok("Actualizacion de skills externas completa.");
    Ok(())
}

/// Uninstall every skill this installer manages from ~/.agents/skills.
/// Own skills: remove the dirs that match this repo's `skills/*` (cloned to
/// learn their names). External skills: `skills remove` by name. Non-fatal.
fn run_uninstall() -> io::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| io::Error::other("sin HOME"))?;
    let dest = agents_skills_dir(&home);

    // Own skills: discover names from a shallow clone, remove matching dirs.
    let tmp = std::env::temp_dir().join(format!("prim-skills-rm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    if run_ok(
        "git",
        &["clone", "--depth", "1", SKILLS_REPO, tmp.to_str().unwrap()],
    ) {
        if let Ok(entries) = std::fs::read_dir(tmp.join("skills")) {
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.path().is_dir() {
                    let target = dest.join(entry.file_name());
                    if target.exists() && std::fs::remove_dir_all(&target).is_ok() {
                        log(&format!(
                            "skill propia eliminada: {}",
                            entry.file_name().to_string_lossy()
                        ));
                    }
                }
            }
        }
        let _ = std::fs::remove_dir_all(&tmp);
    } else {
        err("no se pudo clonar prim-skills; se omite la limpieza de skills propias.");
    }

    // External skills: remove by name via the skills CLI.
    if has_npx() {
        for (_, skill) in EXTERNAL_SKILLS {
            log(&format!("Quitando skill externa {skill}..."));
            let _ = run_npx(&["-y", "skills", "remove", skill, "--yes"]);
        }
    } else {
        err("npx no disponible; no se pueden quitar las skills externas.");
    }

    ok("Skills desinstaladas de ~/.agents/skills.");
    Ok(())
}

/// ~/.agents/skills — the unified destination (a Pi scan root).
fn agents_skills_dir(home: &Path) -> PathBuf {
    home.join(".agents").join("skills")
}

/// Clone this repo to a temp dir and copy its `skills/*` into the destination.
/// The clone is removed afterward so nothing lingers.
fn install_own_skills(dest: &Path) -> io::Result<()> {
    let tmp = std::env::temp_dir().join(format!("prim-skills-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    log("Clonando prim-skills (skills propias)...");
    if !run_ok(
        "git",
        &["clone", "--depth", "1", SKILLS_REPO, tmp.to_str().unwrap()],
    ) {
        err("no se pudo clonar prim-skills; se omiten las skills propias.");
        return Ok(());
    }
    let from_root = tmp.join("skills");
    let mut n = 0;
    if let Ok(entries) = std::fs::read_dir(&from_root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_dir() {
                let to = dest.join(entry.file_name());
                let _ = std::fs::remove_dir_all(&to); // refresh, don't merge stale
                if copy_dir(&p, &to).is_ok() {
                    n += 1;
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    log(&format!(
        "{n} skills propias instaladas en {}",
        dest.display()
    ));
    Ok(())
}

/// Install external skills from source with the `skills` CLI (updatable).
fn install_external_skills() {
    if !has_npx() {
        err("npx no disponible; se omiten las skills externas (Node lo instala devlab).");
        return;
    }
    for (repo, skill) in EXTERNAL_SKILLS {
        log(&format!("Instalando skill externa {repo} ({skill})..."));
        let ok_install = run_npx(&[
            "-y", "skills", "add", repo, "--skill", skill, "--agent", "pi", "--global", "--copy",
            "--yes",
        ]);
        if !ok_install {
            err(&format!("fallo instalando {repo} ({skill}) (continuo)"));
        }
    }
}

// --- helpers ----------------------------------------------------------------

/// Where devlab installs Node. Not always on a non-interactive shell's PATH
/// (that only happens via .zshrc), so we look here directly and prepend it when
/// running npx — otherwise npx's shebang fails to find `node`.
fn node_bin_dir() -> Option<PathBuf> {
    let dir = dirs::home_dir()?.join(".local").join("node").join("bin");
    dir.is_dir().then_some(dir)
}

fn has_npx() -> bool {
    which("npx")
        || node_bin_dir()
            .map(|d| d.join("npx").is_file())
            .unwrap_or(false)
}

/// Run npx with ~/.local/node/bin prepended to PATH so both npx and the `node`
/// its shebang invokes resolve even in a bare non-interactive environment.
fn run_npx(args: &[&str]) -> bool {
    let mut cmd = Command::new("npx");
    cmd.args(args);
    if let Some(node_dir) = node_bin_dir() {
        let path = std::env::var_os("PATH").unwrap_or_default();
        let mut dirs: Vec<PathBuf> = vec![node_dir];
        dirs.extend(std::env::split_paths(&path));
        if let Ok(joined) = std::env::join_paths(dirs) {
            cmd.env("PATH", joined);
        }
    }
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

fn run_ok(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
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

/// Recursively copy a directory tree.
fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let src = entry.path();
        let dst = to.join(entry.file_name());
        if src.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

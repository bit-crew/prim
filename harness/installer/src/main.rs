//! prim-skills-install — installs the agent-agnostic skills into the agent.
//! harness-install — installs and configures a coding agent.
//!
//! Two layers, one component:
//!   - AGENT RUNTIME (agent.rs): Google ADK (uv), Pi, gentle-ai + Pi settings.
//!   - AGENT CONFIG: skills (own + external) into ~/.agents/skills, plus curation
//!     (instructions into AGENTS.md, Engram local, skill-registry refresh).
//!
//! ~/.agents/skills is a scan root for Pi, so skills land where Pi reads them.
//! Target agent: Pi (for now); harness is meant to configure different agents.
//!
//! Subcommands:
//!   (none)        install the agent runtime + skills + curation
//!   --skills-only install only the skills (no runtime), for lighter setups
//!   update        update external skills to latest (`skills update`)
//!   uninstall     remove managed skills
//!
//! Dependencies: uv (agent runtime; ensured here if missing), Node/npx (external
//! skills), git (sparse clone), curl/bash (official agent installers).

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

mod agent;

const BLUE: &str = "\x1b[1;34m";
const GREEN: &str = "\x1b[1;32m";
const RED: &str = "\x1b[1;31m";
const RESET: &str = "\x1b[0m";

/// The monorepo, cloned (sparse) at install time to get the OWN skills.
const SKILLS_REPO: &str = "https://github.com/bit-crew/prim.git";
/// Subpath inside the monorepo holding this component's skills.
const SKILLS_SUBPATH: &str = "harness/skills";

/// External skills installed from source via the `skills` CLI.
/// (repo, skill-name-or-"*")
const EXTERNAL_SKILLS: &[(&str, &str)] = &[
    ("tt-a1i/archify", "archify"),
    ("mattpocock/skills", "grilling"),
    ("DietrichGebert/ponytail", "ponytail"),
];

fn log(msg: &str) {
    println!("{BLUE}[harness]{RESET} {msg}");
}
fn ok(msg: &str) {
    println!("{GREEN}[harness]{RESET} {msg}");
}
fn err(msg: &str) {
    eprintln!("{RED}[harness]{RESET} {msg}");
}

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_default();
    let result = match arg.as_str() {
        "update" => run_update(),
        "uninstall" => run_uninstall(),
        "--skills-only" | "skills-only" => run_skills_only(),
        _ => run_install(),
    };
    if let Err(e) = result {
        err(&format!("fallo: {e}"));
        std::process::exit(1);
    }
}

/// Install only the skills (own + external) into ~/.agents/skills. No runtime.
fn run_skills_only() -> io::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| io::Error::other("sin HOME"))?;
    let dest = agents_skills_dir(&home);
    std::fs::create_dir_all(&dest)?;
    ensure_node();
    install_skills(&dest)?;
    ok("Skills instaladas/actualizadas en ~/.agents/skills (solo skills).");
    Ok(())
}

/// Full install: ensure uv, install the agent runtime, install skills, curate.
fn run_install() -> io::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| io::Error::other("sin HOME"))?;
    let dest = agents_skills_dir(&home);
    std::fs::create_dir_all(&dest)?;

    ensure_uv();
    ensure_node();
    agent::install_runtime();

    // Curation installs the skills in-process (no curl|sh recursion).
    agent::curate(|| {
        let _ = install_skills(&dest);
    });

    ok("Agente instalado y configurado (runtime + skills + curaduria).");
    Ok(())
}

/// Install own + external skills into `dest`. Shared by full install and
/// --skills-only and the curation callback.
fn install_skills(dest: &Path) -> io::Result<()> {
    install_own_skills(dest)?;
    install_external_skills();
    Ok(())
}

/// Ensure uv is available (the agent runtime's ADK needs it). Installs via the
/// official script if missing, so `harness` can set up the agent standalone.
fn ensure_uv() {
    if which("uv") {
        return;
    }
    log("Instalando uv (requerido por el runtime del agente)...");
    if let Some(script) = curl_text("https://astral.sh/uv/install.sh") {
        let _ = pipe_sh(&script);
    } else {
        err("no se pudo descargar el instalador de uv (continuo; ADK podria fallar).");
    }
}

/// Fetch a URL body as text (IPv4 forced).
fn curl_text(url: &str) -> Option<String> {
    let out = Command::new("curl")
        .args(["-4", "-fsSL", url])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Pipe a script to sh.
fn pipe_sh(script: &str) -> bool {
    use std::io::Write as _;
    use std::process::Stdio;
    match Command::new("sh").stdin(Stdio::piped()).spawn() {
        Ok(mut c) => {
            if let Some(mut stdin) = c.stdin.take() {
                let _ = stdin.write_all(script.as_bytes());
            }
            c.wait().map(|s| s.success()).unwrap_or(false)
        }
        Err(_) => false,
    }
}

/// Ensure Node/npx is available (external skills install via `npx skills add`).
/// Installs the official Node tarball into ~/.local/node if missing, so harness
/// can install external skills standalone (without rig). Matches rig's layout.
fn ensure_node() {
    if has_npx() {
        return;
    }
    let Some(home) = dirs::home_dir() else { return };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => {
            err(&format!(
                "arch {other} no soportada para Node (continuo sin externas)"
            ));
            return;
        }
    };
    let Some(ver) = node_latest_version() else {
        err("no se pudo resolver version de Node (continuo sin externas)");
        return;
    };
    log(&format!(
        "Instalando Node.js {ver} (requerido por skills externas)..."
    ));
    let tmp = std::env::temp_dir().join(format!("harness-node-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let tar = tmp.join("node.tar.xz");
    let url = format!("https://nodejs.org/dist/{ver}/node-{ver}-linux-{arch}.tar.xz");
    let node_dir = home.join(".local").join("node");
    if run_ok(
        "curl",
        &["-4", "-fsSL", "-o", tar.to_str().unwrap_or(""), &url],
    ) {
        let _ = std::fs::create_dir_all(&node_dir);
        let _ = run_ok(
            "tar",
            &[
                "-xf",
                tar.to_str().unwrap_or(""),
                "-C",
                node_dir.to_str().unwrap_or(""),
                "--strip-components=1",
            ],
        );
    } else {
        err("no se pudo descargar Node.js (continuo sin externas)");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Resolve the latest Node version string (vX.Y.Z) from the dist index.
fn node_latest_version() -> Option<String> {
    let body = curl_text("https://nodejs.org/dist/index.json")?;
    let idx = body.find("\"version\"")?;
    let seg = &body[idx..];
    let q1 = seg.find('"')?;
    let after = &seg[q1 + 1..];
    let colon = after.find(':')?;
    let seg2 = &after[colon..];
    let a = seg2.find('"')? + 1;
    let b = seg2[a..].find('"')? + a;
    Some(seg2[a..b].to_string())
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

    // Own skills: discover names from a sparse clone, remove matching dirs.
    let tmp = std::env::temp_dir().join(format!("prim-skills-rm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    if sparse_clone(SKILLS_REPO, SKILLS_SUBPATH, &tmp) {
        if let Ok(entries) = std::fs::read_dir(tmp.join(SKILLS_SUBPATH)) {
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
    log("Clonando prim (skills propias, sparse)...");
    if !sparse_clone(SKILLS_REPO, SKILLS_SUBPATH, &tmp) {
        err("no se pudo clonar prim; se omiten las skills propias.");
        return Ok(());
    }
    let from_root = tmp.join(SKILLS_SUBPATH);
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

/// Sparse-clone only `subpath` of the monorepo into `tmp`. Avoids downloading
/// the whole repo to read one subdirectory. Returns true on success; on success
/// the subdir is at `tmp/<subpath>`.
fn sparse_clone(repo: &str, subpath: &str, tmp: &Path) -> bool {
    let tmp_s = match tmp.to_str() {
        Some(s) => s,
        None => return false,
    };
    run_ok(
        "git",
        &[
            "clone",
            "--depth",
            "1",
            "--filter=blob:none",
            "--sparse",
            repo,
            tmp_s,
        ],
    ) && run_ok_in(tmp, "git", &["sparse-checkout", "set", subpath])
}

/// Run a command with a given working directory.
fn run_ok_in(cwd: &Path, program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .current_dir(cwd)
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

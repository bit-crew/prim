//! Phase 1b: install tools not in Debian's apt (or too old there) by downloading
//! official release binaries. Ports run_once_15-install-binaries.sh.
//!
//! Mirrors the shell precedence: prefer an upstream release binary; a few Rust
//! tools are built with cargo when no good binary exists. kiro-cli is NOT ported
//! (removed: superseded by Pi/gentle-ai).
//!
//! Idempotent: each tool installs only when its binary is missing. Failures are
//! non-fatal (logged, continue) to match the shell's resilient behavior.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{which, Profile};

const BLUE: &str = "\x1b[1;34m";
const RESET: &str = "\x1b[0m";

fn note(msg: &str) {
    println!("{BLUE}[binaries]{RESET} {msg}");
}

/// Target architecture label families used across release URLs.
#[derive(Clone, Copy)]
pub struct Arch {
    uname: &'static str, // x86_64 | aarch64
}

/// Detect the architecture; None if unsupported.
pub fn detect_arch() -> Option<Arch> {
    match std::env::consts::ARCH {
        "x86_64" => Some(Arch { uname: "x86_64" }),
        "aarch64" => Some(Arch { uname: "aarch64" }),
        _ => None,
    }
}

impl Arch {
    fn is_x86(self) -> bool {
        self.uname == "x86_64"
    }
}

/// Common paths used by the installers.
struct Env {
    home: PathBuf,
    local_bin: PathBuf,
}

fn env() -> io::Result<Env> {
    let home = dirs::home_dir().ok_or_else(|| io::Error::other("sin HOME"))?;
    let local_bin = home.join(".local").join("bin");
    std::fs::create_dir_all(&local_bin)?;
    Ok(Env { home, local_bin })
}

/// Install all phase-1b tools. `profile` gates the VPS-only infra block.
pub fn install_all(profile: Profile) -> io::Result<()> {
    let Some(arch) = detect_arch() else {
        note("Arquitectura no soportada; se omite la descarga de binarios.");
        return Ok(());
    };
    let e = env()?;

    // Phase 1c first: rustup + uv must exist before cargo/uv tools below.
    install_toolchains(profile);

    // Shared tools (both profiles).
    install_cargo_tools();
    install_helix(&e, arch);
    install_lazygit(&e, arch);
    install_delta(&e, arch);
    install_marksman(&e, arch);
    install_node(&e, arch);
    install_go(&e, arch);
    install_lazysql();
    install_npm_lsps(&e);
    install_uv_tools();
    install_yazi(&e, arch);
    install_nerd_font(&e);

    // Multiplexer: Herdr on both profiles. It is the agent-aware multiplexer on
    // the VPS (drives Moshi over SSH) and a complete, lighter multiplexer with
    // saved layouts for offline manual work on the client. One tool, both cases.
    install_herdr();

    if profile == Profile::Vps {
        install_vps_infra(&e, arch);
    }

    note("Binarios y herramientas listas.");
    Ok(())
}

// --- download / extract helpers ---------------------------------------------

/// Download `url` to `dest` with curl. Returns false on failure (non-fatal).
/// Forces IPv4 (`-4`): some hosts (e.g. dl.google.com for the Go tarball)
/// resolve to IPv6, and a VPS with broken/unrouted IPv6 would hang or fail.
fn download(url: &str, dest: &Path) -> bool {
    Command::new("curl")
        .args(["-4", "-fsSL", url, "-o", dest.to_str().unwrap()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Fetch a URL's body as text (for piping installer scripts into sh).
/// Forces IPv4 for the same reason as `download` (hosts with broken IPv6).
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

/// Run `sh <args>` feeding `script` on stdin. Returns success as bool.
fn sh_stdin(script: &str, args: &[&str]) -> bool {
    pipe_to("sh", script, args)
}

/// Run `bash <args>` feeding `script` on stdin. Some installers (gentle-ai)
/// are bash-specific and fail under POSIX sh.
fn bash_stdin(script: &str, args: &[&str]) -> bool {
    pipe_to("bash", script, args)
}

/// Spawn `program <args>` and feed `script` to its stdin.
fn pipe_to(program: &str, script: &str, args: &[&str]) -> bool {
    use std::io::Write as _;
    use std::process::Stdio;
    let child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .spawn();
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

/// Resolve the latest release tag of a GitHub repo via the API, returning the
/// numeric version (no leading v). None on failure.
fn latest_github_version(repo: &str) -> Option<String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let out = Command::new("curl")
        .args(["-4", "-fsSL", &url])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let body = String::from_utf8_lossy(&out.stdout);
    // Find "tag_name": "vX.Y.Z" and strip to digits/dots.
    let idx = body.find("\"tag_name\"")?;
    let rest = &body[idx..];
    let start = rest.find(':')? + 1;
    let seg = &rest[start..];
    let q1 = seg.find('"')? + 1;
    let q2 = seg[q1..].find('"')? + q1;
    let tag = &seg[q1..q2];
    let ver: String = tag
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if ver.is_empty() {
        None
    } else {
        Some(ver)
    }
}

/// Make a file executable (no-op on non-unix).
fn chmod_x(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            let mut p = meta.permissions();
            p.set_mode(0o755);
            let _ = std::fs::set_permissions(path, p);
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// Run a command, returning success as bool (non-fatal callers).
fn run_ok(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// A fresh temp dir under the system temp.
fn tmpdir() -> io::Result<PathBuf> {
    let base = std::env::temp_dir().join(format!("devlab-{}", std::process::id()));
    std::fs::create_dir_all(&base)?;
    let d = base.join(format!("t{}", rand_suffix()));
    std::fs::create_dir_all(&d)?;
    Ok(d)
}

fn rand_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

// --- toolchains (phase 1c: rustup + uv + cargo.txt) -------------------------

/// Install rustup (stable) and uv if missing, then the per-profile tools from
/// packages/cargo.txt. Ports run_once_10-install-toolchains.sh.
fn install_toolchains(profile: Profile) {
    // rustup (stable toolchain), no PATH modification (dotfiles own PATH).
    if !which("rustup") {
        note("Instalando rustup (toolchain stable)...");
        let script = curl_text("https://sh.rustup.rs");
        if let Some(s) = script {
            // sh -s -- -y --default-toolchain stable --no-modify-path
            if !sh_stdin(
                &s,
                &[
                    "-s",
                    "--",
                    "-y",
                    "--default-toolchain",
                    "stable",
                    "--no-modify-path",
                ],
            ) {
                note("fallo instalacion de rustup (continuo)");
            }
        } else {
            note("no se pudo descargar rustup (continuo)");
        }
    }

    // uv (Python per-project manager).
    if !which("uv") {
        note("Instalando uv...");
        let script = curl_text("https://astral.sh/uv/install.sh");
        if let Some(s) = script {
            if !sh_stdin(&s, &["-s"]) {
                note("fallo instalacion de uv (continuo)");
            }
        } else {
            note("no se pudo descargar uv (continuo)");
        }
    }

    install_cargo_txt_tools(profile);
}

/// Install rustup components and cargo tools listed in packages/cargo.txt,
/// filtered by profile. Lines: "<item> <profile>". "component:X" -> rustup
/// component add; otherwise cargo install (if the binary is missing).
fn install_cargo_txt_tools(profile: Profile) {
    let text = include_str!("data/cargo.txt");
    for (item, is_component) in select_cargo_items(text, profile.as_str()) {
        if is_component {
            note(&format!("rustup component add {item}..."));
            let _ = run_ok("rustup", &["component", "add", &item]);
        } else if !which(&item) {
            note(&format!("cargo install {item}..."));
            if !run_ok("cargo", &["install", &item]) {
                note(&format!("fallo cargo install {item} (continuo)"));
            }
        }
    }
}

/// Pure selector for cargo.txt: returns (item, is_component) for the profile.
fn select_cargo_items(text: &str, profile: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        let item = fields[0];
        let prof = fields.get(1).copied().unwrap_or("both");
        if prof != "both" && prof != profile {
            continue;
        }
        if let Some(comp) = item.strip_prefix("component:") {
            out.push((comp.to_string(), true));
        } else {
            out.push((item.to_string(), false));
        }
    }
    out
}

// --- cargo tools ------------------------------------------------------------

/// Rust tools not in apt (or outdated): installed via cargo if missing.
fn install_cargo_tools() {
    for tool in ["eza", "zoxide", "starship", "atuin"] {
        if !which(tool) {
            note(&format!("cargo install {tool}..."));
            if !run_ok("cargo", &["install", "--locked", tool]) {
                note(&format!("fallo cargo install {tool} (continuo)"));
            }
        }
    }
    // taplo: TOML LSP, needs the 'lsp' feature.
    if !which("taplo") {
        note("cargo install taplo-cli (con LSP)...");
        if !run_ok(
            "cargo",
            &["install", "--locked", "--features", "lsp", "taplo-cli"],
        ) {
            note("fallo cargo install taplo-cli (continuo)");
        }
    }
}

// --- uv tools ---------------------------------------------------------------

/// Python dev tools installed globally-isolated via uv.
fn install_uv_tools() {
    if !which("uv") {
        return;
    }
    for tool in ["ruff", "pudb"] {
        if !which(tool) {
            note(&format!("uv tool install {tool}..."));
            if !run_ok("uv", &["tool", "install", tool]) {
                note(&format!("fallo uv tool install {tool} (continuo)"));
            }
        }
    }
}

// --- helix ------------------------------------------------------------------

/// Helix editor: official binary tarball (includes runtime). Pinned version.
fn install_helix(e: &Env, arch: Arch) {
    if which("hx") {
        return;
    }
    note("Instalando Helix (binario oficial)...");
    let ver = "25.01.1";
    let a = if arch.is_x86() { "x86_64" } else { "aarch64" };
    let Ok(tmp) = tmpdir() else { return };
    let url = format!(
        "https://github.com/helix-editor/helix/releases/download/{ver}/helix-{ver}-{a}-linux.tar.xz"
    );
    let tar = tmp.join("helix.tar.xz");
    if !download(&url, &tar) {
        note(&format!("no se pudo descargar Helix {ver} (continuo)"));
        return;
    }
    if !run_ok(
        "tar",
        &["-xf", tar.to_str().unwrap(), "-C", tmp.to_str().unwrap()],
    ) {
        note("fallo extrayendo Helix (continuo)");
        return;
    }
    // find helix-* dir
    let Some(dir) = first_subdir(&tmp, "helix-") else {
        return;
    };
    let share = e.home.join(".local").join("share");
    let _ = std::fs::create_dir_all(&share);
    let runtime_dst = share.join("helix-runtime");
    let _ = std::fs::remove_dir_all(&runtime_dst);
    let _ = copy_dir(&dir.join("runtime"), &runtime_dst);
    let hx = e.local_bin.join("hx");
    if std::fs::copy(dir.join("hx"), &hx).is_ok() {
        chmod_x(&hx);
        note("Helix instalado en ~/.local/bin/hx");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

// --- herdr (VPS multiplexer) ------------------------------------------------

/// Herdr: agent-aware multiplexer for the VPS (shows agent state, drives Moshi
/// over SSH). Installed via the official installer, which downloads the right
/// release binary onto PATH — same pattern as rustup/uv/chezmoi.
fn install_herdr() {
    if which("herdr") {
        return;
    }
    note("Instalando Herdr (instalador oficial)...");
    let Some(script) = curl_text("https://herdr.dev/install.sh") else {
        note("no se pudo descargar el instalador de Herdr (continuo)");
        return;
    };
    if sh_stdin(&script, &["-s"]) {
        note("Herdr instalado.");
    } else {
        note("fallo instalacion de Herdr (continuo)");
    }
}

// --- lazygit ----------------------------------------------------------------

fn install_lazygit(e: &Env, arch: Arch) {
    if which("lazygit") {
        return;
    }
    note("Instalando lazygit (binario oficial)...");
    let a = if arch.is_x86() { "x86_64" } else { "arm64" };
    let Some(ver) = latest_github_version("jesseduffield/lazygit") else {
        note("no se pudo resolver version de lazygit (continuo)");
        return;
    };
    let Ok(tmp) = tmpdir() else { return };
    let url = format!(
        "https://github.com/jesseduffield/lazygit/releases/latest/download/lazygit_{ver}_Linux_{a}.tar.gz"
    );
    let tar = tmp.join("lazygit.tar.gz");
    if download(&url, &tar)
        && run_ok(
            "tar",
            &[
                "-xf",
                tar.to_str().unwrap(),
                "-C",
                e.local_bin.to_str().unwrap(),
                "lazygit",
            ],
        )
    {
        chmod_x(&e.local_bin.join("lazygit"));
        note("lazygit instalado.");
    } else {
        note("no se pudo instalar lazygit (continuo)");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

// --- delta ------------------------------------------------------------------

fn install_delta(e: &Env, arch: Arch) {
    if which("delta") {
        return;
    }
    note("Instalando delta (binario oficial)...");
    let a = if arch.is_x86() {
        "x86_64-unknown-linux-gnu"
    } else {
        "aarch64-unknown-linux-gnu"
    };
    let Some(ver) = latest_github_version("dandavison/delta") else {
        note("no se pudo resolver version de delta (continuo)");
        return;
    };
    let Ok(tmp) = tmpdir() else { return };
    let url = format!(
        "https://github.com/dandavison/delta/releases/latest/download/delta-{ver}-{a}.tar.gz"
    );
    let tar = tmp.join("delta.tar.gz");
    if download(&url, &tar)
        && run_ok(
            "tar",
            &["-xf", tar.to_str().unwrap(), "-C", tmp.to_str().unwrap()],
        )
    {
        if let Some(bin) = find_file(&tmp, "delta") {
            let dst = e.local_bin.join("delta");
            if std::fs::copy(&bin, &dst).is_ok() {
                chmod_x(&dst);
                note(&format!("delta {ver} instalado."));
            }
        }
    } else {
        note("no se pudo instalar delta (continuo)");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

// --- marksman ---------------------------------------------------------------

fn install_marksman(e: &Env, arch: Arch) {
    if which("marksman") {
        return;
    }
    note("Instalando marksman (binario oficial)...");
    let a = if arch.is_x86() { "x64" } else { "arm64" };
    let url = format!(
        "https://github.com/artempyanykh/marksman/releases/latest/download/marksman-linux-{a}"
    );
    let dst = e.local_bin.join("marksman");
    if download(&url, &dst) {
        chmod_x(&dst);
        note("marksman instalado.");
    } else {
        note("no se pudo descargar marksman (continuo)");
    }
}

// --- node -------------------------------------------------------------------

/// Node.js official binary tarball (needed by basedpyright, yaml-ls, wrangler).
/// Installed under ~/.local/node (bin added to PATH by the dotfiles).
fn install_node(e: &Env, arch: Arch) {
    if which("node") {
        return;
    }
    note("Instalando Node.js (binario oficial)...");
    let a = if arch.is_x86() { "x64" } else { "arm64" };
    let Some(ver) = node_latest_version() else {
        note("no se pudo resolver version de Node (continuo)");
        return;
    };
    let Ok(tmp) = tmpdir() else { return };
    let url = format!("https://nodejs.org/dist/{ver}/node-{ver}-linux-{a}.tar.xz");
    let tar = tmp.join("node.tar.xz");
    let node_dir = e.home.join(".local").join("node");
    if download(&url, &tar) {
        let _ = std::fs::create_dir_all(&node_dir);
        if run_ok(
            "tar",
            &[
                "-xf",
                tar.to_str().unwrap(),
                "-C",
                node_dir.to_str().unwrap(),
                "--strip-components=1",
            ],
        ) {
            note(&format!("Node.js {ver} instalado."));
        }
    } else {
        note("no se pudo descargar Node.js (continuo)");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Resolve the latest Node version string (vX.Y.Z) from the dist index.
fn node_latest_version() -> Option<String> {
    let out = Command::new("curl")
        .args(["-4", "-fsSL", "https://nodejs.org/dist/index.json"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let body = String::from_utf8_lossy(&out.stdout);
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

// --- Go toolchain -----------------------------------------------------------

/// Go toolchain (official tarball) under ~/.local/go. First-class tool for both
/// profiles: the dotfiles add ~/.local/go/bin (the toolchain) and ~/go/bin
/// (GOPATH bin, where `go install` places binaries) to PATH. gentle-ai also
/// uses Go to build engram, but here Go is installed as a usable tool, not a
/// transient build dependency. Same tarball pattern as Node.
fn install_go(e: &Env, arch: Arch) {
    // The toolchain binary lives at ~/.local/go/bin/go.
    if e.home
        .join(".local")
        .join("go")
        .join("bin")
        .join("go")
        .is_file()
        || which("go")
    {
        return;
    }
    note("Instalando Go (binario oficial)...");
    let a = if arch.is_x86() { "amd64" } else { "arm64" };
    let Some(ver) = go_latest_version() else {
        note("no se pudo resolver version de Go (continuo)");
        return;
    };
    let Ok(tmp) = tmpdir() else { return };
    let url = format!("https://go.dev/dl/{ver}.linux-{a}.tar.gz");
    let tar = tmp.join("go.tar.gz");
    let go_dir = e.home.join(".local").join("go");
    if download(&url, &tar) {
        // Never untar over an existing tree (official docs: that produces a
        // broken install). Remove the previous dir first, then recreate.
        let _ = std::fs::remove_dir_all(&go_dir);
        let _ = std::fs::create_dir_all(&go_dir);
        if run_ok(
            "tar",
            &[
                "-xf",
                tar.to_str().unwrap(),
                "-C",
                go_dir.to_str().unwrap(),
                "--strip-components=1",
            ],
        ) {
            note(&format!("Go {ver} instalado en {}", go_dir.display()));
        }
    } else {
        note("no se pudo descargar Go (continuo)");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Resolve the latest stable Go version tag (e.g. "go1.27.2"). Uses the
/// official plain-text endpoint (first line is the version), which is far more
/// robust than parsing the JSON index.
fn go_latest_version() -> Option<String> {
    let out = Command::new("curl")
        .args(["-4", "-fsSL", "https://go.dev/VERSION?m=text"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let body = String::from_utf8_lossy(&out.stdout);
    let first = body.lines().next()?.trim();
    if first.starts_with("go") {
        Some(first.to_string())
    } else {
        None
    }
}

// --- lazysql (DB TUI) -------------------------------------------------------

/// lazysql: a TUI database client (PostgreSQL, MySQL, SQLite, ...). Installed
/// with `go install` into ~/go/bin (already on PATH), so it depends on Go
/// having been installed just above. Both profiles. No BigQuery (lazysql speaks
/// the Postgres/MySQL wire protocol; BigQuery would need a separate tool).
fn install_lazysql() {
    if which("lazysql") {
        return;
    }
    // Resolve the `go` we installed under ~/.local/go, else fall back to PATH.
    let go = dirs::home_dir()
        .map(|h| h.join(".local").join("go").join("bin").join("go"))
        .filter(|p| p.is_file())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "go".to_string());
    if !which("go") && go == "go" {
        note("Go no disponible; se omite lazysql (continuo)");
        return;
    }
    note("Instalando lazysql (go install)...");
    if !run_ok(&go, &["install", "github.com/jorgerojas26/lazysql@latest"]) {
        note("fallo go install lazysql (continuo)");
    }
}

// --- npm LSPs ---------------------------------------------------------------

/// basedpyright + yaml-language-server via npm (global under ~/.local).
fn install_npm_lsps(e: &Env) {
    let npm = e.home.join(".local").join("node").join("bin").join("npm");
    let npm_s = if npm.is_file() {
        npm.to_str().unwrap().to_string()
    } else {
        "npm".to_string()
    };
    if !which("npm") && !npm.is_file() {
        return;
    }
    let prefix = e.home.join(".local");
    for (bin, pkg) in [
        ("basedpyright-langserver", "basedpyright"),
        ("yaml-language-server", "yaml-language-server"),
    ] {
        if !which(bin) {
            note(&format!("npm install -g {pkg}..."));
            if !run_ok(
                &npm_s,
                &["install", "-g", "--prefix", prefix.to_str().unwrap(), pkg],
            ) {
                note(&format!("fallo npm install {pkg} (continuo)"));
            }
        }
    }
}

// --- yazi -------------------------------------------------------------------

fn install_yazi(e: &Env, arch: Arch) {
    if which("yazi") {
        return;
    }
    note("Instalando Yazi (binario oficial)...");
    let a = if arch.is_x86() {
        "x86_64-unknown-linux-gnu"
    } else {
        "aarch64-unknown-linux-gnu"
    };
    let Ok(tmp) = tmpdir() else { return };
    let url = format!("https://github.com/sxyazi/yazi/releases/latest/download/yazi-{a}.zip");
    let zip = tmp.join("yazi.zip");
    if download(&url, &zip)
        && run_ok(
            "unzip",
            &["-q", zip.to_str().unwrap(), "-d", tmp.to_str().unwrap()],
        )
    {
        let sub = tmp.join(format!("yazi-{a}"));
        for b in ["yazi", "ya"] {
            let from = sub.join(b);
            if from.is_file() {
                let dst = e.local_bin.join(b);
                if std::fs::copy(&from, &dst).is_ok() {
                    chmod_x(&dst);
                }
            }
        }
        note("Yazi instalado en ~/.local/bin/yazi");
    } else {
        note("no se pudo instalar Yazi (continuo)");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

// --- nerd font --------------------------------------------------------------

fn install_nerd_font(e: &Env) {
    let font_dir = e.home.join(".local").join("share").join("fonts");
    if font_dir.join("JetBrainsMonoNerdFont-Regular.ttf").is_file() {
        return;
    }
    note("Instalando JetBrains Mono Nerd Font...");
    let _ = std::fs::create_dir_all(&font_dir);
    let Ok(tmp) = tmpdir() else { return };
    let zip = tmp.join("font.zip");
    let url = "https://github.com/ryanoasis/nerd-fonts/releases/latest/download/JetBrainsMono.zip";
    if download(url, &zip) {
        let _ = run_ok(
            "unzip",
            &[
                "-qo",
                zip.to_str().unwrap(),
                "-d",
                font_dir.to_str().unwrap(),
                "*.ttf",
            ],
        );
        let _ = run_ok("fc-cache", &["-f", font_dir.to_str().unwrap()]);
        note("JetBrains Mono Nerd Font instalada.");
    } else {
        note("no se pudo descargar la Nerd Font (continuo)");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

// --- VPS-only infra ---------------------------------------------------------

/// VPS-only infra tools. The agent stack is Pi + gentle-ai: Pi is the base
/// coding agent (gentle-ai refuses to run without it), so it is installed
/// before gentle-ai. kiro-cli is intentionally NOT installed anymore.
fn install_vps_infra(e: &Env, arch: Arch) {
    note("Perfil vps: instalando infra...");
    install_aws(e, arch);
    install_terraform(e, arch);
    install_sops(e, arch);
    install_wrangler(e);
    install_gcloud(e, arch);
    install_adk();
    install_pi();
    install_gentle(e, arch);
    configure_pi_settings(e);
    install_moshi();
    configure_vps_services();
}

/// Enable the OS services Moshi needs to survive on the VPS: the SSH server
/// running (Moshi connects over SSH/Mosh) and user `linger` so the moshi-hook
/// systemd *user* service keeps running without an open login session and
/// restarts after reboot. Both need privileges; tried with `sudo -n`
/// (passwordless on the VPS). On failure, the exact command is printed to run
/// by hand. Non-fatal.
fn configure_vps_services() {
    // SSH server up (openssh-server is installed from system.txt; enable+start).
    if !try_sudo(&["systemctl", "enable", "--now", "ssh"]) {
        note("no se pudo habilitar ssh con sudo; ejecuta: sudo systemctl enable --now ssh");
    }
    // Linger: user services persist without a session and across reboots.
    let user = std::env::var("USER").unwrap_or_default();
    if user.is_empty() || !try_sudo(&["loginctl", "enable-linger", &user]) {
        note("no se pudo activar linger con sudo; ejecuta: sudo loginctl enable-linger $USER");
    } else {
        note("linger activado: los servicios de usuario sobreviven reboots.");
    }
}

/// Run a command via `sudo -n` (non-interactive). Returns success as bool.
fn try_sudo(args: &[&str]) -> bool {
    let mut full = vec!["-n"];
    full.extend_from_slice(args);
    run_ok("sudo", &full)
}

/// moshi-hook: the server-side daemon of Moshi (getmoshi.app) — bridges AI
/// coding agents on the VPS to the Moshi mobile app (approve prompts from the
/// phone's lock screen, see agent state, jump to the exact Herdr/tmux tab).
/// VPS-only. It is a Go binary (no Bun/Node), installed with Moshi's official
/// install script — same `curl | sh` pattern as Herdr/Pi/gentle-ai, and always
/// the latest version (no pinned tarball to maintain).
///
/// Pairing (`moshi-hook pair --token ...`, token from the phone app) and
/// starting the daemon are manual/interactive. A systemd user service is written
/// so the daemon persists across reboots once paired (linger is enabled in
/// configure_vps_services).
fn install_moshi() {
    if which("moshi-hook") {
        return;
    }
    note("Instalando moshi-hook (install.sh oficial)...");
    let Some(script) = curl_text("https://getmoshi.app/install.sh") else {
        note("no se pudo descargar install.sh de Moshi (continuo)");
        return;
    };
    if sh_stdin(&script, &[]) {
        note("moshi-hook instalado.");
        write_moshi_service();
        note("Pareo pendiente (manual): 'moshi-hook pair --token <token de la app>', 'moshi-hook install', y 'systemctl --user start moshi-hook'.");
    } else {
        note("fallo instalacion de moshi-hook (continuo)");
    }
}

/// Write a systemd *user* service so the moshi-hook daemon persists across
/// reboots once paired. Not enabled/started here (pairing is manual first).
fn write_moshi_service() {
    let Some(home) = dirs::home_dir() else { return };
    let dir = home.join(".config").join("systemd").join("user");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    // moshi-hook lands on PATH via the installer; reference it by name so the
    // service works wherever the installer placed it.
    let unit = "[Unit]\n\
         Description=moshi-hook daemon (Moshi mobile bridge)\n\
         After=network-online.target\n\n\
         [Service]\n\
         ExecStart=%h/.local/bin/moshi-hook serve\n\
         Restart=on-failure\n\n\
         [Install]\n\
         WantedBy=default.target\n";
    if std::fs::write(dir.join("moshi-hook.service"), unit).is_ok() {
        note("systemd user service escrito (~/.config/systemd/user/moshi-hook.service).");
    }
}

/// Post-install Pi settings fix. gentle-ai installs the `gentle-pi` package,
/// which registers a `codemode` tool that collides with Pi's built-in
/// `codemode` extension; Pi otherwise resolves this on first launch by writing
/// `-builtin:codemode` into ~/.pi/agent/settings.json. We apply it ahead of
/// time so the first run is clean. Idempotent merge: the rest of the file
/// (packages, tuiMode, ...) is preserved. Runs AFTER install_gentle because the
/// collision only exists once gentle-pi is present. No-op if settings.json is
/// missing (gentle-ai did not run) or the entry is already there.
fn configure_pi_settings(e: &Env) {
    use serde_json::Value;

    let path = e.home.join(".pi").join("agent").join("settings.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        note("~/.pi/agent/settings.json no existe; se omite fix codemode");
        return;
    };
    let Ok(mut root) = serde_json::from_str::<Value>(&text) else {
        note("settings.json invalido; se omite fix codemode (continuo)");
        return;
    };

    let entry = Value::String("-builtin:codemode".to_string());
    let Some(obj) = root.as_object_mut() else {
        note("settings.json: raiz no es objeto; se omite (continuo)");
        return;
    };
    let slot = obj
        .entry("extensions")
        .or_insert_with(|| Value::Array(vec![]));
    let Value::Array(list) = slot else {
        note("settings.json: 'extensions' no es un arreglo; se omite (continuo)");
        return;
    };
    if list.contains(&entry) {
        return; // already applied
    }
    list.push(entry);

    match serde_json::to_string_pretty(&root) {
        Ok(out) => {
            if std::fs::write(&path, out + "\n").is_ok() {
                note("Pi: '-builtin:codemode' agregado a settings.json.");
            } else {
                note("no se pudo escribir settings.json (continuo)");
            }
        }
        Err(_) => note("no se pudo serializar settings.json (continuo)"),
    }
}

/// Pi — the base coding agent (earendil-works/pi). gentle-ai configures Pi but
/// does NOT install it: `gentle-ai install --agent pi` fails preflight with
/// "Pi requires the `pi` executable in PATH" when Pi is missing. The official
/// Linux/macOS installer pins its dependencies and installs `pi` into
/// ~/.local/bin (already on PATH). VPS-only; runs before install_gentle.
fn install_pi() {
    if which("pi") {
        return;
    }
    note("Instalando Pi (install.sh oficial)...");
    let Some(script) = curl_text("https://pi.dev/install.sh") else {
        note("no se pudo descargar install.sh de Pi (continuo)");
        return;
    };
    if sh_stdin(&script, &[]) {
        note("Pi instalado.");
    } else {
        note("fallo instalacion de Pi (continuo)");
    }
}

/// gentle-ai: the agent configurator for the Gentleman ecosystem. On Linux
/// (the VPS) the official method is the install script piped to bash. Once the
/// binary is present we run the non-interactive setup targeting the Pi agent
/// (gentle-shell), which provisions the persona + Engram (local) stack. Engram
/// local is the default (no ENGRAM_CLOUD_* env set). VPS-only; skill curation
/// happens in a later phase.
fn install_gentle(_e: &Env, _arch: Arch) {
    if !which("gentle-ai") {
        note("Instalando gentle-ai (install.sh oficial)...");
        let Some(script) = curl_text(
            "https://raw.githubusercontent.com/Gentleman-Programming/gentle-ai/main/scripts/install.sh",
        ) else {
            note("no se pudo descargar install.sh de gentle-ai (continuo)");
            return;
        };
        // gentle-ai's installer is bash-specific; pipe it to bash.
        if !bash_stdin(&script, &[]) {
            note("fallo instalacion de gentle-ai (continuo)");
            return;
        }
    }
    if !which("gentle-ai") {
        note("gentle-ai no quedo en PATH; se omite setup (continuo)");
        return;
    }
    // Non-interactive setup for the Pi agent (gentle-shell).
    // Flags verified against docs/usage.md (CLI Flags: install):
    //  - scope via GENTLE_AI_INSTALL_SCOPE env (the doc's documented
    //    non-interactive/CI knob) = global: write to the agent's global config,
    //    appropriate for a dedicated dev VPS.
    //  - --pi-background-subagents on: without it a non-interactive run resolves
    //    `auto` to `off`; we want managed background subagents enabled.
    //  - --channel stable (default, explicit).
    // RDD has no install flag (ON by default globally; managed via
    // `gentle-ai review mode`), so it is intentionally not set here.
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

/// Google Agent Development Kit (ADK) — the framework CX Agent Studio builds on.
/// Installed globally via uv (with the a2a extra). GCP is part of the stack.
fn install_adk() {
    if !which("uv") || which("adk") {
        return;
    }
    note("Instalando google-adk (uv tool, con extra a2a)...");
    if !run_ok("uv", &["tool", "install", "google-adk[a2a]"]) {
        note("fallo uv tool install google-adk (continuo)");
    }
}

fn install_aws(e: &Env, _arch: Arch) {
    if which("aws") {
        return;
    }
    note("Instalando AWS CLI v2...");
    let a = std::env::consts::ARCH; // aws uses x86_64 / aarch64 directly
    let Ok(tmp) = tmpdir() else { return };
    let zip = tmp.join("awscli.zip");
    let url = format!("https://awscli.amazonaws.com/awscli-exe-linux-{a}.zip");
    if download(&url, &zip)
        && run_ok(
            "unzip",
            &["-q", zip.to_str().unwrap(), "-d", tmp.to_str().unwrap()],
        )
    {
        let installer = tmp.join("aws").join("install");
        let _ = run_ok(
            installer.to_str().unwrap(),
            &[
                "--bin-dir",
                e.local_bin.to_str().unwrap(),
                "--install-dir",
                e.home.join(".local").join("aws-cli").to_str().unwrap(),
                "--update",
            ],
        );
        note("AWS CLI instalado.");
    } else {
        note("no se pudo instalar AWS CLI (continuo)");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

fn install_terraform(e: &Env, arch: Arch) {
    if which("terraform") {
        return;
    }
    note("Instalando Terraform (binario oficial)...");
    let a = if arch.is_x86() { "amd64" } else { "arm64" };
    let Some(ver) = latest_github_version("hashicorp/terraform") else {
        note("no se pudo resolver version de Terraform (continuo)");
        return;
    };
    let Ok(tmp) = tmpdir() else { return };
    let zip = tmp.join("tf.zip");
    let url =
        format!("https://releases.hashicorp.com/terraform/{ver}/terraform_{ver}_linux_{a}.zip");
    if download(&url, &zip)
        && run_ok(
            "unzip",
            &["-q", zip.to_str().unwrap(), "-d", tmp.to_str().unwrap()],
        )
    {
        let dst = e.local_bin.join("terraform");
        if std::fs::copy(tmp.join("terraform"), &dst).is_ok() {
            chmod_x(&dst);
            note(&format!("Terraform {ver} instalado."));
        }
    } else {
        note("no se pudo instalar Terraform (continuo)");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

fn install_sops(e: &Env, arch: Arch) {
    if which("sops") {
        return;
    }
    note("Instalando sops (binario oficial)...");
    let a = if arch.is_x86() { "amd64" } else { "arm64" };
    let Some(ver) = latest_github_version("getsops/sops") else {
        note("no se pudo resolver version de sops (continuo)");
        return;
    };
    let url =
        format!("https://github.com/getsops/sops/releases/latest/download/sops-v{ver}.linux.{a}");
    let dst = e.local_bin.join("sops");
    if download(&url, &dst) {
        chmod_x(&dst);
        note(&format!("sops {ver} instalado."));
    } else {
        note("no se pudo instalar sops (continuo)");
    }
}

fn install_wrangler(e: &Env) {
    let npm = e.home.join(".local").join("node").join("bin").join("npm");
    let npm_s = if npm.is_file() {
        npm.to_str().unwrap().to_string()
    } else {
        "npm".to_string()
    };
    if which("wrangler") || (!which("npm") && !npm.is_file()) {
        return;
    }
    note("Instalando wrangler (npm)...");
    let prefix = e.home.join(".local");
    if !run_ok(
        &npm_s,
        &[
            "install",
            "-g",
            "--prefix",
            prefix.to_str().unwrap(),
            "wrangler",
        ],
    ) {
        note("fallo instalacion de wrangler (continuo)");
    }
}

// --- gcloud SDK -------------------------------------------------------------

/// Google Cloud SDK: resolve the latest archive by listing the official release
/// bucket, download, extract to ~/.local, run its install.sh, symlink binaries.
/// Mirrors run_once_15 (uses the storage.googleapis.com mirror, reliable).
fn install_gcloud(e: &Env, arch: Arch) {
    if which("gcloud") {
        return;
    }
    note("Instalando Google Cloud SDK (binario oficial)...");
    // gcloud archive arch label: x86_64 or arm.
    let a = if arch.is_x86() { "x86_64" } else { "arm" };
    let Some(file) = gcloud_latest_file(a) else {
        note("no se pudo resolver la version de gcloud SDK (continuo)");
        return;
    };
    let Ok(tmp) = tmpdir() else { return };
    let tar = tmp.join("gcloud.tar.gz");
    let url = format!("https://storage.googleapis.com/cloud-sdk-release/{file}");
    if !download(&url, &tar) {
        note("no se pudo descargar gcloud SDK (continuo)");
        let _ = std::fs::remove_dir_all(&tmp);
        return;
    }
    let local = e.home.join(".local");
    if run_ok(
        "tar",
        &["-xf", tar.to_str().unwrap(), "-C", local.to_str().unwrap()],
    ) {
        let sdk = local.join("google-cloud-sdk");
        let installer = sdk.join("install.sh");
        let _ = run_ok(
            installer.to_str().unwrap(),
            &["--quiet", "--path-update", "false"],
        );
        for bin in ["gcloud", "gsutil"] {
            let target = sdk.join("bin").join(bin);
            let link = e.local_bin.join(bin);
            #[cfg(unix)]
            {
                let _ = std::fs::remove_file(&link);
                let _ = std::os::unix::fs::symlink(&target, &link);
            }
            #[cfg(not(unix))]
            let _ = (&target, &link);
        }
        note(&format!("gcloud SDK instalado ({file})."));
    } else {
        note("fallo extrayendo gcloud SDK (continuo)");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

/// List the official cloud-sdk-release bucket and pick the newest linux archive
/// for the given arch label (x86_64 | arm). Returns the file name or None.
fn gcloud_latest_file(arch_label: &str) -> Option<String> {
    let url = "https://storage.googleapis.com/storage/v1/b/cloud-sdk-release/o?prefix=google-cloud-cli-&maxResults=4000";
    let out = Command::new("curl").args(["-fsSL", url]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let body = String::from_utf8_lossy(&out.stdout);
    let suffix = format!("-linux-{arch_label}.tar.gz");
    // Collect every "google-cloud-cli-<ver>-linux-<arch>.tar.gz" occurrence.
    let mut matches: Vec<String> = Vec::new();
    for part in body.split('"') {
        if part.starts_with("google-cloud-cli-") && part.ends_with(&suffix) {
            matches.push(part.to_string());
        }
    }
    // Newest by version-aware sort (compare the numeric segments).
    matches.sort_by_key(|n| version_key(n));
    matches.pop()
}

/// Extract a comparable numeric key from a gcloud archive name
/// (google-cloud-cli-537.0.0-linux-x86_64.tar.gz -> [537,0,0]).
fn version_key(name: &str) -> Vec<u64> {
    name.trim_start_matches("google-cloud-cli-")
        .split('-')
        .next()
        .unwrap_or("")
        .split('.')
        .filter_map(|s| s.parse::<u64>().ok())
        .collect()
}

// --- fs helpers -------------------------------------------------------------

/// First immediate subdirectory of `dir` whose name starts with `prefix`.
fn first_subdir(dir: &Path, prefix: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with(prefix))
                    .unwrap_or(false)
        })
}

/// Recursively find the first file named `name` under `dir`.
fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_dir() {
            if let Some(found) = find_file(&p, name) {
                return Some(found);
            }
        } else if p.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(p);
        }
    }
    None
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

#[cfg(test)]
mod tests {
    use super::{select_cargo_items, version_key};

    #[test]
    fn version_key_parses_segments() {
        assert_eq!(
            version_key("google-cloud-cli-537.0.0-linux-x86_64.tar.gz"),
            vec![537, 0, 0]
        );
    }

    #[test]
    fn version_key_orders_newest_last() {
        let mut v = [
            "google-cloud-cli-498.0.1-linux-x86_64.tar.gz".to_string(),
            "google-cloud-cli-537.0.0-linux-x86_64.tar.gz".to_string(),
            "google-cloud-cli-500.2.0-linux-x86_64.tar.gz".to_string(),
        ];
        v.sort_by_key(|n| version_key(n));
        assert!(v.last().unwrap().contains("537.0.0"));
    }

    const CARGO_TXT: &str = "\
# comment
component:rust-analyzer both
bacon both
foo-vps vps
bar-dc1 dc1
";

    #[test]
    fn cargo_items_vps_includes_both_and_vps() {
        let got = select_cargo_items(CARGO_TXT, "vps");
        assert_eq!(
            got,
            vec![
                ("rust-analyzer".to_string(), true),
                ("bacon".to_string(), false),
                ("foo-vps".to_string(), false),
            ]
        );
    }

    #[test]
    fn cargo_items_dc1_excludes_vps() {
        let got = select_cargo_items(CARGO_TXT, "dc1");
        assert_eq!(
            got,
            vec![
                ("rust-analyzer".to_string(), true),
                ("bacon".to_string(), false),
                ("bar-dc1".to_string(), false),
            ]
        );
    }
}

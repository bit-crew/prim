//! devlab-install — the devlab environment installer (Rust).
//!
//! Replaces the split shell installer (setup.sh + bootstrap.sh + run_once_*).
//! This binary OWNS the installation; chezmoi is kept only to apply dotfiles.
//!
//! What it does: detect the package manager and architecture, install system
//! packages (embedded from src/data/system.txt), create fd/bat symlinks,
//! install toolchains (rustup/uv/go) and downloaded binaries (helix, herdr,
//! lazygit, ...). On the VPS profile it also installs the agent (Pi + gentle-ai),
//! delegates skills to prim-skills, installs moshi-hook, and enables the OS
//! services it needs. Finally it installs chezmoi and applies the dotfiles.
//!
//! Profile: "vps" or "dc1" (client). Resolved from --profile, else DEVLAB_PROFILE,
//! else prompt. Non-interactive when stdin is not a TTY.

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::Command;

mod binaries;
mod gentle;
mod hook;

const BLUE: &str = "\x1b[1;34m";
const RED: &str = "\x1b[1;31m";
const RESET: &str = "\x1b[0m";

pub(crate) fn log(msg: &str) {
    println!("{BLUE}[devlab-install]{RESET} {msg}");
}

fn err(msg: &str) {
    eprintln!("{RED}[devlab-install]{RESET} {msg}");
}

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_default();
    let result = match arg.as_str() {
        "verify" => run_verify(),
        "hook" => run_hook_install(),
        _ => run(),
    };
    if let Err(e) = result {
        err(&format!("fallo: {e}"));
        std::process::exit(1);
    }
}

/// `devlab-install verify` — run by the git pre-commit hook. Exit 1 blocks commit.
fn run_verify() -> io::Result<()> {
    if hook::verify(&hook::cwd())? {
        Ok(())
    } else {
        std::process::exit(1);
    }
}

/// `devlab-install hook` — install the pre-commit hook into the current repo.
fn run_hook_install() -> io::Result<()> {
    let exe = std::env::current_exe()?;
    hook::install(&hook::cwd(), &exe)?;
    log("Hook de pre-commit instalado (verifica el proyecto antes de cada commit).");
    Ok(())
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Profile {
    Vps,
    Dc1,
}

impl Profile {
    fn as_str(self) -> &'static str {
        match self {
            Profile::Vps => "vps",
            Profile::Dc1 => "dc1",
        }
    }
}

fn run() -> io::Result<()> {
    ensure_install_path();
    let profile = resolve_profile()?;
    log(&format!("Perfil: {}", profile.as_str()));

    let pkgmgr = detect_pkgmgr()
        .ok_or_else(|| io::Error::other("gestor no soportado (no hay apt, pkg ni brew)"))?;
    log(&format!("Gestor de paquetes: {}", pkgmgr.name));

    install_system_packages(&pkgmgr, profile)?;
    ensure_fd_bat_symlinks();
    binaries::install_all(profile)?;
    // After gentle-ai is installed (VPS), curate it with prim-skills content.
    if profile == Profile::Vps {
        gentle::curate();
    }
    ensure_chezmoi()?;
    apply_dotfiles(profile)?;
    set_login_shell_zsh();

    log("Instalacion completa.");
    Ok(())
}

/// Set the user's login shell to zsh so the devlab environment (PATH, prompt,
/// integrations configured in ~/.zshrc) loads on every login without a manual
/// `chsh`. No-op if zsh is missing or already the login shell. Tries sudo first
/// (passwordless on a typical VPS); falls back to plain `chsh`, which may prompt
/// for the password once. Non-fatal: a failure only means the user keeps their
/// current shell and can run `zsh` by hand.
fn set_login_shell_zsh() {
    let Some(zsh) = which_path("zsh") else {
        log("zsh no encontrado; no se cambia el shell de login.");
        return;
    };
    let zsh = zsh.to_string_lossy().to_string();

    // Already the login shell? (SHELL reflects the login shell in practice.)
    if std::env::var("SHELL").map(|s| s == zsh).unwrap_or(false) {
        return;
    }

    log(&format!("Configurando zsh como shell de login ({zsh})..."));
    let user = std::env::var("USER").unwrap_or_default();
    let sudo_ok = !user.is_empty()
        && Command::new("sudo")
            .args(["-n", "chsh", "-s", &zsh, &user])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    if sudo_ok {
        log("Shell de login = zsh (via sudo). Reconecta para que tome efecto.");
        return;
    }
    // Fallback: plain chsh (may prompt for the password once).
    if run_cmd("chsh", &["-s", &zsh]).is_ok() {
        log("Shell de login = zsh. Reconecta para que tome efecto.");
    } else {
        log("No se pudo cambiar el shell de login (continuo). Ejecuta 'zsh' manualmente o 'chsh -s $(which zsh)'.");
    }
}

// --- profile + paths --------------------------------------------------------

/// Prepend the install locations to this process's PATH so every subprocess
/// (gentle's installer, cargo/uv tools, node-based LSPs) can find binaries that
/// were just installed, before the shell dotfiles that set PATH are applied.
/// Mirrors the `export PATH=...` the old run_once scripts did at their start.
fn ensure_install_path() {
    let Some(home) = dirs::home_dir() else { return };
    let extra = [
        home.join(".cargo").join("bin"),
        home.join(".local").join("bin"),
        home.join(".local").join("node").join("bin"),
        home.join(".local").join("go").join("bin"), // Go toolchain
        home.join("go").join("bin"),                // GOPATH bin (go install)
    ];
    let current = std::env::var_os("PATH").unwrap_or_default();
    let mut paths: Vec<PathBuf> = extra.to_vec();
    paths.extend(std::env::split_paths(&current));
    if let Ok(joined) = std::env::join_paths(paths) {
        std::env::set_var("PATH", joined);
    }
}

/// Resolve the profile: --profile <p>, else DEVLAB_PROFILE, else prompt
/// (default vps). Non-interactive stdin defaults to vps.
fn resolve_profile() -> io::Result<Profile> {
    if let Some(p) = arg_value("--profile").or_else(|| std::env::var("DEVLAB_PROFILE").ok()) {
        return parse_profile(&p);
    }
    if !io::stdin().is_terminal() {
        return Ok(Profile::Vps);
    }
    let ans = prompt("Perfil [vps/dc1] (vps): ")?;
    let ans = ans.trim();
    if ans.is_empty() {
        Ok(Profile::Vps)
    } else {
        parse_profile(ans)
    }
}

fn parse_profile(s: &str) -> io::Result<Profile> {
    match s.trim().to_ascii_lowercase().as_str() {
        "vps" => Ok(Profile::Vps),
        "dc1" => Ok(Profile::Dc1),
        other => Err(io::Error::other(format!(
            "perfil invalido: {other} (usa vps|dc1)"
        ))),
    }
}

/// Read a `--flag value` style argument.
fn arg_value(flag: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}

// --- package manager detection ----------------------------------------------

struct PkgMgr {
    name: &'static str,
    /// Column index in packages/system.txt: apt=2, pkg=3, brew=4 (0-based,
    /// after id=0, profile=1).
    column: usize,
}

/// Detect apt | pkg (Termux) | brew, mirroring bootstrap.sh.
fn detect_pkgmgr() -> Option<PkgMgr> {
    // Termux defines $PREFIX containing com.termux and ships `pkg`.
    if let Ok(prefix) = std::env::var("PREFIX") {
        if prefix.contains("com.termux") && which("pkg") {
            return Some(PkgMgr {
                name: "pkg",
                column: 3,
            });
        }
    }
    if which("apt-get") {
        return Some(PkgMgr {
            name: "apt",
            column: 2,
        });
    }
    if which("brew") {
        return Some(PkgMgr {
            name: "brew",
            column: 4,
        });
    }
    None
}

/// Is `bin` on PATH? (adds .exe/.cmd on Windows for local dev/testing).
fn which(bin: &str) -> bool {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let exts: &[&str] = if cfg!(windows) {
        &["", ".exe", ".cmd"]
    } else {
        &[""]
    };
    std::env::split_paths(&path)
        .any(|dir| exts.iter().any(|e| dir.join(format!("{bin}{e}")).is_file()))
}

// --- system packages --------------------------------------------------------

/// Parse packages/system.txt and install the packages for this profile using
/// the detected manager's column. Columns: 0=id 1=profile 2=apt 3=pkg 4=brew.
/// A "-" means "not available in that manager". Skips comments and blanks.
///
/// The package list is embedded at compile time (include_str!) so the binary is
/// standalone: it does not need the devlab repo cloned on the target host.
fn install_system_packages(mgr: &PkgMgr, profile: Profile) -> io::Result<()> {
    let text = include_str!("data/system.txt");

    let pkgs = select_packages(text, mgr.column, profile.as_str());

    if pkgs.is_empty() {
        log(&format!(
            "No hay paquetes para el perfil {} en {}",
            profile.as_str(),
            mgr.name
        ));
        return Ok(());
    }
    log(&format!("Instalando ({}): {}", mgr.name, pkgs.join(" ")));
    run_pkg_install(mgr, &pkgs)
}

/// Pure selector: given the file text, the manager column and the profile,
/// return the packages to install (skips comments, blanks, wrong profile, "-").
fn select_packages(text: &str, column: usize, profile: &str) -> Vec<String> {
    let mut pkgs = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() <= column {
            continue;
        }
        let prof = fields[1];
        if prof != "both" && prof != profile {
            continue;
        }
        let pkg = fields[column];
        if pkg != "-" {
            pkgs.push(pkg.to_string());
        }
    }
    pkgs
}

/// Run the manager-specific install command.
fn run_pkg_install(mgr: &PkgMgr, pkgs: &[String]) -> io::Result<()> {
    let status = match mgr.name {
        "apt" => {
            run_cmd("sudo", &["apt-get", "update", "-y"])?;
            let mut args = vec!["DEBIAN_FRONTEND=noninteractive", "apt-get", "install", "-y"];
            args.extend(pkgs.iter().map(|s| s.as_str()));
            run_cmd("sudo", &args)
        }
        "pkg" => {
            run_cmd("pkg", &["update", "-y"])?;
            let mut args = vec!["install", "-y"];
            args.extend(pkgs.iter().map(|s| s.as_str()));
            run_cmd("pkg", &args)
        }
        "brew" => {
            let mut args = vec!["install"];
            args.extend(pkgs.iter().map(|s| s.as_str()));
            run_cmd("brew", &args)
        }
        other => return Err(io::Error::other(format!("gestor desconocido: {other}"))),
    };
    status
}

// --- fd/bat symlinks (Debian naming) ----------------------------------------

/// On Debian, fd-find installs `fdfind` and bat installs `batcat`. Symlink them
/// to fd/bat in ~/.local/bin. On Termux/brew the binaries are already fd/bat.
fn ensure_fd_bat_symlinks() {
    let Some(home) = dirs::home_dir() else { return };
    let bin = home.join(".local").join("bin");
    let _ = std::fs::create_dir_all(&bin);
    for (src, dst) in [("fdfind", "fd"), ("batcat", "bat")] {
        if which(src) {
            let target = bin.join(dst);
            if !target.exists() {
                if let Some(src_path) = which_path(src) {
                    #[cfg(unix)]
                    let _ = std::os::unix::fs::symlink(&src_path, &target);
                    #[cfg(not(unix))]
                    let _ = std::fs::copy(&src_path, &target);
                    log(&format!("symlink {dst} -> {src}"));
                }
            }
        }
    }
}

fn which_path(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let exts: &[&str] = if cfg!(windows) {
        &["", ".exe", ".cmd"]
    } else {
        &[""]
    };
    std::env::split_paths(&path)
        .flat_map(|dir| exts.iter().map(move |e| dir.join(format!("{bin}{e}"))))
        .find(|p| p.is_file())
}

// --- chezmoi ----------------------------------------------------------------

/// Install chezmoi into ~/.local/bin if missing (official installer script).
fn ensure_chezmoi() -> io::Result<()> {
    if which("chezmoi") {
        return Ok(());
    }
    let home = dirs::home_dir().ok_or_else(|| io::Error::other("sin HOME"))?;
    let bin = home.join(".local").join("bin");
    std::fs::create_dir_all(&bin)?;
    log("Instalando chezmoi en ~/.local/bin...");
    // sh -c "$(curl -fsLS get.chezmoi.io)" -- -b <bin>
    let script = capture("curl", &["-fsLS", "get.chezmoi.io"])?;
    run_stdin("sh", &["-s", "--", "-b", bin.to_str().unwrap()], &script)
}

// --- dotfiles via chezmoi ---------------------------------------------------

/// Apply dotfiles with chezmoi, source = <repo>/home. Passes profile + git
/// Apply dotfiles with chezmoi. The dotfiles live in devlab/home, which is not
/// present when the binary runs standalone, so we shallow-clone devlab to a temp
/// dir, apply from <clone>/home, and remove the clone. Passes profile + git
/// identity as prompt values (matching home/.chezmoi.toml.tmpl prompt texts).
fn apply_dotfiles(profile: Profile) -> io::Result<()> {
    let tmp = std::env::temp_dir().join(format!("devlab-dotfiles-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    log("Clonando devlab para los dotfiles (home/)...");
    run_cmd(
        "git",
        &[
            "clone",
            "--depth",
            "1",
            "https://github.com/bit-crew/devlab.git",
            tmp.to_str().unwrap(),
        ],
    )?;
    let source = tmp.join("home");

    let git_name = capture("git", &["config", "--global", "user.name"])
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "devlab".into());
    let git_email = capture("git", &["config", "--global", "user.email"])
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "devlab@localhost".into());

    log(&format!(
        "Aplicando dotfiles con chezmoi (source: {})",
        source.display()
    ));
    // --promptString matches by prompt TEXT (see home/.chezmoi.toml.tmpl).
    let res = run_cmd(
        "chezmoi",
        &[
            "init",
            "--apply",
            "--force",
            "--source",
            source.to_str().unwrap(),
            "--promptString",
            &format!("Perfil (dc1|vps)={}", profile.as_str()),
            "--promptString",
            &format!("Nombre para git={git_name}"),
            "--promptString",
            &format!("Email para git={git_email}"),
        ],
    );
    let _ = std::fs::remove_dir_all(&tmp);
    res
}

// --- command helpers --------------------------------------------------------

/// Run a command, inheriting stdio. Errors if it cannot run or exits nonzero.
fn run_cmd(program: &str, args: &[&str]) -> io::Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|e| io::Error::other(format!("no se pudo ejecutar '{program}': {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "'{program}' fallo (codigo {:?})",
            status.code()
        )))
    }
}

/// Run a command and capture its stdout as a String.
fn capture(program: &str, args: &[&str]) -> io::Result<String> {
    let out = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| io::Error::other(format!("no se pudo ejecutar '{program}': {e}")))?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "'{program}' fallo (codigo {:?})",
            out.status.code()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Run a command feeding `input` to its stdin.
fn run_stdin(program: &str, args: &[&str], input: &str) -> io::Result<()> {
    use std::process::Stdio;
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| io::Error::other(format!("no se pudo ejecutar '{program}': {e}")))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes())?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "'{program}' fallo (codigo {:?})",
            status.code()
        )))
    }
}

fn prompt(msg: &str) -> io::Result<String> {
    print!("{msg}");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim_end_matches(['\n', '\r']).to_string())
}

#[cfg(test)]
mod tests {
    use super::select_packages;

    // Mirrors packages/system.txt shape: id profile apt pkg brew
    const SAMPLE: &str = "\
# comment
git             both    git            git        git
openssh-server  both    openssh-server openssh    -
openjdk         vps     openjdk-21-jdk openjdk-21 openjdk@21
docker          vps     docker.io      -          -

age             dc1     age            age        age
";

    // Columns: apt=2, pkg=3, brew=4

    #[test]
    fn apt_vps_includes_both_and_vps() {
        let got = select_packages(SAMPLE, 2, "vps");
        assert_eq!(
            got,
            vec!["git", "openssh-server", "openjdk-21-jdk", "docker.io"]
        );
    }

    #[test]
    fn apt_dc1_excludes_vps_only() {
        let got = select_packages(SAMPLE, 2, "dc1");
        assert_eq!(got, vec!["git", "openssh-server", "age"]);
    }

    #[test]
    fn brew_skips_dash_entries() {
        // openssh-server and docker are "-" in brew; openjdk is openjdk@21.
        let got = select_packages(SAMPLE, 4, "vps");
        assert_eq!(got, vec!["git", "openjdk@21"]);
    }

    #[test]
    fn pkg_column_and_comments_and_blanks() {
        let got = select_packages(SAMPLE, 3, "vps");
        assert_eq!(got, vec!["git", "openssh", "openjdk-21"]);
        assert!(!got.iter().any(|p| p.starts_with('#')));
    }
}

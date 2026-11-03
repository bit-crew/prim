//! devlab-install — the devlab environment installer (Rust).
//!
//! Replaces the split shell installer (setup.sh + bootstrap.sh + run_once_*).
//! This binary OWNS the installation; chezmoi is kept only to apply dotfiles.
//!
//! What it does: detect the package manager and architecture, install system
//! packages (embedded from src/data/system.txt), create fd/bat symlinks,
//! install toolchains (rustup/uv/go) and downloaded binaries (helix, herdr,
//! lazygit, ...). On the VPS profile it also installs cloud infra (AWS,
//! Terraform, sops, wrangler, gcloud), moshi-hook, and enables the OS services
//! it needs. Finally it installs chezmoi and applies the dotfiles.
//!
//! The agent (ADK + Pi + gentle-ai) is NOT installed here — that is the
//! harness component's job. Rig is the environment; harness configures agents.
//!
//! Profile: "vps" or "dc1" (client). Resolved from --profile, else DEVLAB_PROFILE,
//! else prompt. Non-interactive when stdin is not a TTY.

use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

mod binaries;
mod hook;
mod receipt;

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
        // "" | "install" (and any flag like --profile) falls through to run().
        "update" => run_update(),
        "uninstall" => run_uninstall(),
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

/// `rig-install update` (T4): bring an existing install to its latest state,
/// driven by the receipt (profile, installed packages, recorded binaries).
/// Every step is non-fatal; the run finishes by writing a FRESH receipt with
/// new timestamps, the versions actually installed and the fresh package list.
fn run_update() -> io::Result<()> {
    ensure_install_path();

    // Profile from the receipt. Without one: prompt on a TTY, refuse when
    // non-interactive (there is nothing recorded to update).
    let mut bins: Vec<receipt::Bin> = Vec::new();
    let mut apt: Vec<String> = Vec::new();
    let mut shell_changed = false;
    let mut prev_pkgmgr = String::new();
    let profile = match receipt::load() {
        Ok(r) => {
            bins = r.bins;
            apt = r.apt;
            shell_changed = r.login_shell_changed;
            prev_pkgmgr = r.pkgmgr;
            parse_profile(&r.profile)?
        }
        Err(e) if io::stdin().is_terminal() => {
            log(&format!("sin receipt legible ({e}); se preguntara el perfil."));
            resolve_profile()?
        }
        Err(e) => {
            return Err(io::Error::other(format!(
                "sin receipt legible ({e}) y sin TTY para preguntar: instala una vez primero (rig-install install)"
            )))
        }
    };
    log(&format!("Actualizando (perfil {})", profile.as_str()));

    // System packages: for apt/pkg/brew, installing again is upgrading. On
    // failure keep the receipt's previous list (the best truth available).
    let mgr = detect_pkgmgr();
    if let Some(m) = &mgr {
        match install_system_packages(m, profile) {
            Ok(fresh) => apt = fresh,
            Err(e) => log(&format!("actualizacion de paquetes fallo ({e}; continuo)")),
        }
    } else {
        log("gestor de paquetes no detectado; se omiten los paquetes del sistema.");
    }

    ensure_fd_bat_symlinks();
    binaries::update_bins(&mut bins);
    binaries::upgrade_uv_tools();
    binaries::reinstall_npm_lsps();
    binaries::ensure_cargo_tools(profile);
    binaries::update_rustup();
    log("Toolchain go: no se toca en update (reinstalalo a mano si necesitas otra version).");

    if let Err(e) = ensure_chezmoi().and_then(|()| apply_dotfiles(profile)) {
        log(&format!("dotfiles: fallo ({e}; continuo)"));
    }
    // Sticky: once rig changed the login shell, the receipt keeps recording it
    // so a later uninstall restores bash even when update found zsh already set.
    shell_changed |= set_login_shell_zsh();
    ensure_work_dir();

    let pkgmgr_name = match &mgr {
        Some(m) => m.name,
        None if !prev_pkgmgr.is_empty() => prev_pkgmgr.as_str(),
        None => "desconocido",
    };
    write_receipt(profile, pkgmgr_name, apt, bins, shell_changed)?;
    log("Actualizacion completa.");
    Ok(())
}

/// `rig-install uninstall` (T5): remove exactly what the receipt recorded —
/// nothing else. User data is never touched (S4): the protected notice is
/// printed first and the final summary restates what was kept.
fn run_uninstall() -> io::Result<()> {
    let r = receipt::load().map_err(|e| {
        io::Error::other(format!(
            "no se pudo leer el receipt ({e}); no se desinstala nada. Instala primero (rig-install install)."
        ))
    })?;
    let home = dirs::home_dir().ok_or_else(|| io::Error::other("sin HOME"))?;

    log("Desinstalando rig (el receipt es la unica fuente de verdad).");
    log("Datos protegidos, nunca se tocan: ~/work, claves SSH de ~/.ssh, archivos .env.local, repos y datos de agentes.");

    let mut removed: Vec<String> = Vec::new();
    let mut manual: Vec<String> = Vec::new();

    // 1. System packages (S3a): manager-specific remove, non-fatal.
    if !r.apt.is_empty() {
        match remove_cmd(&r.pkgmgr, &r.apt) {
            Some((prog, args)) => {
                log(&format!(
                    "Quitando {} paquetes de {}...",
                    r.apt.len(),
                    r.pkgmgr
                ));
                let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                if run_cmd(&prog, &refs).is_ok() {
                    removed.push(format!("paquetes de {} ({})", r.pkgmgr, r.apt.len()));
                } else {
                    manual.push(format!("{prog} {}", args.join(" ")));
                }
            }
            None => log(&format!(
                "gestor '{}' no soportado para remove; quita a mano si hace falta: {}",
                r.pkgmgr,
                r.apt.join(" ")
            )),
        }
    }

    // 2. Recorded binaries.
    for b in &r.bins {
        let p = Path::new(&b.path);
        if p.is_file() {
            match std::fs::remove_file(p) {
                Ok(()) => removed.push(format!("bin {}", b.name)),
                Err(e) => log(&format!("no se pudo borrar {} ({e}; continuo)", b.path)),
            }
        }
    }

    // 3. Recorded dirs (stored "~"-relative, e.g. ~/.local/node).
    for d in &r.dirs {
        let p = expand_tilde(&home, d);
        if p.is_dir() {
            match std::fs::remove_dir_all(&p) {
                Ok(()) => removed.push(format!("dir {d}")),
                Err(e) => log(&format!(
                    "no se pudo borrar {} ({e}; continuo)",
                    p.display()
                )),
            }
        }
    }

    // 4. Recorded toolchains.
    let has = |n: &str| r.toolchains.iter().any(|t| t.as_str() == n);
    if has("rustup") && home.join(".rustup").is_dir() {
        log("rustup self uninstall (borra ~/.rustup y ~/.cargo)...");
        if run_cmd("rustup", &["self", "uninstall", "-y"]).is_ok() {
            removed.push("toolchain rustup (~/.rustup, ~/.cargo)".into());
        } else {
            manual.push("rustup self uninstall -y".into());
        }
    }
    if has("go") {
        let g = home.join(".local").join("go");
        if g.is_dir() && std::fs::remove_dir_all(&g).is_ok() {
            removed.push("toolchain go (~/.local/go)".into());
        }
    }
    if has("uv") {
        let bin = home.join(".local").join("bin").join("uv");
        let data = home.join(".local").join("share").join("uv");
        let mut gone = false;
        if bin.is_file() {
            gone |= std::fs::remove_file(&bin).is_ok();
        }
        if data.is_dir() {
            gone |= std::fs::remove_dir_all(&data).is_ok();
        }
        if gone {
            removed.push("toolchain uv (~/.local/bin/uv, ~/.local/share/uv)".into());
        }
    }

    // 5. Dotfiles: chezmoi's own state (not the applied files in ~). The
    // direct removals below also cover the case where the chezmoi binary is
    // already gone.
    if which("chezmoi") {
        if run_cmd("chezmoi", &["purge", "--force"]).is_ok() {
            removed.push("estado de chezmoi (purge)".into());
        } else {
            manual.push("chezmoi purge --force".into());
        }
    }
    let cz_bin = home.join(".local").join("bin").join("chezmoi");
    if cz_bin.is_file() && std::fs::remove_file(&cz_bin).is_ok() {
        removed.push("bin chezmoi".into());
    }
    for (label, dir) in [
        ("~/.config/chezmoi", home.join(".config").join("chezmoi")),
        (
            "~/.local/share/chezmoi",
            home.join(".local").join("share").join("chezmoi"),
        ),
    ] {
        if dir.is_dir() && std::fs::remove_dir_all(&dir).is_ok() {
            removed.push(format!("estado de chezmoi ({label})"));
        }
    }

    // 6. Login shell back to bash, only if rig changed it and it is still zsh.
    if r.login_shell_changed && std::env::var("SHELL").is_ok_and(|s| s.ends_with("zsh")) {
        let user = std::env::var("USER").unwrap_or_default();
        if !user.is_empty() && run_cmd("sudo", &["-n", "chsh", "-s", "/bin/bash", &user]).is_ok() {
            log("Shell de login devuelto a /bin/bash. Reconecta para que tome efecto.");
            removed.push("cambio de shell de login".into());
        } else {
            log("no se pudo restaurar el shell; ejecuta: sudo chsh -s /bin/bash $USER");
            manual.push("sudo chsh -s /bin/bash $USER".into());
        }
    }

    // 7. The receipt itself, always last.
    match receipt::receipt_path() {
        Ok(p) if p.is_file() => match std::fs::remove_file(&p) {
            Ok(()) => removed.push("receipt".into()),
            Err(e) => log(&format!("no se pudo borrar el receipt ({e}; continuo)")),
        },
        _ => log("no habia receipt que borrar."),
    }

    log(&format!(
        "Eliminado ({}): {}",
        removed.len(),
        removed.join(", ")
    ));
    if !manual.is_empty() {
        log(&format!("Pendiente manual: {}", manual.join("; ")));
    }
    log("Conservado (protegido): ~/work, ~/.ssh, archivos .env.local, repos y datos de agentes.");
    log("Desinstalacion de rig completa.");
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

    let apt = install_system_packages(&pkgmgr, profile)?;
    ensure_fd_bat_symlinks();
    binaries::install_all(profile)?;
    ensure_chezmoi()?;
    apply_dotfiles(profile)?;
    let login_shell_changed = set_login_shell_zsh();
    ensure_work_dir();

    let home = dirs::home_dir().ok_or_else(|| io::Error::other("sin HOME"))?;
    write_receipt(
        profile,
        pkgmgr.name,
        apt,
        binaries::manifest(&home),
        login_shell_changed,
    )?;
    log("Instalacion completa.");
    Ok(())
}

/// Record the install receipt — the truth the future update/uninstall verbs
/// read (S3): profile, package manager plus the packages it installed, the
/// downloaded-binary manifest, toolchains, chezmoi dotfiles, login-shell
/// change and ~/work. Reached only when every step of run() succeeded, so a
/// failed install leaves no receipt behind. Update reuses it with the bins it
/// refreshed and the freshly installed package list.
fn write_receipt(
    profile: Profile,
    pkgmgr_name: &str,
    apt: Vec<String>,
    bins: Vec<receipt::Bin>,
    login_shell_changed: bool,
) -> io::Result<()> {
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| io::Error::other(format!("reloj del sistema invalido: {e}")))?
        .as_secs();
    let receipt = receipt::Receipt {
        component: "rig".into(),
        schema: 1,
        installed_at_unix: unix.to_string(),
        installed_at_iso: receipt::iso_utc(unix),
        profile: profile.as_str().into(),
        pkgmgr: pkgmgr_name.into(),
        apt,
        bins,
        dirs: vec!["~/.local/node".into()],
        toolchains: vec!["rustup".into(), "uv".into(), "go".into()],
        dotfiles: "chezmoi".into(),
        login_shell_changed,
        work_dir: true,
    };
    receipt::save(&receipt)?;
    log("Receipt escrito (~/.local/share/prim/rig.receipt.json).");
    Ok(())
}

/// Set the user's login shell to zsh so the devlab environment (PATH, prompt,
/// integrations configured in ~/.zshrc) loads on every login without a manual
/// `chsh`. No-op if zsh is missing or already the login shell. Tries sudo first
/// (passwordless on a typical VPS); falls back to plain `chsh`, which may prompt
/// for the password once. Non-fatal: a failure only means the user keeps their
/// current shell and can run `zsh` by hand. Returns true only when the shell
/// was actually changed (recorded in the install receipt).
fn set_login_shell_zsh() -> bool {
    let Some(zsh) = which_path("zsh") else {
        log("zsh no encontrado; no se cambia el shell de login.");
        return false;
    };
    let zsh = zsh.to_string_lossy().to_string();

    // Already the login shell? (SHELL reflects the login shell in practice.)
    if std::env::var("SHELL").map(|s| s == zsh).unwrap_or(false) {
        return false;
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
        return true;
    }
    // Fallback: plain chsh (may prompt for the password once).
    if run_cmd("chsh", &["-s", &zsh]).is_ok() {
        log("Shell de login = zsh. Reconecta para que tome efecto.");
        return true;
    }
    log("No se pudo cambiar el shell de login (continuo). Ejecuta 'zsh' manualmente o 'chsh -s $(which zsh)'.");
    false
}

/// ~/work is where organizations live (one directory per git account, managed
/// by `prim org`; see rig/docs/organizaciones.md). Created here so a fresh
/// install ends with it ready, and new shells land in it (see dot_zshrc).
fn ensure_work_dir() {
    let Some(home) = dirs::home_dir() else { return };
    let work = home.join("work");
    match std::fs::create_dir_all(&work) {
        Ok(()) => log(&format!(
            "Carpeta de organizaciones lista: {}",
            work.display()
        )),
        Err(e) => log(&format!(
            "no se pudo crear {} ({e}; continuo)",
            work.display()
        )),
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
/// Returns the packages that were (to be) installed — recorded in the install
/// receipt so a future uninstall can remove exactly those.
///
/// The package list is embedded at compile time (include_str!) so the binary is
/// standalone: it does not need the devlab repo cloned on the target host.
fn install_system_packages(mgr: &PkgMgr, profile: Profile) -> io::Result<Vec<String>> {
    let text = include_str!("data/system.txt");

    let pkgs = select_packages(text, mgr.column, profile.as_str());

    if pkgs.is_empty() {
        log(&format!(
            "No hay paquetes para el perfil {} en {}",
            profile.as_str(),
            mgr.name
        ));
        return Ok(Vec::new());
    }
    log(&format!("Instalando ({}): {}", mgr.name, pkgs.join(" ")));
    run_pkg_install(mgr, &pkgs)?;
    Ok(pkgs)
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
/// Apply dotfiles with chezmoi. The dotfiles live in the monorepo at rig/home,
/// which is not present when the binary runs standalone, so we sparse-clone just
/// that subpath from prim to a temp dir, apply from <clone>/rig/home, and remove
/// the clone. Passes profile + git identity as prompt values (matching
/// rig/home/.chezmoi.toml.tmpl prompt texts).
fn apply_dotfiles(profile: Profile) -> io::Result<()> {
    let tmp = std::env::temp_dir().join(format!("prim-dotfiles-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    log("Clonando prim para los dotfiles (rig/home, sparse)...");
    run_cmd(
        "git",
        &[
            "clone",
            "--depth",
            "1",
            "--filter=blob:none",
            "--sparse",
            "https://github.com/bit-crew/prim.git",
            tmp.to_str().unwrap(),
        ],
    )?;
    run_cmd_in(&tmp, "git", &["sparse-checkout", "set", "rig/home"])?;
    let source = tmp.join("rig").join("home");

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

/// Like `run_cmd` but with a working directory (for e.g. git sparse-checkout).
fn run_cmd_in(cwd: &Path, program: &str, args: &[&str]) -> io::Result<()> {
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
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

// --- lifecycle helpers (T4/T5) ----------------------------------------------

/// Manager-specific removal command for the receipt's packages (S3a).
/// None = unknown manager: the caller skips removal with a warning instead of
/// guessing (a receipt from another machine must not drive a blind remove).
fn remove_cmd(pkgmgr: &str, pkgs: &[String]) -> Option<(String, Vec<String>)> {
    let build = |prog: &str, pre: &[&str]| {
        let mut args: Vec<String> = pre.iter().map(|s| s.to_string()).collect();
        args.extend(pkgs.iter().cloned());
        Some((prog.to_string(), args))
    };
    match pkgmgr {
        "apt" => build("sudo", &["-n", "apt-get", "remove", "-y"]),
        "pkg" => build("pkg", &["remove", "-y"]),
        "brew" => build("brew", &["uninstall"]),
        _ => None,
    }
}

/// Expand a receipt "~"-relative path against the real home (receipts store
/// user-readable values like "~/.local/node").
fn expand_tilde(home: &Path, s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        home.join(rest)
    } else if s == "~" {
        home.to_path_buf()
    } else {
        PathBuf::from(s)
    }
}

#[cfg(test)]
mod tests {
    use super::{expand_tilde, remove_cmd, select_packages};
    use std::path::{Path, PathBuf};

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

    #[test]
    fn remove_cmd_builds_manager_specific_removal() {
        let (prog, args) = remove_cmd("apt", &["git".into(), "zsh".into()]).unwrap();
        assert_eq!(prog, "sudo");
        assert_eq!(args, vec!["-n", "apt-get", "remove", "-y", "git", "zsh"]);

        let (prog, args) = remove_cmd("pkg", &["git".into()]).unwrap();
        assert_eq!(prog, "pkg");
        assert_eq!(args, vec!["remove", "-y", "git"]);

        let (prog, args) = remove_cmd("brew", &["git".into()]).unwrap();
        assert_eq!(prog, "brew");
        assert_eq!(args, vec!["uninstall", "git"]);
    }

    #[test]
    fn remove_cmd_unknown_manager_is_none() {
        assert!(remove_cmd("apt-sim", &["git".into()]).is_none());
        assert!(remove_cmd("", &[]).is_none());
    }

    #[test]
    fn expand_tilde_resolves_receipt_paths() {
        let home = Path::new("/tmp/h");
        assert_eq!(
            expand_tilde(home, "~/.local/node"),
            PathBuf::from("/tmp/h/.local/node")
        );
        assert_eq!(expand_tilde(home, "~"), PathBuf::from("/tmp/h"));
        assert_eq!(expand_tilde(home, "/abs/path"), PathBuf::from("/abs/path"));
    }
}

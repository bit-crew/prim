//! prim bootstrap — make the machine prim-ready (S6a, S7, S8).
//!
//! `prim bootstrap` (and every install whose selection includes rig) makes
//! prim self-sufficient on a bare machine: apt update/upgrade, official Nix
//! single-user install and the static identity file. Steps are non-fatal by
//! design — anything that cannot run is noted and skipped; only catastrophic
//! failures (no $HOME, unwritable identity file) return Err, which makes
//! main exit 1. `RIG_SKIP_APT=1` skips the OS step (test/dry-run flag; also
//! an escape hatch for hosts whose OS updates are managed elsewhere).

use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::lifecycle::curl_to_file;

/// Env flag that skips the apt step. Test/dry-run guard: passwordless sudo
/// can be real on dev machines, so sandbox runs must defuse the OS step.
const SKIP_APT_ENV: &str = "RIG_SKIP_APT";

/// Official Nix installer script; run single-user via `--no-daemon` (S7).
const NIX_INSTALL_URL: &str = "https://nixos.org/nix/install";

/// Placeholder identity values when there is no TTY to ask (S6d).
pub(crate) const PLACEHOLDER_NAME: &str = "devlab";
pub(crate) const PLACEHOLDER_EMAIL: &str = "devlab@localhost";

/// Collects per-step outcomes ("ejecutado"/"omitido") for the summary.
#[derive(Default)]
struct Summary {
    ran: Vec<String>,
    skipped: Vec<String>,
}

impl Summary {
    /// Step 4: what ran, what skipped, and the session activation hint.
    fn print(&self) {
        println!("[prim] resumen del bootstrap:");
        for line in &self.ran {
            println!("[prim]   ejecutado: {line}");
        }
        for line in &self.skipped {
            println!("[prim]   omitido:   {line}");
        }
        println!("[prim] activa nix en esta sesion: . ~/.nix-profile/etc/profile.d/nix.sh");
    }
}

/// `prim bootstrap`: OS update + Nix single-user + identity, then summary.
pub fn run() -> io::Result<()> {
    let Some(home) = dirs::home_dir() else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no se pudo determinar $HOME",
        ));
    };
    println!("[prim] bootstrap: preparando la maquina (os, nix, identidad)...");
    let mut summary = Summary::default();
    let skip_apt = std::env::var_os(SKIP_APT_ENV).is_some_and(|v| v == "1");
    os_step(
        &mut summary,
        skip_apt,
        Path::new("/etc/debian_version").exists(),
    );
    nix_step(&mut summary, &home);
    let mut ask = |prompt: &str| -> Option<String> {
        print!("[prim] {prompt}: ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .ok()
            .map(|_| line.trim().to_string())
    };
    identity_step(&mut summary, &home, io::stdin().is_terminal(), &mut ask)?;
    summary.print();
    Ok(())
}

/// Step 1 (OS): `sudo -n apt-get update/upgrade`. Skipped with a note when
/// RIG_SKIP_APT=1, on non-Debian hosts, or without passwordless sudo. A
/// failing apt command is noted and never aborts the bootstrap.
fn os_step(summary: &mut Summary, skip_apt: bool, is_debian: bool) {
    if skip_apt {
        println!("[prim] RIG_SKIP_APT=1: se omite apt update/upgrade.");
        summary
            .skipped
            .push("apt update/upgrade (RIG_SKIP_APT=1)".into());
        return;
    }
    if !is_debian {
        println!("[prim] sistema no Debian; se omite apt update/upgrade.");
        summary
            .skipped
            .push("apt update/upgrade (no Debian)".into());
        return;
    }
    let sudo = Command::new("sudo")
        .args(["-n", "true"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !sudo {
        println!("[prim] sudo sin password no disponible; se omite apt update/upgrade.");
        summary.skipped.push("apt update/upgrade (sin sudo)".into());
        return;
    }
    for verb in ["update", "upgrade"] {
        let ok = Command::new("sudo")
            .args(["-n", "apt-get", verb, "-y"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            eprintln!("[prim] apt-get {verb} fallo; se continua.");
            summary.skipped.push(format!("apt {verb} (fallo)"));
            return;
        }
    }
    println!("[prim] apt update/upgrade listo.");
    summary.ran.push("apt update/upgrade".into());
}

/// Step 2 (Nix): official single-user install. /nix present = skip; otherwise
/// download nixos.org/nix/install and run `sh <file> --no-daemon`. Always
/// verifies with `nix --version` (~/.nix-profile/bin first, PATH fallback).
fn nix_step(summary: &mut Summary, home: &Path) {
    if Path::new("/nix").exists() {
        println!("[prim] Nix presente (/nix existe); se omite la instalacion.");
        summary
            .skipped
            .push("instalacion de nix (ya presente)".into());
    } else {
        match install_nix() {
            Ok(()) => summary.ran.push("instalacion de nix".into()),
            Err(note) => {
                eprintln!("[prim] {note}");
                summary.skipped.push("instalacion de nix (fallo)".into());
            }
        }
    }
    match nix_version(home) {
        Some(v) => {
            println!("[prim] nix verificado: {v}");
            summary.ran.push(format!("verificacion nix ({v})"));
        }
        None => {
            println!("[prim] no se pudo verificar nix; revisa ~/.nix-profile/bin/nix.");
            summary.skipped.push("verificacion de nix".into());
        }
    }
}

/// Download the official installer to a tmp file and run it single-user.
/// The installer manages its own privileges (creates /nix via sudo when
/// needed, then hands it to the invoking user).
fn install_nix() -> Result<(), String> {
    let tmp = std::env::temp_dir().join("prim-nix-install.sh");
    if !curl_to_file(NIX_INSTALL_URL, &tmp) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("no se pudo descargar {NIX_INSTALL_URL}"));
    }
    let ok = Command::new("sh")
        .arg(&tmp)
        .arg("--no-daemon")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let _ = fs::remove_file(&tmp);
    if ok {
        Ok(())
    } else {
        Err("el instalador de nix fallo; revisa el output arriba.".into())
    }
}

/// `nix --version` output, trying ~/.nix-profile/bin/nix then PATH `nix`
/// (a configured shell may have it on PATH even before profile.d runs).
fn nix_version(home: &Path) -> Option<String> {
    [home.join(".nix-profile/bin/nix"), PathBuf::from("nix")]
        .into_iter()
        .find_map(|bin| {
            Command::new(bin)
                .arg("--version")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        })
}

/// Step 3 (identity): ensure ~/.config/prim/identity.toml exists, never
/// overwrite it. TTY prompts for both values (empty answer = placeholder);
/// non-TTY writes placeholders with an edit-later note. The only hard
/// failure: the file cannot be written.
fn identity_step(
    summary: &mut Summary,
    home: &Path,
    tty: bool,
    ask: &mut dyn FnMut(&str) -> Option<String>,
) -> io::Result<()> {
    let path = identity_path(home);
    if path.exists() {
        println!(
            "[prim] identidad presente ({}); se conserva.",
            path.display()
        );
        summary.skipped.push("identidad (ya existe)".into());
        orgs_step(summary, home);
        return Ok(());
    }
    // An existing global git user is adopted silently — never re-asked (the
    // user's rule: a machine that already has a global identity keeps it).
    if let Some((n, e)) = fs::read_to_string(home.join(".gitconfig"))
        .ok()
        .and_then(|t| parse_git_identity(&t))
    {
        fs::create_dir_all(path.parent().expect("identity path has a parent"))?;
        fs::write(&path, identity_toml(&n, &e))?;
        println!(
            "[prim] identidad detectada en ~/.gitconfig ({n} <{e}>); se adopta sin preguntar."
        );
        summary
            .ran
            .push(format!("identidad (detectada: {n} <{e}>)"));
        orgs_step(summary, home);
        return Ok(());
    }
    let (name, email) = if tty {
        let name = ask("nombre de git (git_name)").filter(|s| !s.is_empty());
        let email = ask("email de git (git_email)").filter(|s| !s.is_empty());
        (
            name.unwrap_or_else(|| PLACEHOLDER_NAME.into()),
            email.unwrap_or_else(|| PLACEHOLDER_EMAIL.into()),
        )
    } else {
        println!(
            "[prim] sin TTY: identidad con placeholders ({PLACEHOLDER_NAME} <{PLACEHOLDER_EMAIL}>); editala luego."
        );
        (PLACEHOLDER_NAME.into(), PLACEHOLDER_EMAIL.into())
    };
    fs::create_dir_all(path.parent().expect("identity path has a parent"))?;
    fs::write(&path, identity_toml(&name, &email))?;
    println!("[prim] identidad escrita: {}", path.display());
    summary.ran.push(format!("identidad ({name} <{email}>)"));
    orgs_step(summary, home);
    Ok(())
}

/// Step 3b (orgs): the rig's ~/.gitconfig includes ~/.config/git/orgs.config
/// instead of inline [includeIf] entries, so a pre-existing ~/.gitconfig
/// with inline stanzas keeps working: they are moved into orgs.config here
/// (verbatim, once; entries already present are never duplicated). The
/// original ~/.gitconfig is never touched — the activation backs it up.
fn orgs_step(summary: &mut Summary, home: &Path) {
    let Ok(text) = fs::read_to_string(home.join(".gitconfig")) else {
        return;
    };
    let stanzas = parse_include_stanzas(&text);
    if stanzas.is_empty() {
        return;
    }
    let orgs = home.join(".config/git/orgs.config");
    let existing = fs::read_to_string(&orgs).unwrap_or_default();
    let fresh: Vec<String> = stanzas
        .into_iter()
        .filter(|s| match stanza_path(s) {
            Some(p) => !existing.contains(p.as_str()),
            None => true,
        })
        .collect();
    if fresh.is_empty() {
        summary.skipped.push("orgs (includeIf ya migrados)".into());
        return;
    }
    let mut body = existing;
    if body.is_empty() {
        body.push_str("# Managed by prim: per-org git identities (includeIf chain).\n");
    }
    for s in fresh {
        body.push_str(&s);
        body.push('\n');
    }
    if let Some(dir) = orgs.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if fs::write(&orgs, body).is_ok() {
        println!(
            "[prim] includeIf de ~/.gitconfig migrados a {} (el original queda respaldado por la activacion).",
            orgs.display()
        );
        summary.ran.push("orgs (migracion de includeIf)".into());
    }
}

/// Global [user] name/email parsed from a ~/.gitconfig body. A half-written
/// user section still counts (the missing half falls back to the
/// placeholder): the file exists, so the machine was configured already and
/// asking again would be noise.
fn parse_git_identity(text: &str) -> Option<(String, String)> {
    let mut name: Option<String> = None;
    let mut email: Option<String> = None;
    let mut in_user = false;
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix('[') {
            in_user = rest.starts_with("user");
            continue;
        }
        let Some((k, v)) = t.split_once('=') else {
            continue;
        };
        if !in_user {
            continue;
        }
        let v = v.trim().trim_matches('"');
        match k.trim() {
            "name" if !v.is_empty() => name = Some(v.into()),
            "email" if !v.is_empty() => email = Some(v.into()),
            _ => {}
        }
    }
    if name.is_none() && email.is_none() {
        return None;
    }
    Some((
        name.unwrap_or_else(|| PLACEHOLDER_NAME.into()),
        email.unwrap_or_else(|| PLACEHOLDER_EMAIL.into()),
    ))
}

/// Verbatim [includeIf \"...\"] stanzas (header + indented body lines) from
/// a ~/.gitconfig body, in order.
fn parse_include_stanzas(text: &str) -> Vec<String> {
    let mut stanzas: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in text.lines() {
        if line.trim_start().starts_with("[includeIf") {
            if !current.is_empty() {
                stanzas.push(current.join("\n"));
            }
            current = vec![line];
        } else if !current.is_empty() {
            if line.trim_start().starts_with('[') {
                stanzas.push(current.join("\n"));
                current = Vec::new();
            } else {
                current.push(line);
            }
        }
    }
    if !current.is_empty() {
        stanzas.push(current.join("\n"));
    }
    stanzas
}

/// The `path = ...` value of one stanza, when present.
fn stanza_path(stanza: &str) -> Option<String> {
    stanza.lines().find_map(|l| {
        l.trim()
            .strip_prefix("path")?
            .split_once('=')
            .map(|(_, v)| v.trim().to_string())
    })
}

/// ~/.config/prim/identity.toml for a given home.
pub(crate) fn identity_path(home: &Path) -> PathBuf {
    home.join(".config/prim/identity.toml")
}

/// Body of identity.toml for a name/email pair.
fn identity_toml(name: &str, email: &str) -> String {
    let header = "# Identidad estatica de prim (git/home-manager). Edita a mano; prim nunca la sobreescribe.";
    format!("{header}\ngit_name = \"{name}\"\ngit_email = \"{email}\"\n")
}

/// Download `url` into `dest` with curl (IPv4 forced, like everywhere else).
/// Local twin of lifecycle::curl_to_file — that one is private and
#[cfg(test)]
mod tests {
    use super::*;

    /// Scenario table for the identity step: (label, tty, answers, exact
    /// expected file body). One logic = one test; no branching in the test.
    #[test]
    fn identity_step_matrix() {
        let scenarios: Vec<(&str, bool, Vec<&str>, String)> = vec![
            (
                "non-tty placeholders",
                false,
                vec![],
                concat!(
                    "# Identidad estatica de prim (git/home-manager). Edita a mano; prim nunca la sobreescribe.\n",
                    "git_name = \"devlab\"\n",
                    "git_email = \"devlab@localhost\"\n"
                )
                .to_string(),
            ),
            (
                "tty answers",
                true,
                vec!["Ada Lovelace", "ada@lovelace.dev"],
                concat!(
                    "# Identidad estatica de prim (git/home-manager). Edita a mano; prim nunca la sobreescribe.\n",
                    "git_name = \"Ada Lovelace\"\n",
                    "git_email = \"ada@lovelace.dev\"\n"
                )
                .to_string(),
            ),
            (
                "tty empty answers fall back to placeholders",
                true,
                vec!["", ""],
                concat!(
                    "# Identidad estatica de prim (git/home-manager). Edita a mano; prim nunca la sobreescribe.\n",
                    "git_name = \"devlab\"\n",
                    "git_email = \"devlab@localhost\"\n"
                )
                .to_string(),
            ),
        ];
        for (i, (name, tty, answers, expected)) in scenarios.into_iter().enumerate() {
            let home = std::env::temp_dir().join(format!("prim-boot-test-{i}"));
            let _ = fs::remove_dir_all(&home);
            fs::create_dir_all(&home).unwrap();
            let mut it = answers.into_iter().map(|a| Some(a.to_string()));
            let mut ask = move |_: &str| it.next().flatten();
            identity_step(&mut Summary::default(), &home, tty, &mut ask).unwrap();
            assert_eq!(
                fs::read_to_string(identity_path(&home)).unwrap(),
                expected,
                "{name}"
            );
            fs::remove_dir_all(&home).unwrap();
        }
    }

    /// Existing identity is kept byte-identical and never prompted for.
    #[test]
    fn identity_step_never_overwrites() {
        let home = std::env::temp_dir().join("prim-boot-test-keep");
        let _ = fs::remove_dir_all(&home);
        let path = identity_path(&home);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "git_name = \"previa\"\ngit_email = \"previa@x\"\n").unwrap();
        let mut asked = false;
        let mut ask = |_: &str| {
            asked = true;
            Some("no-debe-usarse".into())
        };
        identity_step(&mut Summary::default(), &home, true, &mut ask).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "git_name = \"previa\"\ngit_email = \"previa@x\"\n"
        );
        assert!(!asked);
        fs::remove_dir_all(&home).unwrap();
    }

    /// The apt step runs nothing under its two system-independent guards:
    /// the RIG_SKIP_APT flag and the non-Debian check.
    #[test]
    fn os_step_guards_skip_without_apt() {
        let scenarios = [("env flag", true, false), ("non-debian", false, false)];
        for (name, skip_apt, is_debian) in scenarios {
            let mut s = Summary::default();
            os_step(&mut s, skip_apt, is_debian);
            assert_eq!(s.ran.len(), 0, "{name}");
            assert_eq!(s.skipped.len(), 1, "{name}");
        }
    }

    /// A real global identity is adopted; placeholders and halves are not.
    #[test]
    fn parse_git_identity_rules() {
        let real = "[user]\n    name = \"Johan Marin\"\n    email = \"j@x.com\"\n[core]\n    editor = hx\n";
        assert_eq!(
            parse_git_identity(real),
            Some(("Johan Marin".into(), "j@x.com".into()))
        );
        // A chezmoi-era placeholder is still an existing identity: the
        // machine was configured, so prim adopts it instead of asking.
        let placeholder = "[user]\n    name = \"devlab\"\n    email = \"devlab@localhost\"\n";
        assert_eq!(
            parse_git_identity(placeholder),
            Some((PLACEHOLDER_NAME.into(), PLACEHOLDER_EMAIL.into()))
        );
        // Half a user section: the missing half falls back to the placeholder.
        assert_eq!(
            parse_git_identity("[user]\n    name = \"x\"\n"),
            Some(("x".into(), PLACEHOLDER_EMAIL.into()))
        );
        // No user section at all: nothing to adopt.
        assert_eq!(parse_git_identity("[core]\n    editor = hx\n"), None);
    }

    /// includeIf stanzas come out verbatim with their path, and only those.
    #[test]
    fn parse_include_stanzas_rules() {
        let cfg = "[user]\n name = a\n[includeIf \"gitdir:~/work/knt/\"]\n\tpath = /home/x/.gitconfig-konecta\n[core]\n editor = hx\n[includeIf \"gitdir:~/work/gt/\"]\n path = /home/x/.gitconfig-gt\n";
        let stanzas = parse_include_stanzas(cfg);
        assert_eq!(stanzas.len(), 2);
        assert_eq!(
            stanza_path(&stanzas[0]).as_deref(),
            Some("/home/x/.gitconfig-konecta")
        );
        assert_eq!(
            stanza_path(&stanzas[1]).as_deref(),
            Some("/home/x/.gitconfig-gt")
        );
        assert!(parse_include_stanzas("[user]\n name = a\n").is_empty());
    }
}

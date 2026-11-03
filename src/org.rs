//! prim org — manage organizations (one git account per org).
//!
//! An "organization" is a directory under ~/work with its own git identity:
//! a dedicated SSH key (~/.ssh/id_ed25519_<org>) and a direnv environment
//! (.envrc + .env.local) exporting GIT_SSH_COMMAND bound to that key. Entering
//! ~/work/<org> makes every git operation use that org's account (GitHub,
//! Bitbucket, ...). This module is the seed of prim's management surface:
//! prim ends up as a binary that manages projects; `org` is its first command.
//!
//! Subcommands:
//!   prim org new <NAME>         create/refresh the org environment (idempotent)
//!   prim org secret <NAME> K=V  upsert one variable in the org's .env.local
//!   prim org list               list existing organizations

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Entry point for `prim org ...`. Exits on error.
pub fn run(args: &[String]) {
    let strs: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match strs.as_slice() {
        ["new", name] => new_org(name),
        ["secret", name, kv] => secret(name, kv),
        ["list"] => list(),
        _ => usage_error(),
    };
    if let Err(e) = result {
        eprintln!("[prim org] error: {e}");
        std::process::exit(1);
    }
}

fn usage_error() -> io::Result<()> {
    eprintln!(
        "uso: prim org <comando>\n  \
            new <nombre>           crea/actualiza la organizacion (idempotente)\n  \
            secret <nombre> K=V    agrega o actualiza una variable en .env.local\n  \
            list                   lista las organizaciones"
    );
    std::process::exit(2);
}

// --- paths ------------------------------------------------------------------

fn home() -> io::Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| io::Error::other("no se pudo determinar $HOME"))
}

fn org_dir(name: &str) -> io::Result<PathBuf> {
    Ok(home()?.join("work").join(name))
}

fn ssh_key(name: &str) -> io::Result<PathBuf> {
    Ok(home()?.join(".ssh").join(format!("id_ed25519_{name}")))
}

/// Valid org name: `[a-zA-Z0-9_-]+`. Keeps directory and key names sane.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

// --- org new ----------------------------------------------------------------

/// Create or refresh an organization: ~/work/<name>, .envrc, SSH key,
/// .env.local (never clobbered) + GIT_SSH_COMMAND, direnv allow. Idempotent:
/// re-running preserves the existing key and every stored variable.
fn new_org(name: &str) -> io::Result<()> {
    if !valid_name(name) {
        return Err(io::Error::other(format!(
            "nombre de organizacion invalido: '{name}' (usa [a-zA-Z0-9_-])"
        )));
    }
    let home = home()?;
    let dir = org_dir(name)?;
    let key = ssh_key(name)?;
    let ssh_dir = home.join(".ssh");

    fs::create_dir_all(&dir)?;
    fs::create_dir_all(&ssh_dir)?;
    chmod(&ssh_dir, 0o700);

    // direnv loads .env.local if present (never fails when absent).
    let envrc = dir.join(".envrc");
    if !envrc.exists() {
        fs::write(&envrc, "dotenv_if_exists .env.local\n")?;
    }

    // Secrets file: create empty with 0600, never overwrite.
    let envlocal = dir.join(".env.local");
    if !envlocal.exists() {
        fs::write(&envlocal, "")?;
    }
    chmod(&envlocal, 0o600);

    // One key per org; generate without passphrase (automation-friendly,
    // per-machine and per-org, never reused).
    if key.exists() {
        println!("[prim org] la clave SSH ya existe; se conserva.");
    } else {
        let ok = Command::new("ssh-keygen")
            .args(["-t", "ed25519", "-C", name, "-N", "", "-f"])
            .arg(&key)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            return Err(io::Error::other("fallo ssh-keygen"));
        }
        chmod(&key, 0o600);
        chmod(&key.with_extension("pub"), 0o644);
    }

    // Bind git to this org's key via direnv; append only if not already set.
    let line = format!(
        "GIT_SSH_COMMAND=\"ssh -i {} -o IdentitiesOnly=yes\"",
        key.display()
    );
    let mut content = fs::read_to_string(&envlocal)?;
    if !content.lines().any(|l| l.starts_with("GIT_SSH_COMMAND=")) {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&line);
        content.push('\n');
        fs::write(&envlocal, &content)?;
    } else {
        println!("[prim org] GIT_SSH_COMMAND ya existe; no se modifica.");
    }

    direnv_allow(&dir);

    let pubkey = fs::read_to_string(key.with_extension("pub"))
        .unwrap_or_else(|_| "(no se pudo leer la clave publica)".into());
    println!();
    println!("==========================================");
    println!(" ORGANIZACION CONFIGURADA: {name}");
    println!("==========================================");
    println!("Directorio:     {}", dir.display());
    println!("Clave privada:  {}", key.display());
    println!("Entorno direnv: {}", envrc.display());
    println!();
    println!("CLAVE PUBLICA (registrala en la cuenta/org de GitHub, Bitbucket, ...):");
    println!("{pubkey}");
    println!("Con la clave registrada, clona dentro de la org y direnv aplicara");
    println!("GIT_SSH_COMMAND automaticamente:  git clone git@github.com:{name}/repo.git");
    Ok(())
}

// --- org secret -------------------------------------------------------------

/// Upsert a single `KEY=VALUE` in the org's .env.local, leaving every other
/// line untouched. Reloads direnv so a hooked shell picks the change up.
fn secret(name: &str, kv: &str) -> io::Result<()> {
    if !valid_name(name) {
        return Err(io::Error::other(format!(
            "nombre de organizacion invalido: '{name}' (usa [a-zA-Z0-9_-])"
        )));
    }
    let (key, value) = kv
        .split_once('=')
        .ok_or_else(|| io::Error::other("formato esperado: KEY=VALUE"))?;
    if !valid_env_key(key) {
        return Err(io::Error::other(format!(
            "nombre de variable invalido: '{key}' (usa [A-Za-z_][A-Za-z0-9_]*)"
        )));
    }
    let dir = org_dir(name)?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "la organizacion no existe: {} (corre: prim org new {name})",
            dir.display()
        )));
    }

    let envlocal = dir.join(".env.local");
    let content = if envlocal.exists() {
        fs::read_to_string(&envlocal)?
    } else {
        String::new()
    };

    // Quote values with whitespace so direnv's dotenv parser keeps them whole.
    let new_line = if value.chars().any(|c| c.is_whitespace()) {
        format!("{key}=\"{value}\"")
    } else {
        format!("{key}={value}")
    };

    // Replace the first `KEY=` line, or append. Other lines stay byte-for-byte.
    let mut replaced = false;
    let mut out = String::with_capacity(content.len() + new_line.len() + 1);
    for line in content.split_inclusive('\n') {
        if !replaced && line.starts_with(&format!("{key}=")) {
            out.push_str(&new_line);
            out.push('\n');
            replaced = true;
        } else {
            out.push_str(line);
        }
    }
    if !replaced {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&new_line);
        out.push('\n');
    }

    fs::write(&envlocal, &out)?;
    chmod(&envlocal, 0o600);
    direnv_allow(&dir);
    println!(
        "[prim org] {name}: {} {}",
        if replaced { "actualizada" } else { "agregada" },
        key
    );
    println!("[prim org] {}", envlocal.display());
    Ok(())
}

/// Valid .env variable name: `[A-Za-z_][A-Za-z0-9_]*`.
fn valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// --- org list ---------------------------------------------------------------

/// List organizations: directories under ~/work that carry an .envrc.
fn list() -> io::Result<()> {
    let work = home()?.join("work");
    if !work.is_dir() {
        println!(
            "[prim org] no hay organizaciones (~{} no existe).",
            work.display()
        );
        return Ok(());
    }
    let mut orgs: Vec<String> = fs::read_dir(&work)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join(".envrc").is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    orgs.sort();
    if orgs.is_empty() {
        println!("[prim org] no hay organizaciones. Crea una: prim org new <nombre>");
    } else {
        for org in orgs {
            println!("{org}");
        }
    }
    Ok(())
}

// --- helpers ----------------------------------------------------------------

fn chmod(path: &Path, mode: u32) {
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(mode));
}

/// Mark the org's .envrc as allowed so the direnv hook loads it (and reloads
/// on the next prompt). Non-fatal: only a note when direnv is missing.
fn direnv_allow(dir: &Path) {
    let ok = Command::new("direnv")
        .arg("allow")
        .arg(dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        println!("[prim org] direnv: entorno autorizado (se recarga al entrar a la carpeta).");
    } else {
        println!(
            "[prim org] direnv no disponible; ejecuta 'direnv allow {}' a mano.",
            dir.display()
        );
    }
}

//! prim org — manage organizations (one git account per org).
//!
//! An "organization" is a directory under ~/work with its own git identity:
//! a dedicated SSH key (~/.ssh/id_ed25519_<org>), a per-org ~/.gitconfig-<org>
//! (user + core.sshCommand bound to that key, reached from the global config
//! via includeIf in ~/.config/git/orgs.config), and a direnv environment
//! (.envrc + .env.local) for the org's other environment and secrets. Any git
//! operation under ~/work/<org> uses that org's account (GitHub, Bitbucket,
//! ...) — no direnv needed, it is git-native. This module is the seed of
//! prim's management surface: prim ends up as a binary that manages projects;
//! `org` is its first command.
//!
//! Subcommands:
//!   prim org new <NAME>         create/refresh the org environment (idempotent)
//!   prim org secret <NAME> K=V  upsert one variable in the org's .env.local
//!   prim org list               list existing organizations

use std::fs;
use std::io::{self, IsTerminal};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Entry point for `prim org ...`. Exits on error.
pub fn run(args: &[String]) {
    let strs: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match strs.as_slice() {
        ["new", name] => new_org(name),
        ["secret", name] => edit_secret(name),
        ["secret", name, kv] => secret(name, kv),
        ["edit", name] => edit_secret(name),
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
            secret <nombre>        abre .env.local en el editor (todos los secretos)\n  \
            edit <nombre>          alias: abre .env.local en el editor\n  \
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
/// .env.local (never clobbered), ~/.gitconfig-<name> + includeIf wiring,
/// direnv allow. Idempotent: re-running preserves the existing key, the
/// gitconfig and every stored variable.
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

    // Per-org git identity (user's includeIf pattern): ~/.gitconfig-<org>
    // carries [user] + [core] sshCommand bound to this org's key. -F /dev/null
    // isolates from ~/.ssh/config; IdentitiesOnly keeps the agent from offering
    // other keys. The global gitconfig (home-manager) includes orgs.config,
    // which holds one includeIf "gitdir:~/work/<org>/" per org. Re-running in
    // a terminal re-prompts identity with current values as defaults.
    let gitconfig = home.join(format!(".gitconfig-{name}"));
    let exists = gitconfig.exists();
    let (gname, gemail) = org_git_identity(&home, name, exists);
    if !exists {
        write_org_gitconfig(&gitconfig, name, &gname, &gemail)?;
    } else if gname != git_user_field(&gitconfig, "name")
        || gemail != git_user_field(&gitconfig, "email")
    {
        write_org_gitconfig(&gitconfig, name, &gname, &gemail)?;
        println!("[prim org] ~/.gitconfig-{name}: identidad actualizada.");
    } else {
        println!("[prim org] ~/.gitconfig-{name} ya existe; se conserva.");
    }
    register_org_include(&home, name)?;

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
    println!("Con la clave registrada, clona dentro de la org — git usara la");
    println!("identidad y la clave de {name} automaticamente (includeIf):");
    println!("  git clone git@github.com:{name}/repo.git");
    if io::stdin().is_terminal() {
        println!();
        println!("[prim org] abriendo .env.local para los secretos; cerrá el editor");
        println!("[prim org] para activarlos (direnv allow + reload automaticos).");
        edit_env_local(&home, name)?;
    } else {
        println!("[prim org] secretos: corre 'prim org secret {name}' (sin KEY=VALUE)");
        println!("[prim org] para abrir el editor y cargarlos todos de una vez.");
    }
    Ok(())
}

/// Git identity for a new org: prompt when interactive (defaults from
/// ~/.config/prim/identity.toml, placeholders as last resort).
/// Read one `key = value` line from a gitconfig's [user] section (first
/// match, quotes trimmed); "" when absent.
fn git_user_field(path: &Path, key: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .find_map(|l| l.trim_start().strip_prefix(&format!("{key} =")))
        .map(|v| v.trim().trim_matches('"').to_string())
        .unwrap_or_default()
}

/// Git identity for an org: prompt when interactive (defaults: the org's
/// current ~/.gitconfig-<org> values, then ~/.config/prim/identity.toml,
/// then placeholders — so re-running SUGGESTS the current values and Enter
/// keeps them). Non-interactive returns the defaults untouched.
fn org_git_identity(home: &Path, name: &str, _exists: bool) -> (String, String) {
    let gitconfig = home.join(format!(".gitconfig-{name}"));
    let cur = |k: &str| {
        let v = git_user_field(&gitconfig, k);
        (!v.is_empty()).then_some(v)
    };
    let text = fs::read_to_string(home.join(".config/prim/identity.toml")).unwrap_or_default();
    let idt = |k: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(&format!("{k} =")))
            .map(|v| v.trim().trim_matches('"').to_string())
            .filter(|v| !v.is_empty())
    };
    let dname = cur("name")
        .or_else(|| idt("git_name"))
        .unwrap_or_else(|| "devlab".into());
    let demail = cur("email")
        .or_else(|| idt("git_email"))
        .unwrap_or_else(|| "devlab@localhost".into());
    if !io::stdin().is_terminal() {
        return (dname, demail);
    }
    let prompt = |q: &str, d: &str| -> String {
        print!("[prim org] {q} [{d}]: ");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_ok() {
            let t = line.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
        d.to_string()
    };
    (
        prompt("nombre git de la org", &dname),
        prompt("email git de la org", &demail),
    )
}

/// Write the org's gitconfig: [user] identity + [core] sshCommand bound to
/// the org key (-F /dev/null isolates from ~/.ssh/config, IdentitiesOnly
/// keeps the agent from offering other keys).
fn write_org_gitconfig(path: &Path, name: &str, gname: &str, gemail: &str) -> io::Result<()> {
    fs::write(
        path,
        format!(
            "[user]\n    name = {gname}\n    email = {gemail}\n\n[core]\n    # clave de la organizacion {name} (prim org)\n    sshCommand = \"ssh -i ~/.ssh/id_ed25519_{name} -o IdentitiesOnly=yes -F /dev/null\"\n"
        ),
    )
}

/// Append this org's includeIf block to ~/.config/git/orgs.config (prim-
/// managed; the home-manager global gitconfig includes this file). Idempotent.
fn register_org_include(home: &Path, name: &str) -> io::Result<()> {
    let dir = home.join(".config").join("git");
    fs::create_dir_all(&dir)?;
    let path = dir.join("orgs.config");
    let mut content = fs::read_to_string(&path).unwrap_or_default();
    let needle = format!("gitdir:~/work/{name}/");
    if content.contains(&needle) {
        return Ok(());
    }
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&format!(
        "[includeIf \"gitdir:~/work/{name}/\"]\n    path = ~/.gitconfig-{name}\n"
    ));
    fs::write(&path, content)
}

// --- editor flow ------------------------------------------------------------

/// Open the org's .env.local in the user's editor ($VISUAL, $EDITOR, hx,
/// nano — first available) so ALL secrets can be added at once, then run
/// direnv allow + reload automatically (the human path; KEY=VALUE stays for
/// scripts).
fn edit_secret(name: &str) -> io::Result<()> {
    home().and_then(|h| edit_env_local(&h, name))
}

fn edit_env_local(home: &Path, name: &str) -> io::Result<()> {
    let dir = home.join("work").join(name);
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "la organizacion no existe: {} (corre: prim org new {name})",
            dir.display()
        )));
    }
    let envlocal = dir.join(".env.local");
    if !envlocal.exists() {
        fs::write(&envlocal, "")?;
    }
    chmod(&envlocal, 0o600);
    println!("[prim org] editando {}", envlocal.display());
    if !open_editor(&envlocal) {
        println!(
            "[prim org] sin editor disponible (VISUAL/EDITOR/hx/nano); edita {} a mano.",
            envlocal.display()
        );
    }
    direnv_allow(&dir);
    direnv_reload(&dir);
    Ok(())
}

/// Run the first available editor on `path` (VISUAL > EDITOR > hx > nano).
/// True when one ran (any exit code — the file was open for editing).
fn open_editor(path: &Path) -> bool {
    let mut candidates: Vec<String> = ["VISUAL", "EDITOR"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .filter(|v| !v.is_empty())
        .collect();
    candidates.extend(["hx".to_string(), "nano".to_string()]);
    for ed in &candidates {
        match Command::new(ed).arg(path).status() {
            Ok(_) => return true,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(_) => continue,
        }
    }
    false
}

/// Best-effort `direnv reload` from inside the org dir (only works in a
/// hooked shell; otherwise the allow message covers the next cd).
fn direnv_reload(dir: &Path) {
    let ok = Command::new("direnv")
        .arg("reload")
        .current_dir(dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        println!("[prim org] direnv: entorno recargado.");
    } else {
        println!("[prim org] direnv reload omitido (se recargara al entrar a la carpeta).");
    }
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

//! prim lifecycle — install/update/uninstall/status across components.
//!
//! prim owns its binary: the root install.sh copies it to ~/.local/bin/prim
//! and `prim update` self-updates it from the repo's `latest` release before
//! forwarding the verb to every component installed on the machine. A
//! component counts as installed when a receipt exists
//! (~/.local/share/prim/<key>.receipt.json, written by the component's own
//! installer) or, for machines set up before receipts, a legacy marker
//! (~/.local/share/chezmoi for rig, ~/.agents/skills for harness).
//!
//! Uninstall is receipt-driven by design: without a receipt prim cannot know
//! what an installer touched, so it refuses and points the user at
//! `prim update`, which makes the installer regenerate the receipt. Uninstall
//! never touches user data — each component's own script owns that contract.
//!
//! Subcommands:
//!   prim update               self-update + forward "update" to components
//!   prim uninstall [keys...]  forward "uninstall" to the named components
//!   prim status               prim version + per-receipt summary

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{run_component, select_from, Component, COMPONENTS};

/// Base URL of the prim "latest" release assets (self-update source).
const RELEASE_BASE: &str = "https://github.com/bit-crew/prim/releases/download/latest";

// --- receipts ----------------------------------------------------------------

/// Receipts live at ~/.local/share/prim/<key>.receipt.json (one per component).
pub fn receipts_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".local/share/prim"))
}

/// True when <key>.receipt.json exists in the receipts dir.
fn has_receipt(key: &str) -> bool {
    receipts_dir()
        .map(|d| d.join(format!("{key}.receipt.json")).is_file())
        .unwrap_or(false)
}

/// Components present on this machine: receipt file first, legacy marker
/// (~/.local/share/chezmoi for rig, ~/.agents/skills for harness) as fallback
/// for installs that predate receipts.
pub fn installed_components() -> Vec<Component> {
    let Some(home) = dirs::home_dir() else {
        return vec![];
    };
    COMPONENTS
        .iter()
        .copied()
        .filter(|c| {
            has_receipt(c.key)
                || match c.key {
                    "rig" => home.join(".local/share/chezmoi").exists(),
                    "harness" => home.join(".agents/skills").exists(),
                    _ => false,
                }
        })
        .collect()
}

// --- update ------------------------------------------------------------------

/// `prim update`: self-update prim, then forward "update" to every installed
/// component. Actions touch only components present on the machine.
pub fn update() {
    self_update();
    let installed = installed_components();
    if installed.is_empty() {
        println!("[prim] no hay componentes instalados; nada que actualizar.");
        return;
    }
    let mut failures = 0;
    for comp in &installed {
        let Some(path) = comp.install_path else {
            eprintln!("[prim] {} aun no es instalable; se omite.", comp.key);
            failures += 1;
            continue;
        };
        println!("[prim] actualizando {}...", comp.key);
        if !run_component(path, &["update"]) {
            eprintln!("[prim] fallo actualizando {} (continuo).", comp.key);
            failures += 1;
        }
    }
    if failures > 0 {
        std::process::exit(1);
    }
    println!("[prim] listo.");
}

/// Self-update ~/.local/bin/prim from the repo's `latest` release: download,
/// verify sha256, skip with a note when the installed binary already matches,
/// else atomically replace (tmp file in the same dir + rename) and chmod 755.
/// Non-fatal: on any failure it notes the problem and returns so component
/// updates still run.
fn self_update() {
    let suffix = match std::env::consts::ARCH {
        "x86_64" => "x86_64-linux",
        "aarch64" => "aarch64-linux",
        arch => {
            eprintln!("[prim] sin binario publicado para {arch}; no se actualiza prim.");
            return;
        }
    };
    let Some(home) = dirs::home_dir() else {
        eprintln!("[prim] no se pudo determinar $HOME; no se actualiza prim.");
        return;
    };
    let bin_dir = home.join(".local/bin");
    if let Err(e) = fs::create_dir_all(&bin_dir) {
        eprintln!("[prim] no se pudo crear {}: {e}", bin_dir.display());
        return;
    }
    let target = bin_dir.join("prim");
    let tmp = bin_dir.join(".prim.new");

    let bin_url = format!("{RELEASE_BASE}/prim-{suffix}");
    println!("[prim] buscando actualizacion de prim ({suffix})...");
    if !curl_to_file(&bin_url, &tmp) {
        let _ = fs::remove_file(&tmp);
        eprintln!("[prim] no se pudo descargar {bin_url}; no se actualiza prim.");
        return;
    }
    if !checksum_ok(&bin_url, &tmp) {
        let _ = fs::remove_file(&tmp);
        return;
    }
    if let (Some(new), Some(current)) = (sha256_of(&tmp), sha256_of(&target)) {
        if new == current {
            let _ = fs::remove_file(&tmp);
            println!("[prim] prim ya esta en la ultima version (checksum identico).");
            return;
        }
    }
    if let Err(e) = fs::rename(&tmp, &target) {
        let _ = fs::remove_file(&tmp);
        eprintln!("[prim] no se pudo instalar {}: {e}", target.display());
        return;
    }
    let _ = fs::set_permissions(&target, fs::Permissions::from_mode(0o755));
    println!("[prim] prim actualizado: {}", target.display());
}

/// Verify `tmp` against the published `.sha256`. A missing checksum file or a
/// missing sha256 tool downgrades to a skip-verify note; a mismatch aborts.
fn checksum_ok(bin_url: &str, tmp: &Path) -> bool {
    let sum_path = tmp.with_file_name(".prim.new.sha256");
    let mut expected = None;
    if curl_to_file(&format!("{bin_url}.sha256"), &sum_path) {
        expected = fs::read_to_string(&sum_path)
            .ok()
            .and_then(|t| t.split_whitespace().next().map(str::to_string));
    } else {
        eprintln!("[prim] no se pudo descargar el .sha256 (continuo sin verificar).");
    }
    let _ = fs::remove_file(&sum_path);

    let Some(actual) = sha256_of(tmp) else {
        eprintln!("[prim] ni sha256sum ni shasum disponibles; no se verifica el binario.");
        return true;
    };
    match expected.as_deref() {
        Some(e) if e == actual => {
            println!("[prim] checksum verificado.");
            true
        }
        Some(_) => {
            eprintln!("[prim] checksum no coincide; no se actualiza prim.");
            false
        }
        // No checksum available anywhere: skip-verify, keep going.
        None => true,
    }
}

/// sha256 hex digest of `path` via sha256sum (shasum fallback); None when
/// neither tool is available.
fn sha256_of(path: &Path) -> Option<String> {
    let tools: [(&str, &[&str]); 2] = [("sha256sum", &[]), ("shasum", &["-a", "256"])];
    for (cmd, args) in tools {
        if let Ok(out) = Command::new(cmd).args(args).arg(path).output() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if let Some(first) = stdout.split_whitespace().next() {
                return Some(first.to_string());
            }
        }
    }
    None
}

/// Download `url` into `dest` with curl (IPv4 forced, like everywhere else).
fn curl_to_file(url: &str, dest: &Path) -> bool {
    Command::new("curl")
        .args(["-4", "-fsSL", "-o"])
        .arg(dest)
        .arg(url)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// --- uninstall ---------------------------------------------------------------

/// `prim uninstall [keys...]`: forward "uninstall" to the named components.
/// Explicit keys required (TTY gets a numbered picker); a key without a
/// receipt is refused — prim cannot know what an installer touched without
/// one, and `prim update` regenerates it.
pub fn uninstall(args: &[String]) {
    let keys = if !args.is_empty() {
        resolve_keys(args)
    } else if stdin_is_tty() {
        pick_uninstall()
    } else {
        eprintln!("uso: prim uninstall <componente>... (ej. prim uninstall rig harness)");
        std::process::exit(2);
    };
    if keys.is_empty() {
        eprintln!("[prim] nothing selected");
        std::process::exit(1);
    }

    let mut failures = 0;
    for comp in COMPONENTS.iter().filter(|c| keys.contains(&c.key)) {
        let Some(path) = comp.install_path else {
            eprintln!(
                "[prim] {} aun no es instalable; nada que desinstalar.",
                comp.key
            );
            failures += 1;
            continue;
        };
        if !has_receipt(comp.key) {
            eprintln!(
                "[prim] {}: sin receipt no se desinstala — corre 'prim update' una vez para generar el receipt.",
                comp.key
            );
            failures += 1;
            continue;
        }
        println!("[prim] desinstalando {}...", comp.key);
        if !run_component(path, &["uninstall"]) {
            eprintln!("[prim] fallo desinstalando {} (continuo).", comp.key);
            failures += 1;
        }
    }
    if failures > 0 {
        std::process::exit(1);
    }
    println!("[prim] listo.");
}

/// Explicit keys only (case-insensitive); unknown keys are a usage error.
fn resolve_keys(args: &[String]) -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = Vec::new();
    for arg in args {
        let lower = arg.to_lowercase();
        match COMPONENTS.iter().find(|c| c.key == lower) {
            Some(c) if !keys.contains(&c.key) => keys.push(c.key),
            Some(_) => {}
            None => {
                let valid = COMPONENTS
                    .iter()
                    .map(|c| c.key)
                    .collect::<Vec<_>>()
                    .join(", ");
                eprintln!("[prim] componente desconocido: {arg} (validos: {valid})");
                std::process::exit(2);
            }
        }
    }
    keys
}

/// Interactive picker over installed components (TTY only). Exits 0 when
/// nothing is installed — there is simply nothing to uninstall.
fn pick_uninstall() -> Vec<&'static str> {
    let installed = installed_components();
    if installed.is_empty() {
        println!("[prim] no hay componentes instalados.");
        std::process::exit(0);
    }
    println!("prim — desinstalacion\n");
    for (i, c) in installed.iter().enumerate() {
        println!("  {}) {} — {}", i + 1, c.key, c.name);
    }
    println!("  a) todo");
    print!("\nElige componentes a desinstalar (ej. 1 2, o 'a'): ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return vec![];
    }
    select_from(&installed, line.trim())
}

/// True when stdin is a terminal (decides picker vs usage error).
fn stdin_is_tty() -> bool {
    use std::io::IsTerminal;
    io::stdin().is_terminal()
}

// --- status ------------------------------------------------------------------

/// `prim status`: prim version plus one line per receipt (key, profile,
/// installed_at). The component installer writes the JSON; prim only scans it.
pub fn status() {
    println!("prim {}", env!("CARGO_PKG_VERSION"));
    let Some(dir) = receipts_dir() else {
        println!("[prim] sin receipts; nada instalado todavia.");
        return;
    };
    let mut receipts: Vec<String> = fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.ends_with(".receipt.json"))
                .collect()
        })
        .unwrap_or_default();
    receipts.sort();
    if receipts.is_empty() {
        println!("[prim] sin receipts; nada instalado todavia.");
        return;
    }
    for name in receipts {
        let key = name.trim_end_matches(".receipt.json");
        let text = fs::read_to_string(dir.join(&name)).unwrap_or_default();
        let profile = json_field(&text, "profile").unwrap_or_else(|| "-".into());
        let installed_at = json_field(&text, "installed_at_iso").unwrap_or_else(|| "-".into());
        println!("{key}: profile={profile} installed_at={installed_at}");
    }
}

/// Extract a string field from JSON text by scanning for `"field"` and the
/// first quoted value after the colon. Receipts are tiny; no parser needed.
fn json_field(text: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let rest = &text[text.find(&key)? + key.len()..];
    let after_colon = &rest[rest.find(':')? + 1..];
    let start = after_colon.find('"')? + 1;
    let end = start + after_colon[start..].find('"')?;
    Some(after_colon[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Field scan finds quoted values, misses absent fields, and needs whole
    /// quoted keys ("at" must not match inside "installed_at").
    #[test]
    fn json_field_scans_receipt_fields() {
        let receipt = r#"{"key":"rig","profile":"dc1","installed_at_iso":"2026-11-03T10:00:00Z"}"#;
        assert_eq!(json_field(receipt, "profile").as_deref(), Some("dc1"));
        assert_eq!(
            json_field(receipt, "installed_at_iso").as_deref(),
            Some("2026-11-03T10:00:00Z")
        );
        assert_eq!(json_field(receipt, "at"), None);
        assert_eq!(json_field(receipt, "missing"), None);
        assert_eq!(json_field("", "profile"), None);
    }
}

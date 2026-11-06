//! prim doctor — read-only diagnostics for the prim/Nix stack (P5, S1/S5).
//!
//! `prim doctor` inspects the machine and prints Spanish advisory lines:
//! nix presence/version/experimental features, the rig receipt (written by
//! prim itself since P3), global flake reachability (network-tolerant,
//! bounded by a timeout), /nix/store size, direnv GC roots of active
//! per-repo environments, and pre-Nix leftovers (chezmoi stack, legacy
//! ~/.local/bin binaries now owned by the Nix profile, google-cloud-sdk).
//! Leftovers are advisory only: doctor never removes anything — nix GC owns
//! removal (S5) and the final per-machine cleanup is P6 with consent.
//! When the files component's receipt exists, a `files` section (S11h)
//! reports the SMB-over-Tailscale share, also best-effort and read-only.
//!
//! `prim doctor --prune` additionally runs `nix-collect-garbage
//! --delete-older-than 30d`: it prints the exact command first and the
//! PRIM_DRY_RUN=1 guard (same semantics as src/nix.rs) prints the plan
//! instead of running it, so the flow can be exercised in sandboxes.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::lifecycle::{json_field, receipts_dir};
use crate::nix;

/// The global rig flake doctor probes (same ref src/nix.rs activates; kept
/// local here because the const there is private).
const RIG_FLAKE_REF: &str = "github:bit-crew/prim?dir=rig/nix";

/// Upper bound for the flake reachability probe: network/eval can be slow;
/// doctor is tolerant and just notes a timeout.
const FLAKE_TIMEOUT_SECS: u64 = 30;

/// Garbage collection policy (S1/S5): drop store paths not reachable from
/// any profile generation younger than 30 days.
const GC_ARGS: &[&str] = &["--delete-older-than", "30d"];

/// Legacy ~/.local/bin binaries that the Nix profile now provides (S2
/// global list): each surviving copy shadows the flake via PATH order.
const LEGACY_BINS: &[&str] = &[
    "hx", "lazygit", "delta", "yazi", "ya", "eza", "zoxide", "starship", "atuin", "bat",
    "marksman", "herdr",
];

/// `prim doctor [--prune]`: print every diagnostic section; with `prune`,
/// run (or under PRIM_DRY_RUN=1, print) nix-collect-garbage. Returns false
/// only when the prune step itself failed.
pub fn run(prune: bool) -> bool {
    let Some(home) = dirs::home_dir() else {
        eprintln!("[prim] no se pudo determinar $HOME");
        return false;
    };
    let mode = if prune { " + prune" } else { "" };
    println!("[prim] doctor — diagnostico de solo lectura{mode}:");
    nix_report();
    receipt_report();
    flake_report();
    store_report();
    direnv_report(&home);
    leftovers_report(&home);
    files_report(receipts_dir().as_deref());
    if prune {
        return prune_run();
    }
    println!("[prim] doctor listo (solo lectura). 'prim doctor --prune' hace GC (>30d).");
    true
}

/// Parse the args after `prim doctor`: none = read-only diagnostics,
/// `--prune` adds the GC step, anything else is a usage error.
pub(crate) fn parse_args(args: &[String]) -> Option<bool> {
    match args {
        [] => Some(false),
        [a] if a == "--prune" => Some(true),
        _ => None,
    }
}

// --- sections ----------------------------------------------------------------

/// Files component section (S11h): shown only when the files receipt
/// exists — best-effort probes of the SMB-over-Tailscale share (smbd,
/// nmbd, tailscaled, tailnet IP, share path, user/group match). Every
/// probe degrades to a tolerant note; the section is never fatal.
fn files_report(receipts: Option<&Path>) {
    let Some(text) = receipts.and_then(files_receipt_text) else {
        return;
    };
    let share = json_field(&text, "share_path").unwrap_or_else(|| "-".into());
    let user = json_field(&text, "share_user").unwrap_or_else(|| "-".into());
    println!("[prim]   files: share={share} usuario={user} (SMB solo en el tailnet).");
    println!("[prim]   files: smbd={}", svc_state("smbd"));
    println!(
        "[prim]   files: nmbd={} (enmascarado es lo esperado, S10)",
        svc_state("nmbd")
    );
    println!("[prim]   files: tailscaled={}", svc_state("tailscaled"));
    println!("[prim]   files: ip de tailscale={}", ts_ip());
    if Path::new(&share).is_dir() {
        println!("[prim]   files: {share} existe.");
    } else {
        println!("[prim]   files: {share} no existe — revisa la ruta del receipt.");
    }
    if let Some(line) = files_user_group_advisory(&user) {
        println!("[prim]   files (aviso): {line}");
    }
}

/// Contents of ~/.local/share/prim/files.receipt.json; None = the files
/// component is not installed here (the section is gated on this).
fn files_receipt_text(receipts: &Path) -> Option<String> {
    fs::read_to_string(receipts.join("files.receipt.json")).ok()
}

/// `systemctl is-active <unit>` state as printed ("active", "masked",
/// ...); "-" when systemctl is missing or silent.
fn svc_state(unit: &str) -> String {
    match Command::new("systemctl").args(["is-active", unit]).output() {
        Ok(o) if !o.stdout.is_empty() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "-".into(),
    }
}

/// Tailscale IPv4, best-effort: plain CLI first, then `sudo -n` (the
/// daemon socket is root-only on default installs); never prompts.
fn ts_ip() -> String {
    let attempts: [&[&str]; 2] = [
        &["tailscale", "ip", "-4"],
        &["sudo", "-n", "tailscale", "ip", "-4"],
    ];
    for args in attempts {
        let mut cmd = Command::new(args[0]);
        cmd.args(&args[1..]);
        if let Ok(o) = cmd.output() {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if o.status.success() && !s.is_empty() {
                return s;
            }
        }
    }
    "-".into()
}

/// One advisory line when the receipt's share user or the `primfiles`
/// group no longer exists on the system (removed by hand): the share
/// cannot authenticate until `prim install files` re-runs.
fn files_user_group_advisory(user: &str) -> Option<String> {
    let user_ok = quiet_ok(Command::new("id").arg(user));
    let group_ok = quiet_ok(Command::new("getent").args(["group", "primfiles"]));
    if user_ok && group_ok {
        None
    } else {
        Some(format!(
            "usuario {user} o grupo primfiles ya no existen — 'prim install files' lo repara."
        ))
    }
}

/// True when the command exits 0 with all output discarded.
fn quiet_ok(cmd: &mut Command) -> bool {
    cmd.stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// nix section: version plus experimental features (nix-command and flakes
/// are required by the home-manager flow).
fn nix_report() {
    nix::ensure_nix_path();
    match Command::new("nix").arg("--version").output() {
        Ok(o) if o.status.success() => println!(
            "[prim]   nix: {}",
            String::from_utf8_lossy(&o.stdout).trim()
        ),
        _ => println!("[prim]   nix: no encontrado — corre 'prim bootstrap' o 'prim install rig'."),
    }
    match Command::new("nix")
        .args(["config", "show", "experimental-features"])
        .output()
    {
        Ok(o) if o.status.success() => println!(
            "[prim]   experimental features: {} (se necesitan nix-command y flakes)",
            String::from_utf8_lossy(&o.stdout).trim()
        ),
        _ => println!(
            "[prim]   experimental features: no confirmadas (se necesitan nix-command y flakes)."
        ),
    }
}

/// Receipt section: the rig receipt prim itself writes (profile, date,
/// dotfiles manager). Absent = prim never managed rig on this machine.
fn receipt_report() {
    let text = receipts_dir().and_then(|d| fs::read_to_string(d.join("rig.receipt.json")).ok());
    match text {
        Some(t) => {
            let profile = json_field(&t, "profile").unwrap_or_else(|| "-".into());
            let at = json_field(&t, "installed_at_iso").unwrap_or_else(|| "-".into());
            let dots = json_field(&t, "dotfiles").unwrap_or_else(|| "-".into());
            println!("[prim]   rig: perfil={profile} instalado={at} dotfiles={dots}");
        }
        None => {
            println!("[prim]   rig: sin receipt — prim aun no gestiono el rig en esta maquina.")
        }
    }
}

/// Flake section: can the global rig flake be reached and evaluated?
/// Tolerant by design: on timeout or failure it notes and moves on.
fn flake_report() {
    nix::ensure_nix_path();
    let mut cmd = Command::new("nix");
    cmd.args(["flake", "show", RIG_FLAKE_REF, "--json"]);
    if status_with_timeout(&mut cmd, FLAKE_TIMEOUT_SECS) {
        println!("[prim]   flake global: accesible ({RIG_FLAKE_REF})");
    } else {
        println!(
            "[prim]   flake global: no accesible ahora ({RIG_FLAKE_REF}) — nota: red lenta o timeout de {FLAKE_TIMEOUT_SECS}s; reintenta mas tarde."
        );
    }
}

/// Store section: on-disk size of the (single-user) store via du.
fn store_report() {
    match Command::new("du").args(["-sh", "/nix/store"]).output() {
        Ok(o) if o.status.success() => println!(
            "[prim]   /nix/store: {}",
            String::from_utf8_lossy(&o.stdout).trim()
        ),
        _ => println!("[prim]   /nix/store: no medible (¿falta /nix?)."),
    }
}

/// direnv section: GC roots of active per-repo environments (nix-direnv
/// keeps them in ~/.local/share/direnv, S7).
fn direnv_report(home: &Path) {
    match direnv_roots(home) {
        Some(n) => println!("[prim]   direnv: {n} GC roots activos (~/.local/share/direnv)"),
        None => {
            println!("[prim]   direnv: sin ~/.local/share/direnv (sin entornos por repo activos).")
        }
    }
}

/// Leftovers section: pre-Nix era remains, one advisory line each. Never
/// removed by prim (P6 decides, with consent).
fn leftovers_report(home: &Path) {
    let chezmoi = find_in_path("chezmoi", &std::env::var_os("PATH").unwrap_or_default());
    let lines = leftover_lines(home, chezmoi);
    if lines.is_empty() {
        println!("[prim]   legado: sin restos de la era pre-Nix.");
    } else {
        for line in lines {
            println!("[prim]   legado (era pre-Nix): {line}");
        }
        println!("[prim]   los restos solo se informan: prim nunca los borra (P6 decide).");
    }
}

/// Prune step: print the exact GC command, then run it (unless
/// PRIM_DRY_RUN=1, which prints the plan instead — same guard as nix.rs).
fn prune_run() -> bool {
    nix::ensure_nix_path();
    let gc = format!("nix-collect-garbage {}", GC_ARGS.join(" "));
    println!("[prim] prune: {gc} — borra generaciones >30d y el store no referenciado.");
    if dry_run() {
        println!("[prim] PRIM_DRY_RUN=1: no se ejecuta nada; comando exacto:");
        println!("[prim]   {gc}");
        return true;
    }
    Command::new("nix-collect-garbage")
        .args(GC_ARGS)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// --- detection seams (unit-tested) -------------------------------------------

/// True when PRIM_DRY_RUN=1 (local copy of the nix.rs guard; src/nix.rs is
/// outside this change's edit surface).
fn dry_run() -> bool {
    std::env::var_os("PRIM_DRY_RUN").is_some_and(|v| v == "1")
}

/// Advisory lines for pre-Nix leftovers found under `home`, plus the
/// chezmoi binary when found on PATH. Pure detection: nothing is removed.
fn leftover_lines(home: &Path, chezmoi_bin: Option<PathBuf>) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(bin) = chezmoi_bin {
        lines.push(format!(
            "binario chezmoi en PATH ({}) — home-manager gestiona los dotfiles",
            bin.display()
        ));
    }
    if home.join(".local/share/chezmoi").exists() {
        lines.push(format!(
            "{} (estado de chezmoi)",
            home.join(".local/share/chezmoi").display()
        ));
    }
    let bins: Vec<&str> = LEGACY_BINS
        .iter()
        .copied()
        .filter(|n| is_executable_file(&home.join(".local/bin").join(n)))
        .collect();
    if !bins.is_empty() {
        lines.push(format!(
            "binarios en {}/.local/bin ({}) — el perfil nix los provee",
            home.display(),
            bins.join(",")
        ));
    }
    if home.join("google-cloud-sdk").exists() {
        lines.push(format!(
            "{} (gcloud viejo; ahora es por repo, S2)",
            home.join("google-cloud-sdk").display()
        ));
    }
    lines
}

/// Number of entries in ~/.local/share/direnv (nix-direnv GC roots);
/// None when the directory is absent.
fn direnv_roots(home: &Path) -> Option<usize> {
    fs::read_dir(home.join(".local/share/direnv"))
        .ok()
        .map(|rd| rd.filter_map(|e| e.ok()).count())
}

/// Locate `name` as an executable file on a PATH-style string (a pure
/// `command -v` replacement so the chezmoi check is testable).
fn find_in_path(name: &str, path: &std::ffi::OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
        .filter(|d| !d.as_os_str().is_empty())
        .map(|d| d.join(name))
        .find(|p| is_executable_file(p))
}

/// True when `path` is a regular file with any execute bit set.
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Run `cmd` with discarded output, killing it after `secs` seconds.
/// Timeout, spawn failure and non-zero exit all report false: callers print
/// their own tolerant note.
fn status_with_timeout(cmd: &mut Command, secs: u64) -> bool {
    let Ok(mut child) = cmd.stdout(Stdio::null()).stderr(Stdio::null()).spawn() else {
        return false;
    };
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(200)),
            Err(_) => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// --prune flag parsing: no args = read-only, --prune = prune, anything
    /// else (unknown flags, extra args, abbreviations) is a usage error.
    #[test]
    fn parse_args_matrix() {
        struct Row {
            name: &'static str,
            args: Vec<&'static str>,
            expect: Option<bool>,
        }
        let rows = vec![
            Row {
                name: "no args: read-only",
                args: vec![],
                expect: Some(false),
            },
            Row {
                name: "--prune: adds gc",
                args: vec!["--prune"],
                expect: Some(true),
            },
            Row {
                name: "extra arg after --prune rejected",
                args: vec!["--prune", "x"],
                expect: None,
            },
            Row {
                name: "abbreviation rejected",
                args: vec!["-p"],
                expect: None,
            },
            Row {
                name: "unknown flag rejected",
                args: vec!["--gc"],
                expect: None,
            },
            Row {
                name: "bare word rejected",
                args: vec!["prune"],
                expect: None,
            },
        ];
        for row in rows {
            let args: Vec<String> = row.args.iter().map(|s| s.to_string()).collect();
            assert_eq!(parse_args(&args), row.expect, "{}", row.name);
        }
    }

    /// Files section gating (S11h): the doctor section runs only when
    /// the files receipt exists; without it there is no section at all.
    #[test]
    fn files_section_gated_on_receipt() {
        let dir = std::env::temp_dir().join("prim-doctor-files-gate-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(files_receipt_text(&dir), None, "sin receipt no hay seccion");
        fs::write(
            dir.join("files.receipt.json"),
            "{\n\"component\": \"files\",\n\"share_user\": \"filesmb\"\n}\n",
        )
        .unwrap();
        let text = files_receipt_text(&dir).unwrap();
        assert_eq!(json_field(&text, "component").as_deref(), Some("files"));
        assert_eq!(json_field(&text, "share_user").as_deref(), Some("filesmb"));
        fs::remove_dir_all(&dir).unwrap();
    }

    /// Leftovers detection over a planted fake HOME: chezmoi state dir,
    /// legacy ~/.local/bin binaries and google-cloud-sdk each produce one
    /// advisory line naming the path; an empty home produces none; and the
    /// planted files are still there afterwards (detection never removes).
    #[test]
    fn leftovers_detect_planted_fake_home() {
        let home = std::env::temp_dir().join("prim-doctor-leftovers-test");
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(home.join(".local/bin")).unwrap();
        fs::create_dir_all(home.join(".local/share/chezmoi")).unwrap();
        fs::create_dir_all(home.join("google-cloud-sdk")).unwrap();
        for bin in ["hx", "starship"] {
            let p = home.join(".local/bin").join(bin);
            fs::write(&p, b"#!/bin/sh\n").unwrap();
            fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let lines = leftover_lines(&home, None);
        assert_eq!(lines.len(), 3, "chezmoi state + legacy bins + gcloud sdk");
        assert!(
            lines.iter().any(|l| l.contains(".local/share/chezmoi")),
            "chezmoi state line: {lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains(".local/bin") && l.contains("hx") && l.contains("starship")),
            "legacy bins line: {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("google-cloud-sdk")),
            "gcloud sdk line: {lines:?}"
        );
        // Nothing was removed: detection is read-only.
        assert!(home.join(".local/bin/hx").exists());
        assert!(home.join(".local/share/chezmoi").exists());
        assert!(home.join("google-cloud-sdk").exists());
        // A clean home has no advisories; the chezmoi binary alone is one.
        let clean = std::env::temp_dir().join("prim-doctor-clean-test");
        let _ = fs::remove_dir_all(&clean);
        fs::create_dir_all(&clean).unwrap();
        assert!(leftover_lines(&clean, None).is_empty());
        assert_eq!(
            leftover_lines(&clean, Some(PathBuf::from("/usr/bin/chezmoi"))).len(),
            1
        );
        fs::remove_dir_all(&home).unwrap();
        fs::remove_dir_all(&clean).unwrap();
    }

    /// find_in_path matches an executable file by scanning a PATH-style
    /// string; non-executable files and missing names never match.
    #[test]
    fn find_in_path_executables_only() {
        let dir = std::env::temp_dir().join("prim-doctor-path-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("chezmoi");
        fs::write(&exe, b"#!/bin/sh\n").unwrap();
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(dir.join("plain"), b"data\n").unwrap();
        let path = std::env::join_paths([&dir, Path::new("/usr/bin")]).unwrap();
        assert_eq!(find_in_path("chezmoi", &path), Some(exe.clone()));
        assert_eq!(find_in_path("plain", &path), None, "not executable");
        assert_eq!(find_in_path("missing", &path), None);
        assert_eq!(find_in_path("chezmoi", std::ffi::OsStr::new("")), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    /// direnv GC roots are counted from ~/.local/share/direnv entries; an
    /// absent directory is None (reported as "no direnv yet").
    #[test]
    fn direnv_roots_counts_entries() {
        let home = std::env::temp_dir().join("prim-doctor-direnv-test");
        let _ = fs::remove_dir_all(&home);
        let d = home.join(".local/share/direnv");
        fs::create_dir_all(&d).unwrap();
        for name in ["root-a", "root-b"] {
            fs::write(d.join(name), b"gc-root\n").unwrap();
        }
        assert_eq!(direnv_roots(&home), Some(2));
        fs::remove_dir_all(&home).unwrap();
        assert_eq!(direnv_roots(&home), None, "dir gone -> None");
    }
}

//! prim nix — rig activation over Nix (P3, S5/S6a/S8).
//!
//! rig no longer goes through rig/install.sh: prim speaks Nix directly.
//! `activate` runs home-manager (from its flake, matching the repo's lock)
//! against the global rig flake github:bit-crew/prim?dir=rig/nix — a
//! floating ref to the monorepo's main, re-evaluated on every run, so it
//! tracks latest main; version pinning lives in the repo's flake.lock per
//! S4. `deactivate` runs `home-manager uninstall`. Every invocation first
//! prepends ~/.nix-profile/bin to PATH: after a fresh bootstrap the nix
//! binaries are not on this process's PATH yet. PRIM_DRY_RUN=1 guards
//! both: instead of running nix, the exact command line (with the identity
//! env) is printed and success is assumed, so the flow can be exercised in
//! sandboxes without touching a real HOME.

use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bootstrap::{identity_path, PLACEHOLDER_EMAIL, PLACEHOLDER_NAME};
use crate::dotfiles;
use crate::lifecycle::{json_field, receipts_dir};

/// home-manager runner flake (same ref the rig flake.lock pins).
const HOME_MANAGER_REF: &str = "github:nix-community/home-manager/master";

/// The global rig flake: the rig/nix subdirectory of the monorepo's main
/// branch (no separate branch exists; `?dir=` is nix's subdirectory syntax
/// and the ref floats to latest main, re-evaluated on every activation).
const RIG_FLAKE_REF: &str = "github:bit-crew/prim?dir=rig/nix";

/// Backup suffix for pre-existing conflicting dotfiles: home-manager moves
/// them aside instead of refusing (chezmoi-era files on migrated machines).
const BACKUP_SUFFIX: &str = "prim-backup";

/// Default profile when nothing names one (vps = minimal resting VPS).
const DEFAULT_PROFILE: &str = "vps";

/// How many home-manager generations stay in the profile: the current one
/// plus a couple of previous (for `home-manager rollback`). Every
/// activation creates new links, and every old link is a GC root that
/// keeps its whole closure alive forever: without pruning, the store grows
/// with each update and nothing is ever reclaimed.
const KEEP_GENERATIONS: usize = 3;

/// How many package-profile generations stay. The dotfiles generation is
/// the one worth rolling back (the user's own files live there), so it
/// keeps a margin. The package profile does not: an old user-environment
/// holds the whole closure of the tools that were installed back then,
/// which is how a package that already left the rig kept ~666 MB alive in
/// the store. One generation is enough (it is the current one).
const KEEP_PACKAGE_GENERATIONS: usize = 1;

/// The link series that home-manager's switch creates in the profiles dir:
/// `home-manager<N>-link` (the dotfiles generation) and `profile<N>-link`
/// (the package profile, the user-environment). BOTH are GC roots, and the
/// old package profiles are what keep a removed tool's closure alive. Other
/// entries in the dir (channels, anything else) are not ours to touch.
const GENERATION_SERIES: [(&str, usize); 2] = [
    ("home-manager", KEEP_GENERATIONS),
    ("profile", KEEP_PACKAGE_GENERATIONS),
];

// --- PATH / env helpers ------------------------------------------------------

/// Prepend ~/.nix-profile/bin to this process's PATH (idempotent): after a
/// fresh bootstrap no profile.d ran here yet. Reused before every nix call.
pub fn ensure_nix_path() {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let bin = home.join(".nix-profile/bin");
    let current = std::env::var_os("PATH").unwrap_or_default();
    if std::env::split_paths(&current).any(|p| p == bin) {
        return; // already on PATH
    }
    let new_path = if current.is_empty() {
        bin.to_string_lossy().into_owned()
    } else {
        format!("{}:{}", bin.display(), current.to_string_lossy())
    };
    std::env::set_var("PATH", &new_path);
}

/// Identity env for the nix calls: PRIM_GIT_NAME/PRIM_GIT_EMAIL from
/// ~/.config/prim/identity.toml, with the bootstrap placeholders when the
/// file is missing or incomplete — the flake reads both via guarded
/// builtins.getEnv at switch time.
pub fn identity_env() -> Vec<(&'static str, String)> {
    let text = dirs::home_dir()
        .and_then(|h| fs::read_to_string(identity_path(&h)).ok())
        .unwrap_or_default();
    vec![
        (
            "PRIM_GIT_NAME",
            toml_field(&text, "git_name", PLACEHOLDER_NAME),
        ),
        (
            "PRIM_GIT_EMAIL",
            toml_field(&text, "git_email", PLACEHOLDER_EMAIL),
        ),
    ]
}

/// First non-empty `key = "value"` in a small TOML text; `fallback` when
/// the key is absent. identity.toml is prim-written, so scanning is enough.
fn toml_field(text: &str, key: &str, fallback: &str) -> String {
    text.lines()
        .filter_map(|l| l.split_once('='))
        .find(|(k, _)| k.trim() == key)
        .and_then(|(_, v)| {
            let v = v.trim().trim_matches('"');
            (!v.is_empty()).then(|| v.to_string())
        })
        .unwrap_or_else(|| fallback.to_string())
}

/// True when PRIM_DRY_RUN=1: print the exact plan instead of running nix.
pub(crate) fn dry_run() -> bool {
    std::env::var_os("PRIM_DRY_RUN").is_some_and(|v| v == "1")
}

// --- activation / deactivation ----------------------------------------------

/// A pending nix invocation: extra env plus args, either shown (dry run) or
/// spawned with inherited stdio.
struct Plan {
    env: Vec<(&'static str, String)>,
    args: Vec<String>,
}

impl Plan {
    /// Shell-style line: env assignments then `nix <args>` — what would run.
    fn line(&self) -> String {
        let mut parts: Vec<String> = self.env.iter().map(|(k, v)| format!("{k}='{v}'")).collect();
        parts.push("nix".into());
        parts.extend(self.args.iter().cloned());
        parts.join(" ")
    }

    /// Run with inherited stdio: the user sees nix evaluation, build output
    /// and home-manager's own messages live. True on exit success.
    fn run(&self) -> bool {
        let mut cmd = Command::new("nix");
        cmd.args(&self.args);
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        cmd.status().map(|s| s.success()).unwrap_or(false)
    }
}

/// Activation plan for a profile: home-manager switch from the pushed
/// flake. `-b` moves conflicting dotfiles aside; `--impure` reads identity.
/// HOME_MANAGER_BACKUP_OVERWRITE=1 lets a stale `<file>.prim-backup` be
/// replaced: prim keeps its own snapshots (see dotfiles), so home-manager's
/// backup slot is free to be reused instead of failing the activation.
///
/// `--refresh` is NOT optional: nix caches the resolution of the rig's
/// github ref (tarball-ttl, 1h) and every use refreshes that cache, so
/// without it the activation keeps deploying a STALE revision of the repo
/// (the rig's flake.lock does NOT pin the repo itself). The rig would then
/// install whatever main was an hour ago, and a pushed change would appear
/// to install "successfully" while changing nothing.
fn switch_plan(profile: &str) -> Plan {
    let mut env = identity_env();
    env.push(("HOME_MANAGER_BACKUP_OVERWRITE", "1".to_string()));
    Plan {
        env,
        args: vec![
            "run".into(),
            HOME_MANAGER_REF.into(),
            "--".into(),
            "switch".into(),
            "--flake".into(),
            format!("{RIG_FLAKE_REF}#{profile}"),
            "--refresh".into(),
            "-b".into(),
            BACKUP_SUFFIX.into(),
            "--impure".into(),
        ],
    }
}

/// Uninstall plan: `home-manager uninstall`.
fn uninstall_plan() -> Plan {
    Plan {
        env: identity_env(),
        args: vec![
            "run".into(),
            HOME_MANAGER_REF.into(),
            "--".into(),
            "uninstall".into(),
        ],
    }
}

/// Activate the rig profile over Nix (install + update share this: the
/// github ref re-evaluates to latest main on every run).
///
/// The dotfiles policy is "merge, never clobber": the managed paths are
/// snapshotted before the switch, and after it the user's own versions are
/// put back, the rig's versions archived as `<file>.prim-rig`, and the
/// differences reported (see dotfiles).
pub fn activate(profile: &str) -> bool {
    ensure_nix_path();
    println!("[prim] activando rig (perfil {profile}) con home-manager...");
    let home = dirs::home_dir();
    let env = identity_env();
    let manifest = dotfiles::resolve_manifest(profile, &env);
    let snapshots = match (&home, &manifest) {
        (Some(h), Some(m)) => match dotfiles::make_run_dir(h, &stamp()) {
            Ok(dir) => {
                println!(
                    "[prim] dotfiles: {m} rutas gestionadas; copia de seguridad en {}",
                    dir.display()
                );
                dotfiles::snapshot(h, m, &dir)
            }
            Err(_) => Vec::new(),
        },
        _ => Vec::new(),
    };
    let ok = run_or_show(&switch_plan(profile));
    if ok {
        if let (Some(h), Some(m)) = (&home, &manifest) {
            let entries = dotfiles::merge(h, m, &snapshots);
            dotfiles::print_report(&entries);
        }
        prune_generations();
    }
    ok
}

/// The profile directory where home-manager keeps its generation links:
/// `$NIX_STATE_DIR/profiles`, or `$HOME/.local/state/nix/profiles` (the
/// default state dir).
fn profiles_dir() -> Option<std::path::PathBuf> {
    let state = std::env::var_os("NIX_STATE_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/state/nix")))?;
    Some(state.join("profiles"))
}

/// The generation number of a profile link name, when it belongs to that
/// series: `home-manager-7-link` is 7 of the `home-manager` series.
/// Anything else (the series symlink itself, the store paths, another
/// series) is not a generation link.
fn generation_number(name: &str, series: &str) -> Option<u64> {
    name.strip_prefix(series)?
        .strip_prefix('-')?
        .strip_suffix("-link")?
        .parse::<u64>()
        .ok()
}

/// The generation links of one series with their number, oldest first.
fn generation_links(dir: &Path, series: &str) -> Vec<(u64, String)> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut links: Vec<(u64, String)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let number = generation_number(&name, series)?;
            Some((number, name))
        })
        .collect();
    links.sort_by_key(|(number, _)| *number);
    links
}

/// The links to remove: every one but the `keep` most recent. Pure, so the
/// policy is testable without touching the filesystem.
fn links_to_prune(links: &[(u64, String)], keep: usize) -> Vec<String> {
    let keep_from = links.len().saturating_sub(keep);
    links[..keep_from]
        .iter()
        .map(|(_, name)| name.clone())
        .collect()
}

/// Drop the old generation links after a successful activation. The links
/// are symlinks into the store: removing them frees the generations to be
/// collected (the store keeps them until a GC pass). The current link and
/// the previous ones are kept, so `home-manager rollback` still works.
fn prune_generations() {
    let Some(dir) = profiles_dir() else {
        return;
    };
    let mut removed = 0;
    let mut kept = 0;
    for (series, series_keep) in GENERATION_SERIES {
        let links = generation_links(&dir, series);
        let doomed = links_to_prune(&links, series_keep);
        kept += links.len().saturating_sub(doomed.len());
        if doomed.is_empty() {
            continue;
        }
        if dry_run() {
            println!(
                "[prim] PRIM_DRY_RUN=1: no se ejecuta nada; se removerian {} links viejos de {series}: {}",
                doomed.len(),
                doomed.join(", ")
            );
            continue;
        }
        for name in doomed {
            let path = dir.join(&name);
            // A symlink into the store: removing it never touches the store.
            match fs::remove_file(&path) {
                Ok(()) => removed += 1,
                Err(e) => eprintln!("[prim] no se pudo remover {name}: {e}"),
            }
        }
    }
    if removed > 0 {
        println!(
            "[prim] generaciones: {removed} links viejos removidos (quedan {kept} para rollback)."
        );
    }
}

/// Deactivate the rig: `home-manager uninstall`. The /nix store, the agent
/// stack and ~/work are untouched — only home-manager's links and dotfiles.
/// The user's own dotfiles are snapshotted first and restored if the
/// uninstall takes them down with the managed links.
pub fn deactivate() -> bool {
    ensure_nix_path();
    println!("[prim] nota: /nix (store), el stack del agente y ~/work no se tocan.");
    println!("[prim] desactivando rig (home-manager uninstall)...");
    let home = dirs::home_dir();
    let profile = receipt_profile().unwrap_or_else(|| DEFAULT_PROFILE.to_string());
    let manifest = dotfiles::resolve_manifest(&profile, &identity_env());
    let snapshots = match (&home, &manifest) {
        (Some(h), Some(m)) => match dotfiles::make_run_dir(h, &stamp()) {
            Ok(dir) => dotfiles::snapshot(h, m, &dir),
            Err(_) => Vec::new(),
        },
        _ => Vec::new(),
    };
    let ok = run_or_show(&uninstall_plan());
    if ok {
        if let Some(h) = &home {
            let restored = dotfiles::restore_missing(h, &snapshots);
            if restored > 0 {
                println!("[prim] dotfiles: {restored} archivo(s) tuyos restaurados tras la desinstalacion.");
            }
        }
    }
    ok
}

/// Timestamp for the snapshot run directory (unix seconds).
fn stamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

/// Execute a plan, or print it verbatim under PRIM_DRY_RUN=1.
fn run_or_show(plan: &Plan) -> bool {
    if dry_run() {
        println!("[prim] PRIM_DRY_RUN=1: no se ejecuta nada; plan exacto:");
        println!("[prim]   {}", plan.line());
        return true;
    }
    plan.run()
}

// --- receipts ----------------------------------------------------------------

/// Write ~/.local/share/prim/rig.receipt.json (atomic: tmp + rename in
/// the same dir). Shrunk to what prim knows: Nix owns packages/dotfiles.
pub fn write_receipt(profile: &str) -> io::Result<()> {
    let Some(dir) = receipts_dir() else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no se pudo determinar $HOME",
        ));
    };
    let unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_secs();
    write_receipt_in(&dir, profile, unix)?;
    println!("[prim] receipt: {}", dir.join("rig.receipt.json").display());
    Ok(())
}

/// Receipt writer against an explicit dir (test seam); atomic tmp + rename.
fn write_receipt_in(dir: &Path, profile: &str, unix: u64) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let tmp = dir.join(".rig.receipt.json.tmp");
    fs::write(&tmp, receipt_body(profile, unix))?;
    fs::rename(&tmp, dir.join("rig.receipt.json"))
}

/// Line-oriented receipt body: one field per line, so prim's scanner
/// (lifecycle::json_field) reads it without a JSON parser.
fn receipt_body(profile: &str, unix: u64) -> String {
    let fields = [
        "\"component\": \"rig\"".to_string(),
        "\"schema\": 1".to_string(),
        format!("\"installed_at_unix\": {unix}"),
        format!("\"installed_at_iso\": \"{}\"", epoch_to_iso(unix)),
        format!("\"profile\": \"{profile}\""),
        "\"pkgmgr\": \"nix\"".to_string(),
        "\"apt\": []".to_string(),
        "\"bins\": []".to_string(),
        "\"dirs\": []".to_string(),
        "\"toolchains\": []".to_string(),
        "\"dotfiles\": \"home-manager\"".to_string(),
        "\"login_shell_changed\": false".to_string(),
        "\"work_dir\": true".to_string(),
    ];
    format!("{{\n{}\n}}\n", fields.join(",\n"))
}

// --- profile resolution ------------------------------------------------------

/// Profile to use for rig: receipt (installed state) wins, then the CLI
/// `--profile <p>`, then PRIM_PROFILE, then a TTY prompt (default vps;
/// non-TTY takes the default silently).
pub fn resolve_profile(cli: Option<&str>) -> String {
    let tty = io::stdin().is_terminal();
    let mut ask = || -> Option<String> {
        if !tty {
            return None;
        }
        print!("[prim] perfil del rig [vps/dc1] ({DEFAULT_PROFILE}): ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .ok()
            .map(|_| line.trim().to_string())
    };
    let receipt = receipt_profile();
    let env = env_profile();
    resolve_profile_impl(receipt.as_deref(), cli, env.as_deref(), &mut ask)
}

/// Pure resolution core (test seam): first non-empty of receipt/cli/env,
/// then the ask closure's non-empty answer, then the default profile.
fn resolve_profile_impl(
    receipt: Option<&str>,
    cli: Option<&str>,
    env: Option<&str>,
    ask: &mut dyn FnMut() -> Option<String>,
) -> String {
    for candidate in [receipt, cli, env] {
        if let Some(p) = candidate.filter(|p| !p.is_empty()) {
            return (*p).to_string();
        }
    }
    ask()
        .filter(|a| !a.is_empty())
        .unwrap_or_else(|| DEFAULT_PROFILE.to_string())
}

/// profile recorded in the rig receipt (installed state), if any.
fn receipt_profile() -> Option<String> {
    let text = receipts_dir().and_then(|d| fs::read_to_string(d.join("rig.receipt.json")).ok())?;
    json_field(&text, "profile")
}

/// PRIM_PROFILE env value, when set to something non-empty.
fn env_profile() -> Option<String> {
    std::env::var_os("PRIM_PROFILE")
        .map(|v| v.to_string_lossy().into_owned())
        .filter(|p| !p.is_empty())
}

// --- epoch -> ISO (no date crate) --------------------------------------------

/// Epoch seconds -> "YYYY-MM-DDTHH:MM:SSZ" (UTC): civil-from-days for the
/// date part plus a plain time-of-day split.
fn epoch_to_iso(unix: u64) -> String {
    let (y, m, d) = civil_from_days((unix / 86_400) as i64);
    let t = unix % 86_400;
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        t / 3600,
        (t % 3600) / 60,
        t % 60
    )
}

/// Days since 1970-01-01 -> (year, month, day) in the proleptic Gregorian
/// calendar (Howard Hinnant's civil_from_days).
fn civil_from_days(z: i64) -> (i64, u64, u64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

// --- tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Profile resolution matrix: receipt > CLI --profile > PRIM_PROFILE >
    /// prompt answer > "vps". `ask` models the TTY: Some = answer typed,
    /// None = no TTY (the public wrapper returns None without prompting).
    #[test]
    fn resolve_profile_matrix() {
        struct Row {
            name: &'static str,
            receipt: Option<&'static str>,
            cli: Option<&'static str>,
            env: Option<&'static str>,
            answer: Option<&'static str>,
            expect: &'static str,
            expect_asked: bool,
        }
        let rows = vec![
            Row {
                name: "receipt wins over cli and env",
                receipt: Some("dc1"),
                cli: Some("vps"),
                env: Some("vps"),
                answer: None,
                expect: "dc1",
                expect_asked: false,
            },
            Row {
                name: "cli beats env",
                receipt: None,
                cli: Some("dc1"),
                env: Some("vps"),
                answer: None,
                expect: "dc1",
                expect_asked: false,
            },
            Row {
                name: "env beats the prompt",
                receipt: None,
                cli: None,
                env: Some("dc1"),
                answer: Some("vps"),
                expect: "dc1",
                expect_asked: false,
            },
            Row {
                name: "prompt answer used on tty",
                receipt: None,
                cli: None,
                env: None,
                answer: Some("dc1"),
                expect: "dc1",
                expect_asked: true,
            },
            Row {
                name: "empty prompt answer falls back to vps",
                receipt: None,
                cli: None,
                env: None,
                answer: Some(""),
                expect: DEFAULT_PROFILE,
                expect_asked: true,
            },
            Row {
                name: "no tty: no answer means default vps",
                receipt: None,
                cli: None,
                env: None,
                answer: None,
                expect: DEFAULT_PROFILE,
                expect_asked: true,
            },
            Row {
                name: "empty cli value falls through to env",
                receipt: None,
                cli: Some(""),
                env: Some("dc1"),
                answer: None,
                expect: "dc1",
                expect_asked: false,
            },
        ];
        for row in rows {
            let mut asked = false;
            let mut ask = || {
                asked = true;
                row.answer.map(str::to_string)
            };
            let got = resolve_profile_impl(row.receipt, row.cli, row.env, &mut ask);
            assert_eq!(got, row.expect, "{}", row.name);
            assert_eq!(asked, row.expect_asked, "{}", row.name);
        }
    }

    /// Epoch -> ISO on known instants: epoch start, leap day, the day after
    /// a leap day, and a year boundary.
    #[test]
    fn epoch_to_iso_known_dates() {
        let cases = [
            (0u64, "1970-01-01T00:00:00Z"),
            (951_782_400u64, "2000-02-29T00:00:00Z"),
            (951_868_800u64, "2000-03-01T00:00:00Z"),
            (1_735_689_599u64, "2024-12-31T23:59:59Z"),
            (1_735_689_600u64, "2025-01-01T00:00:00Z"),
        ];
        for (unix, iso) in cases {
            assert_eq!(epoch_to_iso(unix), iso, "unix={unix}");
        }
    }

    /// Receipt roundtrip: what write_receipt_in writes is read back by
    /// lifecycle's json_field scanner (the exact reader status() uses), and
    /// the atomic tmp file is renamed away.
    #[test]
    fn receipt_roundtrip_scans_with_json_field() {
        let dir = std::env::temp_dir().join("prim-nix-receipt-test");
        let _ = fs::remove_dir_all(&dir);
        write_receipt_in(&dir, "dc1", 1_735_689_600).unwrap();
        let text = fs::read_to_string(dir.join("rig.receipt.json")).unwrap();
        assert_eq!(json_field(&text, "component").as_deref(), Some("rig"));
        assert_eq!(json_field(&text, "profile").as_deref(), Some("dc1"));
        assert_eq!(
            json_field(&text, "installed_at_iso").as_deref(),
            Some("2025-01-01T00:00:00Z")
        );
        assert_eq!(json_field(&text, "pkgmgr").as_deref(), Some("nix"));
        assert_eq!(
            json_field(&text, "dotfiles").as_deref(),
            Some("home-manager")
        );
        assert!(
            !dir.join(".rig.receipt.json.tmp").exists(),
            "tmp renamed away"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    /// The activation plan is the exact command line: hm runner, switch,
    /// flake#profile, --refresh (always the latest main), backup suffix and
    /// --impure (identity env present).
    #[test]
    fn switch_plan_line_is_exact() {
        let plan = switch_plan("dc1");
        assert_eq!(
            plan.args.join(" "),
            "run github:nix-community/home-manager/master -- switch --flake github:bit-crew/prim?dir=rig/nix#dc1 --refresh -b prim-backup --impure"
        );
        assert!(plan.env.iter().any(|(k, _)| *k == "PRIM_GIT_NAME"));
        assert!(plan.env.iter().any(|(k, _)| *k == "PRIM_GIT_EMAIL"));
    }

    /// The uninstall plan runs `home-manager uninstall` with identity env.
    #[test]
    fn uninstall_plan_is_exact() {
        let plan = uninstall_plan();
        assert_eq!(
            plan.args.join(" "),
            "run github:nix-community/home-manager/master -- uninstall"
        );
        assert!(plan.env.iter().any(|(k, _)| *k == "PRIM_GIT_NAME"));
    }

    /// Identity TOML scan: quoted values with or without spaces, empty and
    /// missing keys fall back to the bootstrap placeholders.
    #[test]
    fn toml_field_matrix() {
        let text = "# comment\ngit_name = \"Ada Lovelace\"\ngit_email=\"ada@x\"\nempty=\"\"\n";
        let cases = [
            ("git_name", "Ada Lovelace"),
            ("git_email", "ada@x"),
            ("empty", "fallback"),
            ("missing", "fallback"),
        ];
        for (key, expect) in cases {
            assert_eq!(toml_field(text, key, "fallback"), expect, "{key}");
        }
    }

    /// Which names are generation links of a series: the numbered
    /// `<series><N>-link` entries, and nothing else (the series symlink
    /// itself, the store paths and the other series are ignored).
    #[test]
    fn generation_number_matrix() {
        let cases = [
            ("home-manager-1-link", "home-manager", Some(1)),
            ("home-manager-11-link", "home-manager", Some(11)),
            ("home-manager-100-link", "home-manager", Some(100)),
            ("home-manager", "home-manager", None),
            ("home-manager-1", "home-manager", None),
            ("home-manager-x-link", "home-manager", None),
            ("home-manager-1-link.bak", "home-manager", None),
            ("other-1-link", "home-manager", None),
            ("profile-1-link", "profile", Some(1)),
            ("profile-7-link", "profile", Some(7)),
            ("profile", "profile", None),
            ("channels-1-link", "profile", None),
            ("home-manager-1-link", "profile", None),
        ];
        for (name, series, expect) in cases {
            assert_eq!(generation_number(name, series), expect, "{name}");
        }
    }

    /// Pruning policy: every generation but the `keep` most recent is
    /// removed. The table is the single source of truth.
    #[test]
    fn links_to_prune_keeps_the_newest() {
        let scenarios: Vec<(&str, Vec<(u64, String)>, usize, Vec<String>)> = vec![
            (
                "nothing to prune when there are fewer links than the keep",
                vec![
                    (1, "home-manager-1-link".to_string()),
                    (2, "home-manager-2-link".to_string()),
                ],
                3,
                vec![],
            ),
            (
                "the oldest ones go, the newest stay",
                vec![
                    (1, "home-manager-1-link".to_string()),
                    (2, "home-manager-2-link".to_string()),
                    (3, "home-manager-3-link".to_string()),
                    (4, "home-manager-4-link".to_string()),
                ],
                3,
                vec!["home-manager-1-link".to_string()],
            ),
            ("an empty profile prunes nothing", vec![], 3, vec![]),
            (
                "keep zero removes every generation",
                vec![
                    (1, "home-manager-1-link".to_string()),
                    (2, "home-manager-2-link".to_string()),
                ],
                0,
                vec![
                    "home-manager-1-link".to_string(),
                    "home-manager-2-link".to_string(),
                ],
            ),
            (
                "keep one leaves only the current generation",
                vec![
                    (5, "profile-5-link".to_string()),
                    (6, "profile-6-link".to_string()),
                    (7, "profile-7-link".to_string()),
                ],
                1,
                vec!["profile-5-link".to_string(), "profile-6-link".to_string()],
            ),
        ];
        for (name, links, keep, expect) in scenarios {
            assert_eq!(links_to_prune(&links, keep), expect, "{name}");
        }
    }

    /// The keep policy per series: the dotfiles generation keeps a rollback
    /// margin, the package profile keeps only the current one (an old
    /// user-environment holds the closure of the tools of its time).
    #[test]
    fn generation_series_keep_counts() {
        assert_eq!(GENERATION_SERIES.len(), 2);
        assert_eq!(GENERATION_SERIES[0], ("home-manager", KEEP_GENERATIONS));
        assert_eq!(GENERATION_SERIES[1], ("profile", KEEP_PACKAGE_GENERATIONS));
        assert!(KEEP_GENERATIONS >= 2, "rollback margin for the dotfiles");
        assert_eq!(KEEP_PACKAGE_GENERATIONS, 1, "solo la generacion actual");
    }
}

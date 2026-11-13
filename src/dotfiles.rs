//! dotfiles — non-destructive activation policy for the rig's home files.
//!
//! home-manager owns a fixed set of paths in $HOME (the flake's `home-files`
//! tree). Its `switch` replaces every one of them with a symlink into the
//! nix store: a dotfile the user already had is moved aside as
//! `<file>.prim-backup` and the store version takes its place. On a machine
//! that was configured before the rig existed (chezmoi-era files, hand-made
//! systemd units), that silently transfers ownership of the user's own
//! configuration to the repository.
//!
//! The policy here is "merge, never clobber":
//!
//!   1. before the switch, every managed path that already exists is
//!      snapshotted (content + mode, or the symlink target);
//!   2. home-manager runs and replaces what it wants;
//!   3. after the switch, the user's version is put back where it was, the
//!      rig's version is archived next to it as `<file>.prim-rig`, and the
//!      difference is reported;
//!   4. paths the rig creates for the first time become real, editable
//!      files (not store symlinks), so the user can keep changing them.
//!
//! The manifest of managed paths is not hardcoded: it is read from the
//! home-manager generation itself (`<generation>/home-files`), so it can
//! never drift from the flake.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Suffix for the archived rig version of a file the user keeps.
pub const RIG_SUFFIX: &str = ".prim-rig";

/// The global rig flake: the rig/nix subdirectory of the monorepo's main
/// branch (`?dir=` is nix's subdirectory syntax).
const RIG_FLAKE_REF: &str = "github:bit-crew/prim?dir=rig/nix";

/// Managed paths of a profile, relative to $HOME, "/"-separated.
#[derive(Default, Debug, Clone)]
pub struct Manifest {
    pub paths: Vec<String>,
}

impl std::fmt::Display for Manifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.paths.len())
    }
}

/// What happened to one managed path during the merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// The user's file was there before and stays: identical content, or
    /// home-manager skipped it.
    Kept,
    /// The user's file was replaced by the rig's version and was restored;
    /// `diff` holds the unified difference (yours -> rig).
    Restored { diff: String },
    /// The rig created the path; it is now a real editable file.
    Created,
}

/// One managed path and what the merge did with it.
#[derive(Debug, Clone)]
pub struct Entry {
    pub path: String,
    pub action: Action,
}

/// A snapshot of one managed path taken before the switch.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub path: String,
    /// File content (empty when the path was a symlink).
    pub content: Vec<u8>,
    /// Permission bits of the original file.
    pub mode: u32,
    /// Link target when the original was a symlink.
    pub link: Option<String>,
}

/// Resolve the managed-path manifest for a profile by building the rig
/// flake's home-manager generation and reading its `home-files` tree.
/// None when nix is unavailable or the build fails: the merge then works
/// from home-manager's own backups alone.
pub fn resolve_manifest(profile: &str, env: &[(&str, String)]) -> Option<Manifest> {
    let attr = format!("{RIG_FLAKE_REF}#homeConfigurations.{profile}.activationPackage");
    let mut cmd = Command::new("nix");
    cmd.args(["build", "--no-link", "--print-out-paths", "--impure", &attr]);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let generation = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if generation.is_empty() {
        return None;
    }
    let files = fs::read_link(Path::new(&generation).join("home-files")).ok()?;
    let mut paths = Vec::new();
    collect_files(&files, Path::new(""), &mut paths);
    paths.sort();
    Some(Manifest { paths })
}

/// Relative paths of every file under `dir`, "/"-separated.
fn collect_files(dir: &Path, prefix: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let rel = prefix.join(&name);
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            collect_files(&path, &rel, out);
        } else {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// Snapshot every managed path that already exists under `home`. The copies
/// live in `run_dir` so a crashed switch still leaves the user's files
/// recoverable.
pub fn snapshot(home: &Path, manifest: &Manifest, run_dir: &Path) -> Vec<Snapshot> {
    let mut out = Vec::new();
    for rel in &manifest.paths {
        let target = home.join(rel);
        let Ok(kind) = fs::symlink_metadata(&target) else {
            continue;
        };
        let snap = if kind.is_symlink() {
            match fs::read_link(&target) {
                Ok(link) => Snapshot {
                    path: rel.clone(),
                    content: Vec::new(),
                    mode: 0o644,
                    link: Some(link.to_string_lossy().into_owned()),
                },
                Err(_) => continue,
            }
        } else if kind.is_file() {
            let Ok(content) = fs::read(&target) else {
                continue;
            };
            let mode = file_mode(&target);
            Snapshot {
                path: rel.clone(),
                content,
                mode,
                link: None,
            }
        } else {
            continue;
        };
        store_snapshot(&snap, run_dir);
        out.push(snap);
    }
    out
}

/// Persist one snapshot under the run directory (file copy, or a symlink
/// recreated from its recorded target).
fn store_snapshot(snap: &Snapshot, run_dir: &Path) {
    let dest = run_dir.join(&snap.path);
    if let Some(parent) = dest.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match &snap.link {
        Some(target) => {
            let _ = fs::remove_file(&dest);
            let _ = std::os::unix::fs::symlink(target, &dest);
        }
        None => {
            let _ = fs::write(&dest, &snap.content);
            let _ = fs::set_permissions(&dest, permissions(snap.mode));
        }
    }
}

/// Permission bits of an existing file (0o644 when unreadable).
fn file_mode(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o7777)
        .unwrap_or(0o644)
}

/// Permissions from mode bits.
fn permissions(mode: u32) -> fs::Permissions {
    use std::os::unix::fs::PermissionsExt;
    fs::Permissions::from_mode(mode)
}

/// Merge: put the user's files back after home-manager's switch, archive
/// the rig's version, materialize freshly created paths as real files.
pub fn merge(home: &Path, manifest: &Manifest, snapshots: &[Snapshot]) -> Vec<Entry> {
    let mut entries = Vec::new();
    for rel in &manifest.paths {
        let target = home.join(rel);
        let Some(snap) = snapshots.iter().find(|s| &s.path == rel) else {
            // The path did not exist before the switch: the rig created it.
            // A store symlink becomes a real editable file; a file home-
            // manager wrote as text already is one.
            if is_store_symlink(&target) {
                if materialize(&target) {
                    entries.push(Entry {
                        path: rel.clone(),
                        action: Action::Created,
                    });
                }
            } else if target.exists() {
                entries.push(Entry {
                    path: rel.clone(),
                    action: Action::Created,
                });
            }
            continue;
        };
        let rig = fs::read(&target).ok();
        let same = match (&rig, &snap.link) {
            (Some(r), None) => *r == snap.content,
            (None, Some(_)) => false,
            _ => false,
        };
        if same {
            entries.push(Entry {
                path: rel.clone(),
                action: Action::Kept,
            });
            continue;
        }
        // The rig's version wins the path: archive it, restore the user's.
        if let Some(rig) = &rig {
            let archive = with_suffix(&target, RIG_SUFFIX);
            if let Some(parent) = archive.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&archive, rig);
        }
        restore(&target, snap);
        let diff = match (&rig, &snap.link) {
            (Some(rig), None) => unified_diff(
                &String::from_utf8_lossy(&snap.content),
                &String::from_utf8_lossy(rig),
            ),
            _ => String::new(),
        };
        entries.push(Entry {
            path: rel.clone(),
            action: Action::Restored { diff },
        });
    }
    entries
}

/// Put the user's version back at `target`.
fn restore(target: &Path, snap: &Snapshot) {
    match &snap.link {
        Some(link) => {
            let _ = fs::remove_file(target);
            let _ = std::os::unix::fs::symlink(link, target);
        }
        None => {
            if let Some(parent) = target.parent() {
                let _ = fs::create_dir_all(parent);
            }
            // The path may now be a symlink into the nix store: writing
            // through it would fail (read-only) or touch the store, so the
            // link goes first and the user's file takes its place.
            if fs::symlink_metadata(target).is_ok_and(|m| m.is_symlink()) {
                let _ = fs::remove_file(target);
            }
            let _ = fs::write(target, &snap.content);
            let _ = fs::set_permissions(target, permissions(snap.mode));
        }
    }
}

/// Restore paths that disappeared (used after `home-manager uninstall`).
pub fn restore_missing(home: &Path, snapshots: &[Snapshot]) -> usize {
    let mut restored = 0;
    for snap in snapshots {
        let target = home.join(&snap.path);
        if target.exists() {
            continue;
        }
        restore(&target, snap);
        restored += 1;
    }
    restored
}

/// True when the path is a symlink into a home-manager files tree.
fn is_store_symlink(target: &Path) -> bool {
    let Ok(link) = fs::read_link(target) else {
        return false;
    };
    link.to_string_lossy().contains("-home-manager-files/")
}

/// Replace a store symlink with a real copy of its content (editable).
fn materialize(target: &Path) -> bool {
    let Ok(link) = fs::read_link(target) else {
        return false;
    };
    let Ok(content) = fs::read(&link) else {
        return false;
    };
    let mode = file_mode(&link);
    let _ = fs::remove_file(target);
    let _ = fs::write(target, &content);
    let _ = fs::set_permissions(target, permissions(mode));
    true
}

/// Path with a suffix appended to the file name (`x` -> `x.prim-rig`).
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

/// Unified diff between two texts (yours -> rig). Empty when identical.
pub fn unified_diff(a: &str, b: &str) -> String {
    let al: Vec<&str> = a.lines().collect();
    let bl: Vec<&str> = b.lines().collect();
    if al == bl {
        return String::new();
    }
    let ops = diff_ops(&al, &bl);
    // Line numbers of every op in the "a" (yours) and "b" (rig) files.
    let mut numbered: Vec<(Op<'_>, usize, usize)> = Vec::with_capacity(ops.len());
    let (mut ai, mut bi) = (1usize, 1usize);
    for op in ops {
        let consumes_a = !matches!(op, Op::Insert(_));
        let consumes_b = !matches!(op, Op::Remove(_));
        numbered.push((op, ai, bi));
        if consumes_a {
            ai += 1;
        }
        if consumes_b {
            bi += 1;
        }
    }
    // Hunks: runs of changes with three context lines around them.
    let mut hunks: Vec<(usize, usize)> = Vec::new();
    let mut idx = 0;
    while idx < numbered.len() {
        if !is_change(&numbered[idx].0) {
            idx += 1;
            continue;
        }
        let start = idx.saturating_sub(3);
        let mut end = idx;
        while end < numbered.len() && (is_change(&numbered[end].0) || end < idx + 4) {
            end += 1;
        }
        let end = end.min(numbered.len());
        match hunks.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => hunks.push((start, end)),
        }
        idx = end.max(idx + 1);
    }
    let mut out = String::new();
    for (start, end) in hunks {
        let mut a_start = 0;
        let mut b_start = 0;
        let mut a_count = 0;
        let mut b_count = 0;
        for (i, (op, _, _)) in numbered[start..end].iter().enumerate() {
            if i == 0 {
                a_start = numbered[start].1;
                b_start = numbered[start].2;
            }
            if !matches!(op, Op::Insert(_)) {
                a_count += 1;
            }
            if !matches!(op, Op::Remove(_)) {
                b_count += 1;
            }
        }
        out.push_str(&format!(
            "@@ -{a_start},{a_count} +{b_start},{b_count} @@\n"
        ));
        for (op, _, _) in &numbered[start..end] {
            match op {
                Op::Keep(l) => out.push_str(&format!(" {l}\n")),
                Op::Change(l, r) => {
                    out.push_str(&format!("-{l}\n"));
                    out.push_str(&format!("+{r}\n"));
                }
                Op::Remove(l) => out.push_str(&format!("-{l}\n")),
                Op::Insert(r) => out.push_str(&format!("+{r}\n")),
            }
        }
    }
    out
}

/// True when the op changes the file (not a context line).
fn is_change(op: &Op<'_>) -> bool {
    !matches!(op, Op::Keep(_))
}

/// One diff operation over a pair of line sequences.
enum Op<'a> {
    Keep(&'a str),
    Change(&'a str, &'a str),
    Remove(&'a str),
    Insert(&'a str),
}

/// Longest-common-subsequence diff of two line sequences.
fn diff_ops<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<Op<'a>> {
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let mut ops = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a[i] == b[j] {
            ops.push(Op::Keep(a[i]));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            ops.push(Op::Change(a[i], b[j]));
            i += 1;
            j += 1;
        } else {
            ops.push(Op::Remove(a[i]));
            i += 1;
        }
    }
    while i < n {
        ops.push(Op::Remove(a[i]));
        i += 1;
    }
    while j < m {
        ops.push(Op::Insert(b[j]));
        j += 1;
    }
    ops
}

/// Print the merge report for the user.
pub fn print_report(entries: &[Entry]) {
    let restored: Vec<&Entry> = entries
        .iter()
        .filter(|e| matches!(e.action, Action::Restored { .. }))
        .collect();
    let created: Vec<&Entry> = entries
        .iter()
        .filter(|e| matches!(e.action, Action::Created))
        .collect();
    if restored.is_empty() && created.is_empty() {
        println!("[prim] dotfiles: sin conflictos (todo igual o ya creado).");
        return;
    }
    if !restored.is_empty() {
        println!(
            "[prim] dotfiles: {} archivo(s) tuyos restaurados (tu version gana; la del rig queda en <archivo>{}):",
            restored.len(),
            RIG_SUFFIX
        );
        for entry in &restored {
            println!("[prim]   ~/{}{}", entry.path, RIG_SUFFIX);
            if let Action::Restored { diff } = &entry.action {
                for line in diff.lines().take(24) {
                    println!("[prim]     {line}");
                }
                let total = diff.lines().count();
                if total > 24 {
                    println!("[prim]     ... ({total} lineas en total)");
                }
            }
        }
    }
    if !created.is_empty() {
        println!(
            "[prim] dotfiles: {} archivo(s) creados por el rig (editables):",
            created.len()
        );
        for entry in &created {
            println!("[prim]   ~/{}", entry.path);
        }
    }
}

/// The run directory for snapshots: ~/.local/state/prim/dotfiles/<stamp>.
pub fn run_dir(home: &Path, stamp: &str) -> PathBuf {
    home.join(".local/state/prim/dotfiles").join(stamp)
}

/// Create the run directory, returning it.
pub fn make_run_dir(home: &Path, stamp: &str) -> io::Result<PathBuf> {
    let dir = run_dir(home, stamp);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("prim-dotfiles-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A file the user already had is restored after the rig replaces it,
    /// and the rig's version is archived next to it.
    #[test]
    fn merge_restores_the_user_file() {
        let home = tmp("restore");
        let run = home.join("run");
        fs::create_dir_all(&run).unwrap();
        fs::write(home.join(".zshrc"), "mi zshrc\n").unwrap();
        let manifest = Manifest {
            paths: vec![".zshrc".into()],
        };
        let snaps = snapshot(&home, &manifest, &run);
        // home-manager "switches": the store version takes the path.
        fs::write(home.join(".zshrc"), "zshrc del rig\n").unwrap();
        let entries = merge(&home, &manifest, &snaps);
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0].action, Action::Restored { .. }));
        assert_eq!(
            fs::read_to_string(home.join(".zshrc")).unwrap(),
            "mi zshrc\n"
        );
        assert_eq!(
            fs::read_to_string(home.join(format!(".zshrc{RIG_SUFFIX}"))).unwrap(),
            "zshrc del rig\n"
        );
        fs::remove_dir_all(&home).unwrap();
    }

    /// An identical file is left alone: no restore, no archive.
    #[test]
    fn merge_keeps_identical_files() {
        let home = tmp("identical");
        let run = home.join("run");
        fs::create_dir_all(&run).unwrap();
        fs::write(home.join(".zshrc"), "igual\n").unwrap();
        let manifest = Manifest {
            paths: vec![".zshrc".into()],
        };
        let snaps = snapshot(&home, &manifest, &run);
        let entries = merge(&home, &manifest, &snaps);
        assert_eq!(entries[0].action, Action::Kept);
        assert!(!home.join(format!(".zshrc{RIG_SUFFIX}")).exists());
        fs::remove_dir_all(&home).unwrap();
    }

    /// Restoring a regular file over a store symlink must not write THROUGH
    /// the link (that would touch the read-only nix store): the link goes
    /// away and the user's file takes its place.
    #[test]
    fn merge_restores_over_a_store_symlink_without_writing_through_it() {
        let home = tmp("overwrite-link");
        let run = home.join("run");
        fs::create_dir_all(&run).unwrap();
        let store = tmp("home-manager-files-store");
        fs::write(store.join("zshrc"), "zshrc del rig\n").unwrap();
        let target = home.join(".zshrc");
        fs::write(&target, "mi zshrc\n").unwrap();
        let manifest = Manifest {
            paths: vec![".zshrc".into()],
        };
        let snaps = snapshot(&home, &manifest, &run);
        // home-manager replaces the path with a symlink into the store.
        fs::remove_file(&target).unwrap();
        std::os::unix::fs::symlink(store.join("zshrc"), &target).unwrap();
        merge(&home, &manifest, &snaps);
        // The user's file is back, and the store file was not written to.
        assert!(!fs::symlink_metadata(&target).unwrap().is_symlink());
        assert_eq!(fs::read_to_string(&target).unwrap(), "mi zshrc\n");
        assert_eq!(
            fs::read_to_string(store.join("zshrc")).unwrap(),
            "zshrc del rig\n"
        );
        fs::remove_dir_all(&home).unwrap();
        fs::remove_dir_all(&store).unwrap();
    }

    /// A path the rig creates for the first time becomes a real file, not
    /// a symlink into the store, so the user can edit it.
    #[test]
    fn merge_materializes_new_store_links() {
        let home = tmp("materialize");
        let run = home.join("run");
        fs::create_dir_all(&run).unwrap();
        let store = tmp("home-manager-files");
        fs::create_dir_all(store.join(".config")).unwrap();
        fs::write(store.join(".config/env"), "export X=1\n").unwrap();
        let manifest = Manifest {
            paths: vec![".config/env".into()],
        };
        // The path does not exist yet: the snapshot has nothing to keep.
        let snaps = snapshot(&home, &manifest, &run);
        assert!(snaps.is_empty());
        // home-manager creates it as a symlink into the store.
        let target = home.join(".config/env");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(store.join(".config/env"), &target).unwrap();
        let entries = merge(&home, &manifest, &snaps);
        assert_eq!(entries[0].action, Action::Created);
        assert!(!fs::symlink_metadata(&target).unwrap().is_symlink());
        assert_eq!(fs::read_to_string(&target).unwrap(), "export X=1\n");
        fs::remove_dir_all(&home).unwrap();
        fs::remove_dir_all(&store).unwrap();
    }

    /// A symlink the user had is restored as a symlink, not as a copy.
    #[test]
    fn merge_restores_user_symlinks() {
        let home = tmp("symlink");
        let run = home.join("run");
        fs::create_dir_all(&run).unwrap();
        let elsewhere = tmp("elsewhere");
        fs::write(elsewhere.join("target"), "contenido\n").unwrap();
        std::os::unix::fs::symlink(elsewhere.join("target"), home.join(".link")).unwrap();
        let manifest = Manifest {
            paths: vec![".link".into()],
        };
        let snaps = snapshot(&home, &manifest, &run);
        fs::remove_file(home.join(".link")).unwrap();
        fs::write(home.join(".link"), "version del rig\n").unwrap();
        merge(&home, &manifest, &snaps);
        assert!(fs::symlink_metadata(home.join(".link"))
            .unwrap()
            .is_symlink());
        assert_eq!(
            fs::read_to_string(home.join(".link")).unwrap(),
            "contenido\n"
        );
        fs::remove_dir_all(&home).unwrap();
        fs::remove_dir_all(&elsewhere).unwrap();
    }

    /// The diff shows what the rig wanted instead of the user's file.
    #[test]
    fn unified_diff_shows_both_sides() {
        let diff = unified_diff("uno\ndos\n", "uno\ntres\n");
        assert!(diff.contains("-dos"));
        assert!(diff.contains("+tres"));
        assert!(diff.contains(" uno"));
        assert_eq!(unified_diff("igual\n", "igual\n"), "");
    }

    /// The manifest is read from a generation's home-files tree.
    #[test]
    fn manifest_reads_the_files_tree() {
        let gen = tmp("generation");
        let files = tmp("files");
        fs::create_dir_all(files.join(".config")).unwrap();
        fs::write(files.join(".zshrc"), "x").unwrap();
        fs::write(files.join(".config/app.conf"), "y").unwrap();
        std::os::unix::fs::symlink(&files, gen.join("home-files")).unwrap();
        let mut paths = Vec::new();
        collect_files(&files, Path::new(""), &mut paths);
        paths.sort();
        assert_eq!(paths, vec![".config/app.conf", ".zshrc"]);
        fs::remove_dir_all(&gen).unwrap();
        fs::remove_dir_all(&files).unwrap();
    }
}

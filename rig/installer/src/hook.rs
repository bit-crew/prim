//! Lightweight ODD enforcement: a pre-commit hook that verifies the project
//! before allowing a commit. It reads the repo's objective state (which build
//! system is present, whether its tests pass) and decides deterministically —
//! the model does not vote. This forces ODD step 5 (verify) and nothing else,
//! because that is the only step a hook can honestly enforce.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const HOOK_MARKER: &str = "# devlab-install pre-commit hook";

/// Install the pre-commit hook into the given repo (default: cwd). Writes a
/// shell shim that calls `devlab-install verify`. Idempotent and non-destructive:
/// refuses to clobber an existing unmanaged hook.
pub fn install(repo: &Path, self_exe: &Path) -> io::Result<()> {
    let git_dir = repo.join(".git");
    if !git_dir.exists() {
        return Err(io::Error::other(format!(
            "{} no es un repo git (.git no existe)",
            repo.display()
        )));
    }
    let hooks_dir = git_dir.join("hooks");
    std::fs::create_dir_all(&hooks_dir)?;
    let hook_path = hooks_dir.join("pre-commit");

    if hook_path.exists() {
        let existing = std::fs::read_to_string(&hook_path).unwrap_or_default();
        if !existing.contains(HOOK_MARKER) {
            return Err(io::Error::other(format!(
                "ya existe un pre-commit no gestionado en {}. No se sobrescribe.",
                hook_path.display()
            )));
        }
    }

    let exe = self_exe.display().to_string().replace('\\', "/");
    let script = format!(
        "#!/bin/sh\n{HOOK_MARKER}\n# Verifies the project before each commit (ODD step 5).\nexec \"{exe}\" verify\n"
    );
    std::fs::write(&hook_path, script)?;
    make_executable(&hook_path)?;
    Ok(())
}

/// Run verification: detect the build system and run its test command. Returns
/// Ok(true) when checks pass (or there is nothing to verify), Ok(false) when
/// they fail. Deterministic: based on files on disk and exit codes.
pub fn verify(repo: &Path) -> io::Result<bool> {
    let Some((label, program, args)) = detect_verify(repo) else {
        println!("[verify] No se detecto sistema de build conocido; nada que verificar.");
        return Ok(true);
    };
    println!("[verify] {label}: {program} {}", args.join(" "));
    io::stdout().flush().ok();

    match Command::new(program).args(&args).current_dir(repo).status() {
        Ok(s) if s.success() => {
            println!("[verify] OK.");
            Ok(true)
        }
        Ok(_) => {
            eprintln!("[verify] FALLO: la verificacion no paso. Commit bloqueado.");
            Ok(false)
        }
        Err(e) => {
            // Tool missing: do not block the commit on a tooling gap; warn only.
            eprintln!("[verify] No se pudo ejecutar '{program}': {e}. Se omite la verificacion.");
            Ok(true)
        }
    }
}

/// Detect the build/test command from marker files. First match wins.
fn detect_verify(repo: &Path) -> Option<(&'static str, &'static str, Vec<String>)> {
    if repo.join("Cargo.toml").is_file() {
        return Some(("Rust", "cargo", vec!["test".into()]));
    }
    if repo.join("pyproject.toml").is_file() || repo.join("pytest.ini").is_file() {
        return Some(("Python", "pytest", vec!["-q".into()]));
    }
    if repo.join("go.mod").is_file() {
        return Some(("Go", "go", vec!["test".into(), "./...".into()]));
    }
    if repo.join("package.json").is_file() {
        return Some(("Node", "npm", vec!["test".into(), "--silent".into()]));
    }
    None
}

/// The repo root is the current working directory for a git hook.
pub fn cwd() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(unix)]
fn make_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> io::Result<()> {
    Ok(()) // Git on Windows runs .git/hooks/pre-commit via sh regardless of bit.
}

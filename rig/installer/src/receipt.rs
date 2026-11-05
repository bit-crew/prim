//! Installation receipt — the recorded truth of what rig-install put on this
//! machine (S3, task T3 of odd/tasks/prim-lifecycle.md).
//!
//! Written once, atomically, at the end of a successful install to
//! ~/.local/share/prim/rig.receipt.json. The future `update`/`uninstall`
//! verbs read it instead of prompting: which profile was chosen, which
//! packages the system manager installed, and where each downloaded binary
//! lives. No serde dependency: we control both ends and the shape is fixed,
//! so a tiny purpose-built serializer/parser is enough.

use std::io;
use std::path::{Path, PathBuf};

/// One downloaded binary recorded in the receipt.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Bin {
    pub(crate) name: String,
    pub(crate) path: String,
    /// Installed version; None until the update unit (T4) records it.
    pub(crate) version: Option<String>,
    /// Upstream repo (owner/name) for version checks; None for tools whose
    /// installer script hides the release source.
    pub(crate) repo: Option<String>,
}

/// The receipt. schema=1 is the current shape; future fields bump it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Receipt {
    pub(crate) component: String, // always "rig"
    pub(crate) schema: u32,       // 1
    pub(crate) installed_at_unix: String,
    pub(crate) installed_at_iso: String,
    pub(crate) profile: String,
    pub(crate) pkgmgr: String,
    pub(crate) apt: Vec<String>,
    pub(crate) bins: Vec<Bin>,
    pub(crate) dirs: Vec<String>,
    pub(crate) toolchains: Vec<String>,
    pub(crate) dotfiles: String, // "chezmoi"
    pub(crate) login_shell_changed: bool,
    pub(crate) work_dir: bool,
}

/// ~/.local/share/prim/rig.receipt.json
pub(crate) fn receipt_path() -> io::Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| io::Error::other("sin HOME"))?;
    Ok(home
        .join(".local")
        .join("share")
        .join("prim")
        .join("rig.receipt.json"))
}

/// Write the receipt atomically (tmp file + rename in the same dir).
pub(crate) fn save(receipt: &Receipt) -> io::Result<()> {
    save_to(&receipt_path()?, receipt)
}

/// Read the receipt back. Errors if missing or malformed.
pub(crate) fn load() -> io::Result<Receipt> {
    load_from(&receipt_path()?)
}

/// UTC ISO-8601 timestamp (Z suffix) for unix seconds, without a date crate.
pub(crate) fn iso_utc(unix_secs: u64) -> String {
    let days = (unix_secs / 86_400) as i64;
    let tod = unix_secs % 86_400;
    let (h, m, s) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// Days since 1970-01-01 to (year, month, day) — Howard Hinnant's
/// civil_from_days, valid across leap years and centuries.
fn civil_from_days(z: i64) -> (i64, u64, u64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

// --- read/write -------------------------------------------------------------

fn save_to(path: &Path, receipt: &Receipt) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "receipt.json".into());
    let tmp = path.with_file_name(format!("{name}.tmp"));
    std::fs::write(&tmp, to_json(receipt))?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn load_from(path: &Path) -> io::Result<Receipt> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| io::Error::other(format!("no se pudo leer {}: {e}", path.display())))?;
    parse(&text)
}

// --- serializer (hand-rolled; layout is line-oriented by design) -----------

/// Escape a string for JSON: quotes, backslashes and control characters.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn to_json(r: &Receipt) -> String {
    let str_field = |key: &str, val: &str| format!("  \"{key}\": \"{}\"", esc(val));
    let str_array = |key: &str, vals: &[String]| {
        if vals.is_empty() {
            return format!("  \"{key}\": []");
        }
        let items: Vec<String> = vals.iter().map(|v| format!("    \"{}\"", esc(v))).collect();
        format!("  \"{key}\": [\n{}\n  ]", items.join(",\n"))
    };
    let opt = |v: &Option<String>| {
        v.as_ref()
            .map(|s| format!("\"{}\"", esc(s)))
            .unwrap_or_else(|| "null".into())
    };
    let bins = if r.bins.is_empty() {
        "  \"bins\": []".to_string()
    } else {
        let items: Vec<String> = r
            .bins
            .iter()
            .map(|b| {
                let fields = [
                    format!("      \"name\": \"{}\"", esc(&b.name)),
                    format!("      \"path\": \"{}\"", esc(&b.path)),
                    format!("      \"version\": {}", opt(&b.version)),
                    format!("      \"repo\": {}", opt(&b.repo)),
                ]
                .join(",\n");
                format!("    {{\n{fields}\n    }}")
            })
            .collect();
        format!("  \"bins\": [\n{}\n  ]", items.join(",\n"))
    };
    format!(
        "{{\n{}\n}}\n",
        [
            str_field("component", &r.component),
            format!("  \"schema\": {}", r.schema),
            str_field("installed_at_unix", &r.installed_at_unix),
            str_field("installed_at_iso", &r.installed_at_iso),
            str_field("profile", &r.profile),
            str_field("pkgmgr", &r.pkgmgr),
            str_array("apt", &r.apt),
            bins,
            str_array("dirs", &r.dirs),
            str_array("toolchains", &r.toolchains),
            str_field("dotfiles", &r.dotfiles),
            format!("  \"login_shell_changed\": {}", r.login_shell_changed),
            format!("  \"work_dir\": {}", r.work_dir),
        ]
        .join(",\n")
    )
}

// --- parser ------------------------------------------------------------------
// Line-oriented: the writer above emits exactly one scalar, array element or
// brace per line, and esc() guarantees no raw newline inside any value, so
// every line is unambiguous. An explicit context tells string-array elements
// and bin-object fields apart from top-level fields.

fn parse(text: &str) -> io::Result<Receipt> {
    #[derive(PartialEq, Clone, Copy)]
    enum Ctx {
        Apt,
        Dirs,
        Toolchains,
        /// Inside the "bins" array, between objects.
        Bins,
        /// Inside one bin object.
        BinObj,
    }

    let mut r = Receipt {
        component: String::new(),
        schema: 0,
        installed_at_unix: String::new(),
        installed_at_iso: String::new(),
        profile: String::new(),
        pkgmgr: String::new(),
        apt: Vec::new(),
        bins: Vec::new(),
        dirs: Vec::new(),
        toolchains: Vec::new(),
        dotfiles: String::new(),
        login_shell_changed: false,
        work_dir: false,
    };
    let mut ctx: Option<Ctx> = None;
    let mut cur = Bin::default();

    for raw in text.lines() {
        let line = raw.trim();
        let line = line.strip_suffix(',').unwrap_or(line);
        if line == "[" {
            continue; // arrays open on their key line
        }
        if line == "]" {
            ctx = None;
            continue;
        }
        if line == "{" {
            if ctx == Some(Ctx::Bins) {
                ctx = Some(Ctx::BinObj);
                cur = Bin::default();
            }
            continue; // else: the document's own opening brace
        }
        if line == "}" {
            if ctx == Some(Ctx::BinObj) {
                r.bins.push(std::mem::take(&mut cur));
                ctx = Some(Ctx::Bins);
            }
            continue; // else: the document's own closing brace
        }
        if ctx == Some(Ctx::BinObj) {
            let (k, v) = split_field(line)?;
            match k.as_str() {
                "name" => cur.name = unquote(&v)?,
                "path" => cur.path = unquote(&v)?,
                "version" => cur.version = opt_value(&v)?,
                "repo" => cur.repo = opt_value(&v)?,
                other => return Err(bad(&format!("campo desconocido en bin: {other}"))),
            }
            continue;
        }
        match ctx {
            Some(Ctx::Apt) => {
                r.apt.push(unquote(line)?);
                continue;
            }
            Some(Ctx::Dirs) => {
                r.dirs.push(unquote(line)?);
                continue;
            }
            Some(Ctx::Toolchains) => {
                r.toolchains.push(unquote(line)?);
                continue;
            }
            _ => {}
        }
        let (key, val) = split_field(line)?;
        match key.as_str() {
            "component" => r.component = unquote(&val)?,
            "schema" => {
                let n: u32 = val.parse().map_err(|_| bad(&format!("schema: {val}")))?;
                if n != 1 {
                    return Err(bad(&format!("schema no soportado: {n} (esperado 1)")));
                }
                r.schema = n;
            }
            "installed_at_unix" => r.installed_at_unix = unquote(&val)?,
            "installed_at_iso" => r.installed_at_iso = unquote(&val)?,
            "profile" => r.profile = unquote(&val)?,
            "pkgmgr" => r.pkgmgr = unquote(&val)?,
            "apt" => ctx = (val == "[").then_some(Ctx::Apt),
            // bin objects arrive line-by-line; the key line only opens the array.
            "bins" => ctx = (val == "[").then_some(Ctx::Bins),
            "dirs" => ctx = (val == "[").then_some(Ctx::Dirs),
            "toolchains" => ctx = (val == "[").then_some(Ctx::Toolchains),
            "dotfiles" => r.dotfiles = unquote(&val)?,
            "login_shell_changed" => r.login_shell_changed = val == "true",
            "work_dir" => r.work_dir = val == "true",
            other => return Err(bad(&format!("campo desconocido: {other}"))),
        }
    }
    Ok(r)
}

/// Split `"key": value` into (key, raw value). Keys never contain escapes.
fn split_field(line: &str) -> io::Result<(String, String)> {
    let Some(rest) = line.strip_prefix('"') else {
        return Err(bad(line));
    };
    let Some(end) = rest.find("\": ") else {
        return Err(bad(line));
    };
    Ok((rest[..end].to_string(), rest[end + 3..].to_string()))
}

/// Value of an optional string field: `null` -> None, `"x"` -> Some(x).
fn opt_value(v: &str) -> io::Result<Option<String>> {
    if v == "null" {
        Ok(None)
    } else {
        unquote(v).map(Some)
    }
}

/// Strip the surrounding quotes and unescape a JSON string value.
fn unquote(v: &str) -> io::Result<String> {
    let Some(inner) = v.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
        return Err(bad(v));
    };
    Ok(unescape(inner))
}

/// Reverse of `esc`.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    out.push(ch);
                }
            }
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

fn bad(what: &str) -> io::Error {
    io::Error::other(format!("receipt invalido: {what}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fresh scratch dir OUTSIDE the repo (system temp); removed at test end.
    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rig-receipt-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A receipt exercising every field plus escaping: spaces, quotes,
    /// backslashes, commas and newlines must survive the roundtrip.
    fn sample() -> Receipt {
        Receipt {
            component: "rig".into(),
            schema: 1,
            installed_at_unix: "4107542400".into(),
            installed_at_iso: "2100-03-01T00:00:00Z".into(),
            profile: "dc1".into(),
            pkgmgr: "apt".into(),
            apt: vec![
                "git".into(),
                "paquete \"raro\" v2".into(),
                "back\\slash-pkg".into(),
            ],
            bins: vec![
                Bin {
                    name: "hx".into(),
                    path: "/tmp/my bin/hx".into(),
                    version: None,
                    repo: Some("helix-editor/helix".into()),
                },
                Bin {
                    name: "ya".into(),
                    path: "/tmp/we\"ird/ya".into(),
                    version: Some("25.01.1".into()),
                    repo: None,
                },
                Bin {
                    name: "marks".into(),
                    path: "/tmp/comma, dir/back\\slash".into(),
                    version: Some("1.2.3 \"beta\"".into()),
                    repo: Some("a/b".into()),
                },
                Bin {
                    name: "nl".into(),
                    path: "/tmp/new\nline".into(),
                    version: None,
                    repo: None,
                },
            ],
            dirs: vec!["~/.local/node".into()],
            toolchains: vec!["rustup".into(), "uv".into(), "go".into()],
            dotfiles: "chezmoi".into(),
            login_shell_changed: true,
            work_dir: false,
        }
    }

    #[test]
    fn roundtrip_preserves_all_fields() {
        let dir = scratch("roundtrip");
        let path = dir.join("rig.receipt.json");
        let r = sample();
        save_to(&path, &r).unwrap();
        assert_eq!(load_from(&path).unwrap(), r);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn roundtrip_empty_collections() {
        let dir = scratch("empty");
        let path = dir.join("rig.receipt.json");
        let r = Receipt {
            apt: vec![],
            bins: vec![],
            dirs: vec![],
            toolchains: vec![],
            login_shell_changed: false,
            work_dir: true,
            ..sample()
        };
        save_to(&path, &r).unwrap();
        assert_eq!(load_from(&path).unwrap(), r);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_receipt_fails_closed() {
        let dir = scratch("malformed");
        let path = dir.join("rig.receipt.json");
        std::fs::write(&path, "{\"component\": \"rig\",\n  \"schema\": 1 GARBAGE").unwrap();
        let err = load_from(&path).unwrap_err();
        assert!(err.to_string().contains("receipt invalido"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_schema_is_rejected() {
        let dir = scratch("schema");
        let path = dir.join("rig.receipt.json");
        let mut r = sample();
        r.schema = 2;
        save_to(&path, &r).unwrap();
        let err = load_from(&path).unwrap_err();
        assert!(err.to_string().contains("schema no soportado"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_is_atomic_and_creates_parent_dirs() {
        let dir = scratch("atomic");
        let path = dir.join("nested").join("prim").join("rig.receipt.json");
        save_to(&path, &sample()).unwrap();
        assert!(path.is_file());
        let leftovers: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "tmp files left: {leftovers:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_file_is_an_error() {
        let dir = scratch("missing");
        assert!(load_from(&dir.join("nope.json")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn iso_utc_matches_known_instants() {
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc(951_782_400), "2000-02-29T00:00:00Z"); // leap day
        assert_eq!(iso_utc(1_730_668_800), "2024-11-03T21:20:00Z");
        assert_eq!(iso_utc(4_107_542_400), "2100-03-01T00:00:00Z"); // 2100: no leap
    }
}

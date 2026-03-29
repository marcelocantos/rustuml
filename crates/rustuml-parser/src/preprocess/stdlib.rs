// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Stdlib resolution for PlantUML's standard library includes.
//!
//! When `!include <path>` is encountered, the path between angle brackets is
//! resolved relative to the stdlib root directory. The stdlib root is found by
//! checking (in order):
//!
//! 1. `PLANTUML_STDLIB` environment variable
//! 2. `$XDG_CACHE_HOME/rustuml/stdlib/` (or `~/.cache/rustuml/stdlib/`)
//! 3. `~/.rustuml/stdlib/`
//! 4. Well-known clone: `~/work/github.com/plantuml/plantuml-stdlib/stdlib/`
//!
//! If the path has no extension, `.puml` and `.iuml` are tried automatically.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

thread_local! {
    static STDLIB_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// Set the thread-local stdlib override. Call with `None` to clear.
#[cfg(test)]
pub(super) fn set_stdlib_override(root: Option<PathBuf>) {
    STDLIB_OVERRIDE.with(|cell| {
        cell.replace(root);
    });
}

/// Get the effective stdlib root: thread-local override first, then discovery.
pub(super) fn get_stdlib_root() -> Option<PathBuf> {
    STDLIB_OVERRIDE
        .with(|cell| cell.borrow().clone())
        .or_else(find_stdlib_root)
}

/// Resolve a stdlib include path (the part between angle brackets) to an
/// absolute file path. Appends `.puml` if the path has no extension.
pub(super) fn resolve_stdlib_path(stdlib_root: &Path, include_path: &str) -> Option<PathBuf> {
    let rel = PathBuf::from(include_path);

    // If no extension, try .puml first, then .iuml.
    if rel.extension().is_none() {
        let with_puml = stdlib_root.join(rel.with_extension("puml"));
        if with_puml.is_file() {
            return Some(with_puml);
        }
        let with_iuml = stdlib_root.join(rel.with_extension("iuml"));
        if with_iuml.is_file() {
            return Some(with_iuml);
        }
    }

    // Try as-is.
    let exact = stdlib_root.join(&rel);
    if exact.is_file() {
        return Some(exact);
    }

    None
}

/// Find the PlantUML stdlib root directory by searching well-known locations.
fn find_stdlib_root() -> Option<PathBuf> {
    // 1. Explicit env var.
    if let Ok(dir) = std::env::var("PLANTUML_STDLIB") {
        let p = PathBuf::from(&dir);
        if p.is_dir() {
            return Some(p);
        }
    }

    let home = home_dir()?;

    // 2. XDG cache.
    let xdg_cache = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join(".cache"));
    let xdg_path = xdg_cache.join("rustuml").join("stdlib");
    if xdg_path.is_dir() {
        return Some(xdg_path);
    }

    // 3. ~/.rustuml/stdlib/
    let rustuml_path = home.join(".rustuml").join("stdlib");
    if rustuml_path.is_dir() {
        return Some(rustuml_path);
    }

    // 4. Well-known clone location.
    let clone_path = home.join("work/github.com/plantuml/plantuml-stdlib/stdlib");
    if clone_path.is_dir() {
        return Some(clone_path);
    }

    None
}

/// Get the user's home directory.
fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_puml_extension() {
        let dir = std::env::temp_dir().join("rustuml_stdlib_test_ext");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("C4")).unwrap();
        std::fs::write(dir.join("C4/C4_Context.puml"), "").unwrap();

        let result = resolve_stdlib_path(&dir, "C4/C4_Context");
        assert_eq!(result, Some(dir.join("C4/C4_Context.puml")));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_explicit_extension() {
        let dir = std::env::temp_dir().join("rustuml_stdlib_test_explicit");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        std::fs::write(dir.join("lib/common.puml"), "").unwrap();

        let result = resolve_stdlib_path(&dir, "lib/common.puml");
        assert_eq!(result, Some(dir.join("lib/common.puml")));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_iuml_extension() {
        let dir = std::env::temp_dir().join("rustuml_stdlib_test_iuml");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        std::fs::write(dir.join("lib/common.iuml"), "").unwrap();

        let result = resolve_stdlib_path(&dir, "lib/common");
        assert_eq!(result, Some(dir.join("lib/common.iuml")));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_missing_returns_none() {
        let dir = std::env::temp_dir().join("rustuml_stdlib_test_missing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let result = resolve_stdlib_path(&dir, "nonexistent/lib");
        assert_eq!(result, None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stdlib_override_takes_precedence() {
        let dir = std::env::temp_dir().join("rustuml_stdlib_test_override");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        set_stdlib_override(Some(dir.clone()));
        let root = get_stdlib_root();
        assert_eq!(root, Some(dir.clone()));

        set_stdlib_override(None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

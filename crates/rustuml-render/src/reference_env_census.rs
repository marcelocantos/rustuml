// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! The committed Monkey Man census must agree with the stored per-face
//! extractions. This is the local half of the T14.6.8 drift gate. The live
//! half is `scripts/reference_env_census.py check`, which runs on the
//! reference host.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

fn census_font_rows(text: &str) -> BTreeMap<String, String> {
    let mut rows = BTreeMap::new();
    for line in text.lines() {
        let mut fields = line.split('\t');
        if fields.next() != Some("font") {
            continue;
        }
        let hash = fields.next().expect("font hash");
        let path = fields.next().expect("font path");
        assert!(fields.next().is_none(), "extra font field in {line}");
        assert_eq!(hash.len(), 64, "{path}");
        assert!(rows.insert(path.to_string(), hash.to_string()).is_none());
    }
    rows
}

fn census_scalar(text: &str, key: &str) -> String {
    let prefix = format!("{key}=");
    let mut found = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix(&prefix) {
            assert!(found.is_none(), "duplicate {key}");
            found = Some(value.to_string());
        }
    }
    found.unwrap_or_else(|| panic!("census missing {key}"))
}

fn stored_font_hashes(path: &Path) -> BTreeMap<String, String> {
    let mut rows = BTreeMap::new();
    for line in read(path).lines() {
        let Some((hash, name)) = line.split_once("  ") else {
            continue;
        };
        if hash.len() != 64 || !name.starts_with("/System/Library/Fonts/") {
            continue;
        }
        if let Some(previous) = rows.insert(name.to_string(), hash.to_string()) {
            assert_eq!(previous, hash, "{name} in {}", path.display());
        }
    }
    assert!(!rows.is_empty(), "{}", path.display());
    rows
}

fn face_paths(dir: &Path) -> BTreeMap<String, ()> {
    let mut paths = BTreeMap::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("tsv") {
            continue;
        }
        for line in read(&path).lines() {
            if let Some(rest) = line.strip_prefix("#face\t") {
                let font = rest.split('\t').nth(2).expect("#face font path");
                paths.insert(font.to_string(), ());
            }
        }
    }
    paths
}

fn ls_paths(path: &Path) -> BTreeMap<String, u64> {
    let mut current = String::new();
    let mut rows = BTreeMap::new();
    for line in read(path).lines() {
        if let Some(dir) = line.strip_prefix("## ") {
            current = dir.to_string();
            continue;
        }
        let Some(mode) = line.chars().next() else {
            continue;
        };
        if mode != '-' && mode != 'l' {
            continue;
        }
        let mut parts = line.split_whitespace();
        let size: u64 = parts.nth(4).unwrap().parse().unwrap();
        // mode, links, owner, group, size, month, day, year-or-time, then the name.
        let columns: Vec<&str> = line.split_whitespace().collect();
        assert!(columns.len() >= 9, "{line}");
        let name = columns[8..].join(" ");
        let name = if mode == 'l' {
            name.split(" -> ").next().unwrap().to_string()
        } else {
            name
        };
        let full = if name.starts_with('/') {
            name
        } else {
            format!("{current}/{name}")
        };
        assert!(rows.insert(full, size).is_none());
    }
    rows
}

#[test]
fn committed_census_matches_stored_face_extractions() {
    let root = repo_root();
    let chain = root.join("tools/font-chain-reference");
    let census_path = chain.join("monkeyman-reference-env/census.txt");
    let census = read(&census_path);
    let fonts = census_font_rows(&census);

    let records = [
        "monkeyman-round9/font-hashes.txt",
        "ascii-advances-round11/font-hashes.txt",
        "colossus-primary-faces/monkeyman-round7-font-hash.txt",
        "colossus-primary-faces/monkeyman-round8-font-hash.txt",
        "colossus-primary-faces/monkeyman-hb-static.txt",
    ];
    for record in records {
        for (path, hash) in stored_font_hashes(&chain.join(record)) {
            let census_hash = fonts.get(&path).unwrap_or_else(|| {
                panic!("{path} is hashed by {record} and missing from the census")
            });
            assert_eq!(census_hash, &hash, "{path} from {record}");
        }
    }

    let faces = face_paths(&chain.join("monkeyman-round9"));
    assert!(
        faces.len() >= 15,
        "expected the modelled face files, found {}",
        faces.len()
    );
    for path in faces.keys() {
        assert!(
            fonts.contains_key(path),
            "modelled face {path} has no census hash"
        );
    }

    let static_text = read(&chain.join("colossus-primary-faces/monkeyman-hb-static.txt"));
    let hb = "b04ab259a343ba51a50db8d1df637160f31d342a66c4192e18dba63ea42ed04a";
    assert!(static_text.contains(hb));
    assert_eq!(census_scalar(&census, "shaping.harfbuzz.sha256"), hb);
    assert_eq!(
        census_scalar(&census, "shaping.harfbuzz.loaded"),
        "/opt/homebrew/Cellar/harfbuzz/15.0.0_1/lib/libharfbuzz.0.dylib"
    );
    assert!(static_text.contains("Oct 28"));
    assert!(static_text.contains("15.0.0_1"));
    assert_eq!(
        census_scalar(&census, "shaping.harfbuzz.link_switched"),
        "2026-10-28"
    );
    assert_eq!(
        census_scalar(&census, "shaping.harfbuzz.kegs"),
        "10.1.0,13.2.1,15.0.0_1"
    );
    assert_eq!(census_scalar(&census, "host.build_version"), "26B123");
    assert_eq!(census_scalar(&census, "host.product_version"), "27.2");
    assert_eq!(
        census_scalar(&census, "jdk.java.sha256"),
        "ddbdbe280c4d939de81f27f9c135613d0a250738ab2d15e8dec52fd7345354d9"
    );
    assert_eq!(
        census_scalar(&census, "shaping.freetype.sha256"),
        "1bf70387c4ef59accba8116c847cb87d2018ded2672db551686b95dce4c5d931"
    );
    assert_eq!(
        census_scalar(&census, "shaping.freetype.loaded"),
        "/opt/homebrew/Cellar/freetype/2.15.0/lib/libfreetype.6.dylib"
    );

    let loaded = read(&chain.join("monkeyman-reference-env/methodA.loaded.txt"));
    assert!(loaded.contains("/opt/homebrew/Cellar/harfbuzz/15.0.0_1/lib/libharfbuzz.0.dylib"));
    assert!(loaded.contains("/opt/homebrew/Cellar/freetype/2.15.0/lib/libfreetype.6.dylib"));

    let script = read(&root.join("scripts/reference_env_census.py"));
    for path in fonts.keys() {
        assert!(script.contains(path), "record() would not hash {path}");
    }

    for (label, needle) in [
        ("Hack", "Hack-Regular.ttf"),
        ("MesloLGS NF", "MesloLGS NF Regular.ttf"),
        ("JetBrains Mono", "JetBrainsMonoNerdFont-Regular.ttf"),
        ("PingFang", "PingFang.ttc"),
        ("LiHei Pro", "LiHeiPro.ttf"),
    ] {
        assert!(census.contains(needle), "{label} missing from the census");
    }

    let mut users = ls_paths(&chain.join("monkeyman-round6/user-font-census.txt"));
    let assets = ls_paths(&chain.join("monkeyman-round6/asset-font-census.txt"));
    let mut census_users = BTreeMap::new();
    let mut census_assets = BTreeMap::new();
    for line in census.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.first() == Some(&"user-font") {
            census_users.insert(fields[3].to_string(), fields[1].parse::<u64>().unwrap());
        } else if fields.first() == Some(&"asset-font") {
            census_assets.insert(fields[3].to_string(), fields[1].parse::<u64>().unwrap());
        }
    }
    // Round-6 `ls` records the directory entry. The only symlink is Arial
    // Unicode, and `ls` reports the link length 52. The census row is the
    // target file from grok-g7-census3. Round-6 is not re-extracted: that
    // listing has no target hash, and rewriting it would erase the link length.
    let arial = "/Library/Fonts/Arial Unicode.ttf";
    let arial_target = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";
    assert_eq!(users.get(arial).copied(), Some(52));
    let census_arial = census_users.remove(arial).expect("arial user-font row");
    users.remove(arial);
    assert_eq!(census_users, users);
    assert_eq!(census_assets, assets);
    let arial_line = census
        .lines()
        .find(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            fields.first() == Some(&"user-font") && fields.get(3) == Some(&arial)
        })
        .expect("arial user-font row");
    let fields: Vec<&str> = arial_line.split('\t').collect();
    assert_eq!(fields.len(), 5, "{arial_line}");
    assert_eq!(fields[1].parse::<u64>().unwrap(), census_arial);
    assert_eq!(census_arial, 23_278_008);
    assert_eq!(
        fields[2],
        "876af2cd4854644e7f3e7feb2f688997fdb3343c6df6693611209c9dfb47ccec"
    );
    assert_eq!(fields[4], arial_target);
    let round6 = read(&chain.join("monkeyman-round6/user-font-census.txt"));
    assert!(round6.contains(&format!("Arial Unicode.ttf -> {arial_target}")));

    let sums = read(&chain.join("monkeyman-reference-env/SHA256SUMS"));
    for line in sums.lines() {
        let (hash, name) = line.split_once("  ").unwrap();
        let bytes = std::fs::read(chain.join("monkeyman-reference-env").join(name)).unwrap();
        let actual = sha256_hex(&bytes);
        assert_eq!(actual, hash, "{name}");
    }

    let request = root.join("tools/font-chain-reference/monkeyman-census-request");
    assert_eq!(read(&request.join("reference_env_census.py")), script);
    assert_eq!(read(&request.join("census.txt")), census);

    let status = Command::new("python3")
        .arg("-I")
        .arg(root.join("scripts/reference_env_census.py"))
        .arg("self-test")
        .status()
        .expect("python3");
    assert!(status.success(), "census self-test failed");
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

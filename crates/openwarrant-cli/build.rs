// SPDX-License-Identifier: Apache-2.0

//! Embed what this build is (OW-WAR-0143): its class, commit, dirty flag and
//! profile, read by `src/build_identity.rs` through `option_env!`.
//!
//! A release build that is not what it says — `OPENWARRANT_RELEASE_TAG` over a
//! dirty tree, or a tag that is not `v<version>` — panics here, so the binary
//! that would have claimed to be the release is never produced.

#[allow(dead_code)]
#[path = "src/build_identity.rs"]
mod build_identity;

use std::path::{Path, PathBuf};
use std::process::Command;

fn git_path(dir: &Path, args: &[&str]) -> Option<PathBuf> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if text.is_empty() {
        return None;
    }
    let path = PathBuf::from(text);
    Some(if path.is_absolute() {
        path
    } else {
        dir.join(path)
    })
}

fn watch(path: &Path) {
    // A path that does not exist would make cargo rerun this script on every
    // build; only what is there is watched.
    if path.exists() {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn main() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let version = std::env::var("CARGO_PKG_VERSION").expect("cargo sets it");
    println!(
        "cargo:rerun-if-env-changed={}",
        build_identity::RELEASE_TAG_ENV
    );
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/build_identity.rs");

    let facts = build_identity::gather(&dir, &version);
    if facts.vcs_info.is_some() {
        watch(&dir.join(".cargo_vcs_info.json"));
    }
    if facts.git.is_some() {
        // A worktree keeps HEAD and index under .git/worktrees/<name>; the
        // branch refs live in the common directory.
        if let Some(git_dir) = git_path(&dir, &["rev-parse", "--absolute-git-dir"]) {
            watch(&git_dir.join("HEAD"));
            watch(&git_dir.join("index"));
        }
        if let Some(common) = git_path(
            &dir,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        ) {
            watch(&common.join("packed-refs"));
            if let Ok(out) = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(["symbolic-ref", "-q", "HEAD"])
                .output()
            {
                let head = String::from_utf8_lossy(&out.stdout).trim().to_owned();
                if out.status.success() && !head.is_empty() {
                    watch(&common.join(head));
                }
            }
        }
        // The dirty flag is only as fresh as the last rerun: every workspace
        // crate and the lockfile are watched (20-basis.md A-003).
        if let Some(top) = git_path(&dir, &["rev-parse", "--show-toplevel"]) {
            watch(&top.join("Cargo.lock"));
            watch(&top.join("Cargo.toml"));
            if let Ok(entries) = std::fs::read_dir(top.join("crates")) {
                let mut crates: Vec<PathBuf> = entries
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .collect();
                crates.sort();
                for krate in crates {
                    watch(&krate);
                }
            }
        }
    }

    let identity = match build_identity::classify(&facts) {
        Ok(identity) => identity,
        Err(why) => panic!("refusing to build a release that is not one: {why}"),
    };
    println!("cargo:rustc-env=OW_BUILD_CLASS={}", identity.class.as_str());
    println!(
        "cargo:rustc-env=OW_BUILD_COMMIT={}",
        identity.commit.as_deref().unwrap_or("")
    );
    println!("cargo:rustc-env=OW_BUILD_DIRTY={}", identity.dirty);
    println!(
        "cargo:rustc-env=OW_BUILD_PROFILE={}",
        std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_owned())
    );
}

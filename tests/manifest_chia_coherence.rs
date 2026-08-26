//! The regression guard for dig_ecosystem#3161: this workspace MUST NEVER again declare two
//! different minor lines of the same `chia-*` family.
//!
//! ## The defect this exists to catch
//!
//! Published `datalayer-driver` 4.0.0 shipped INTERNALLY SPLIT: every chia primitive
//! (`chia-protocol`, `chia-bls`, `chia-consensus`, `chia-traits`, `chia-puzzle-types`,
//! `clvm-traits`, `clvm-utils`) was declared at `0.36.1` beside `chia-wallet-sdk` at `0.34.0`.
//! Two lines of one family in one crate compile only while every consumer happens to be stale in
//! the matching way; the moment one is not, the crate hands out two incompatible copies of the
//! same types. Because the split lived inside the PUBLISHED manifest, no consumer could escape it
//! by bumping — which is how it came to block `digs`/`digstore-chain` and `dig-wallet-backend`
//! simultaneously.
//!
//! Nothing mechanically prevented it, and no *behavioural* test can: both halves run correctly in
//! isolation, so the split is invisible to every test of what the code DOES. The defect was
//! **manifest coherence**, and manifest coherence is mechanically assertable. That is what these
//! tests assert.
//!
//! ## Why BOTH manifests, and not just the root one
//!
//! This workspace declares the same chia set twice — once in `Cargo.toml` for the library and again
//! in `napi/Cargo.toml`, whose bindings need direct access to the chia types for conversions. A
//! guard reading only the root manifest would be blind to half the surface: `napi/` could sit on
//! `0.34` while the root sat on `0.36` and the library would still build, because the two crates
//! are compiled separately and only meet at the NAPI boundary. That boundary is precisely where two
//! copies of one type produce a silent mismatch, so the two manifests are pooled into ONE coherence
//! judgement rather than checked independently.
//!
//! ## Why the manifest, and not `Cargo.lock`
//!
//! The defect is about the versions this workspace DECLARES. The resolved lock legitimately
//! contains older chia lines we neither choose nor control — `clvmr 0.16.4` vendors `chia-sha2
//! 0.34.0`, `chia-bls 0.28.2` and `chia-traits 0.28.2` internally, and `chialisp 0.4.6` (via
//! `chia-sdk-types` and `rue-lir`) pulls `chia-bls 0.42.1`. A lock-based assertion would have to
//! carve those out by name and would go red every time an upstream evaluator re-vendored something,
//! which is noise rather than signal. `cargo tree -d` cannot be the gate here for the same reason:
//! `chia-wallet-sdk` fails it on its own vendored graph. The manifest is the surface this workspace
//! owns, so it is the surface the guard pins.
//!
//! ## Why per-FAMILY, and not "all chia crates are equal"
//!
//! The ecosystem ceiling is deliberately NOT uniform, so a test asserting one global version would
//! be red on correct code: the primitives publish `0.36.1` while `chia-wallet-sdk` tops out at
//! `0.36.0`, and `chia-puzzles` (0.20.x) and `clvmr` (0.16.x) have never shared a version line with
//! either. What must agree is the `MAJOR.MINOR` **line** within a family — exactly the granularity
//! the 0.34-vs-0.36 defect violated, and exactly the granularity the legitimate 0.36.0-vs-0.36.1
//! patch spread does not.
//!
//! ## If you revert-proof these tests, restore `Cargo.lock` as well
//!
//! Proving this guard fires means reintroducing a split into a manifest and watching it fail.
//! Running the suite in that state **silently re-resolves `Cargo.lock`**. Restoring only the
//! manifest therefore leaves the lock carrying BOTH the experiment's old line and the correct one:
//! a two-line split, created by the proof, in the very workspace whose job is to have none.
//!
//! Restore both files by copy (never `git checkout <path>`, which is destructive on uncommitted
//! work), and make the restore run pass `--locked`. That flag is what turns the leftover into a
//! loud refusal instead of a silent green — without it cargo re-resolves again and the pollution
//! ships.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

/// The manifests this workspace owns, and which therefore must agree with each other.
const MANIFESTS: &[&str] = &["Cargo.toml", "napi/Cargo.toml"];

/// A dependency declaration: which manifest it came from, the crate name, and its version literal.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Declared {
    manifest: String,
    name: String,
    version: String,
}

impl Declared {
    /// The `MAJOR.MINOR` line this requirement sits on — the granularity a family must agree at.
    fn line(&self) -> String {
        self.version
            .split('.')
            .take(2)
            .collect::<Vec<_>>()
            .join(".")
    }
}

/// The families that MUST each be internally coherent, with the line each is pinned to.
///
/// `chia-wallet-sdk` is listed in the SAME family as the chia/clvm primitives precisely because the
/// shipped 4.0.0 defect split those two apart. Separating them here would classify the defect as
/// legitimate and the guard would never fire on it. `chia-puzzles` and `clvmr` version
/// independently of both and of each other, so each is its own family.
const FAMILIES: &[(&str, &str, &[&str])] = &[
    (
        "the chia 0.36 line (chia-wallet-sdk + the chia/clvm primitives)",
        "0.36",
        &[
            "chia-bls",
            "chia-consensus",
            "chia-protocol",
            "chia-puzzle-types",
            "chia-traits",
            "chia-wallet-sdk",
            "clvm-traits",
            "clvm-utils",
        ],
    ),
    ("the chia-puzzles line", "0.20", &["chia-puzzles"]),
    ("the clvmr line", "0.16", &["clvmr"]),
];

/// Reads every `chia-*`/`clvm*` requirement declared in `[dependencies]` and `[dev-dependencies]`
/// across every manifest this workspace owns.
///
/// Read at RUNTIME rather than `include_str!`d so that reverting a version in a manifest and
/// re-running the suite is a genuine end-to-end proof of the guard, with no recompilation subtlety
/// standing between the edit and the verdict.
fn declared_chia_deps() -> Vec<Declared> {
    let root = env!("CARGO_MANIFEST_DIR");
    let mut found = Vec::new();

    for relative in MANIFESTS {
        let path = format!("{root}/{relative}");
        let manifest = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("the workspace manifest `{relative}` is readable: {e}"));

        let mut section = String::new();

        for raw in manifest.lines() {
            let line = raw.trim();
            // Comments carry prose mentioning chia crates (napi/Cargo.toml explains why it needs
            // direct access to "chia types"), so they must never be parsed as declarations.
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if line.starts_with('[') {
                section = line.to_string();
                continue;
            }
            // Deliberately NOT `[target.*.dependencies]`: those carry platform shims such as
            // vendored openssl, never chia crates, and are not part of the coherence surface.
            if section != "[dependencies]" && section != "[dev-dependencies]" {
                continue;
            }
            let Some((name, rest)) = line.split_once('=') else {
                continue;
            };
            let name = name.trim();
            if !(name.starts_with("chia") || name.starts_with("clvm")) {
                continue;
            }
            let version = extract_version(rest.trim()).unwrap_or_else(|| {
                panic!(
                    "dependency `{name}` in `{relative}` declares no literal version this guard \
                     can read: {line}"
                )
            });
            found.push(Declared {
                manifest: (*relative).to_string(),
                name: name.to_string(),
                version,
            });
        }
    }

    assert!(
        !found.is_empty(),
        "the guard parsed ZERO chia dependencies, which means its parser stopped matching the \
         manifests rather than that the workspace stopped declaring chia crates. A guard that \
         silently reads nothing passes every other assertion vacuously.",
    );

    found
}

/// Pulls the version literal out of either `"0.36.1"` or `{ version = "0.36.0", features = [..] }`.
fn extract_version(rest: &str) -> Option<String> {
    let quoted = if rest.starts_with('{') {
        let at = rest.find("version")?;
        &rest[at..]
    } else {
        rest
    };
    let mut parts = quoted.split('"');
    parts.next()?;
    parts.next().map(str::to_string)
}

#[test]
fn every_chia_family_declares_a_single_minor_line() {
    let declared = declared_chia_deps();

    for (family, expected_line, members) in FAMILIES {
        for member in *members {
            for dep in declared.iter().filter(|d| d.name == *member) {
                assert_eq!(
                    dep.line(),
                    *expected_line,
                    "#3161 REGRESSION: `{}` is declared at {} in `{}` but {family} is pinned to \
                     {expected_line}.x. Two minor lines of one chia family in one workspace is the \
                     internal split that shipped as 4.0.0 and blocked every consumer. Move the \
                     whole family together, or re-state the family's line here.",
                    dep.name,
                    dep.version,
                    dep.manifest,
                );
            }
        }
    }
}

#[test]
fn the_two_manifests_agree_on_every_shared_chia_dependency() {
    // Stricter than the family check above, and catching a different failure: `Cargo.toml` at
    // 0.36.1 beside `napi/Cargo.toml` at 0.36.0 sits on ONE minor line, so the family test passes,
    // yet the library and its NAPI bindings would compile against different patch releases of the
    // same types. They meet at the NAPI boundary, so they must be declared identically.
    let mut by_name: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
    for dep in declared_chia_deps() {
        by_name
            .entry(dep.name)
            .or_default()
            .insert((dep.manifest, dep.version));
    }

    for (name, sites) in by_name {
        if sites.len() < 2 {
            continue; // declared in only one manifest — nothing to disagree with.
        }
        let versions: BTreeSet<&str> = sites.iter().map(|(_, v)| v.as_str()).collect();
        assert_eq!(
            versions.len(),
            1,
            "`{name}` is declared at differing versions across this workspace's manifests: {sites:?}. \
             The library and the NAPI bindings exchange these types across the binding boundary, so \
             a disagreement here is two copies of one type meeting at runtime.",
        );
    }
}

#[test]
fn every_declared_chia_dep_belongs_to_a_classified_family() {
    let classified: BTreeSet<&str> = FAMILIES
        .iter()
        .flat_map(|(_, _, members)| members.iter().copied())
        .collect();

    for dep in declared_chia_deps() {
        assert!(
            classified.contains(dep.name.as_str()),
            "`{}` (declared in `{}`) is a chia/clvm dependency this coherence guard does not \
             classify. A new chia dependency can silently arrive on a foreign version line, which \
             is exactly how the 4.0.0 split went unnoticed. Add it to the family it belongs to in \
             FAMILIES.",
            dep.name,
            dep.manifest,
        );
    }
}

#[test]
fn the_declared_chia_dependency_set_is_exactly_the_expected_one() {
    // Pinned by exact membership per manifest, not by count: a guard that only counts cannot tell a
    // dropped dependency from a substituted one, and both are ways for a family to lose a member
    // without the coherence test above ever seeing it (it only checks names it can find).
    let expected: BTreeSet<(&str, &str)> = BTreeSet::from([
        ("Cargo.toml", "chia-bls"),
        ("Cargo.toml", "chia-consensus"),
        ("Cargo.toml", "chia-protocol"),
        ("Cargo.toml", "chia-puzzle-types"),
        ("Cargo.toml", "chia-puzzles"),
        ("Cargo.toml", "chia-traits"),
        ("Cargo.toml", "chia-wallet-sdk"),
        ("Cargo.toml", "clvm-traits"),
        ("Cargo.toml", "clvm-utils"),
        ("Cargo.toml", "clvmr"),
        ("napi/Cargo.toml", "chia-bls"),
        ("napi/Cargo.toml", "chia-protocol"),
        ("napi/Cargo.toml", "chia-puzzle-types"),
        ("napi/Cargo.toml", "chia-traits"),
        ("napi/Cargo.toml", "chia-wallet-sdk"),
    ]);

    let owned: Vec<Declared> = declared_chia_deps();
    let actual: BTreeSet<(&str, &str)> = owned
        .iter()
        .map(|d| (d.manifest.as_str(), d.name.as_str()))
        .collect();

    assert_eq!(
        actual, expected,
        "the workspace's chia/clvm dependency set changed. Confirm the new set is on ONE line per \
         family in BOTH manifests, then update this expectation in the same commit.",
    );
}

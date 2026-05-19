//! Verification: the embedded OEIS reference vector and a tolerant b-file
//! parser.
//!
//! Milestone **M4** (PLAN.md). DESIGN.md §6, §7(a), §7(g).

use crate::Count;
use std::io::{self, BufRead};
use std::path::Path;

/// OEIS **A142886**, `a(n)` for `n = 0..=68` — the canonical reference
/// prefix (single source of truth for every correctness test).
/// Source: <https://oeis.org/A142886>.
pub const REFERENCE: [Count; 69] = [
    1, 1, 0, 0, 1, 1, 0, 0, 1, 2, 0, 0, 3, 2, 0, 0, 5, 4, 0, 0, 12, 7, 0, 0, 20, 11, 0, 0, 45, 20,
    0, 0, 80, 36, 0, 0, 173, 65, 0, 0, 310, 117, 0, 0, 664, 216, 0, 0, 1210, 396, 0, 0, 2570, 736,
    0, 0, 4728, 1369, 0, 0, 9976, 2558, 0, 0, 18468, 4787, 0, 0, 38840,
];

/// Largest `n` the §4.2 baseline reaches at a practical on-demand cost
/// (≈6 s for n=68, ≈30 s for the whole `0..=68` sweep, release build;
/// cost roughly triples per +4 beyond that). Deep checks are capped here;
/// the full b-file range (`n ≤ 163`) needs the M6 rewrite (DESIGN.md §4.5,
/// §7 baseline-runtime note).
pub const DEEP_BOUND: usize = 68;

/// Parse an OEIS b-file (`n a(n)` per line; `#`-comments and blank lines
/// skipped) into `(n, a(n))` pairs. The crate never fetches this file; the
/// caller supplies it (DESIGN.md §4.5 / PLAN.md M5).
pub fn parse_bfile(path: &Path) -> io::Result<Vec<(usize, Count)>> {
    let file = std::fs::File::open(path)?;
    let mut out = Vec::new();
    for line in io::BufReader::new(file).lines() {
        let line = line?;
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let mut it = t.split_whitespace();
        let (Some(ns), Some(vs)) = (it.next(), it.next()) else {
            continue;
        };
        let parse_err = |what: &str, tok: &str| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("b-file: bad {what} {tok:?}"),
            )
        };
        let n: usize = ns.parse().map_err(|_| parse_err("index", ns))?;
        let v: Count = vs.parse().map_err(|_| parse_err("value", vs))?;
        out.push((n, v));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_bfile_is_tolerant() {
        let mut path = std::env::temp_dir();
        path.push("a142886_bfile_test.txt");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            // comments, blank lines, and leading/trailing whitespace
            writeln!(f, "# A142886 b-file (test fixture)").unwrap();
            writeln!(f).unwrap();
            writeln!(f, "0 1").unwrap();
            writeln!(f, "  1   1  ").unwrap();
            writeln!(f, "4 1").unwrap();
            writeln!(f, "9 2").unwrap();
        }
        let got = parse_bfile(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(got, vec![(0, 1), (1, 1), (4, 1), (9, 2)]);
        // fixture agrees with the embedded reference
        for (n, v) in got {
            assert_eq!(v, REFERENCE[n]);
        }
    }

    /// Test (g) — DESIGN.md §7(g): regression against the authoritative OEIS
    /// b-file. Formerly `#[ignore]`d; now always-on (runs in <0.01s on the
    /// optimized enumerator). Still bounded at [`DEEP_BOUND`] (the literal
    /// `0..=163` is infeasible — DESIGN.md §4.5 / §7); skips gracefully if
    /// `b142886.txt` is absent from the crate root.
    #[test]
    fn matches_bfile() {
        use crate::count;
        let path = Path::new("b142886.txt");
        if !path.exists() {
            eprintln!(
                "skip: b142886.txt not in CWD \
                 (curl -L https://oeis.org/A142886/b142886.txt -o b142886.txt)"
            );
            return; // opt-in regression: absent data is not a failure
        }
        let rows = parse_bfile(path).expect("parse b142886.txt");
        let mut checked = 0usize;
        for (n, expected) in rows {
            if n > DEEP_BOUND {
                break;
            }
            // cross-check the independent oracle against the embedded vector
            if n < REFERENCE.len() {
                assert_eq!(expected, REFERENCE[n], "b-file vs REFERENCE at n={n}");
            }
            assert_eq!(count(n), expected, "count vs b-file at n={n}");
            checked += 1;
        }
        assert!(checked > DEEP_BOUND, "only {checked} terms checked");
        eprintln!("matches_bfile: {checked} terms verified (n=0..={DEEP_BOUND})");
    }
}

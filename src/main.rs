//! CLI for the A142886 enumerator.
//!
//! Milestone **M4** (PLAN.md, DESIGN.md §6).

use a142886::verify::{parse_bfile, REFERENCE};
use a142886::{
    count, count_cell_centered, count_cell_centered_parallel, count_parallel,
    count_vertex_centered, count_vertex_centered_parallel, Count,
};
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Center {
    Cell,
    Vertex,
    Both,
}

impl Center {
    /// The term function for this center selection.
    fn term(self, parallel: bool) -> fn(usize) -> Count {
        match (self, parallel) {
            (Center::Cell, false) => count_cell_centered,
            (Center::Vertex, false) => count_vertex_centered,
            (Center::Both, false) => count,
            (Center::Cell, true) => count_cell_centered_parallel,
            (Center::Vertex, true) => count_vertex_centered_parallel,
            (Center::Both, true) => count_parallel,
        }
    }
}

#[derive(Debug)]
struct Args {
    max_n: usize,
    center: Center,
    verify: bool,
    parallel: bool,
    /// Per-term checkpoint file. On startup, parses existing `n a(n)`
    /// lines and skips them. Each newly computed term is appended +
    /// fsync'd so a kill / SIGTERM / spot preemption loses at most one
    /// in-flight term. None = no checkpointing.
    checkpoint: Option<PathBuf>,
}

const HELP: &str = "\
a142886 — enumerate OEIS A142886 (polyominoes with full D8 symmetry)

USAGE:
    a142886 [--max-n N] [--center cell|vertex|both] [--verify] [--parallel]
            [--checkpoint FILE]

OPTIONS:
    --max-n N                 print n a(n) for n = 0..=N (default 0)
    --center cell|vertex|both restrict the enumeration (default both)
    --verify                  check count() vs the embedded OEIS reference
                              (and ./b142886.txt if present); ignores --center
    --parallel                bucket-level rayon over x-axis buckets, with
                              cell + vertex run concurrently via rayon::join.
                              Byte-identical counts; control thread count via
                              RAYON_NUM_THREADS.
    --checkpoint FILE         per-term checkpoint: at startup, read FILE and
                              skip any n already present (echoed to stdout for
                              continuity); each new `n a(n)` is appended +
                              fsync'd so a kill loses at most the in-flight
                              term. Resume by re-running with the same flag.
    -h, --help                show this help

NOTE: cost grows steeply with N (see DESIGN.md §7 baseline-runtime note).";

fn parse_args() -> Result<Args, String> {
    let mut max_n: usize = 0;
    let mut center = Center::Both;
    let mut verify = false;
    let mut parallel = false;
    let mut checkpoint: Option<PathBuf> = None;

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--max-n" => {
                let v = it.next().ok_or("--max-n requires a value")?;
                max_n = v
                    .parse()
                    .map_err(|_| format!("invalid --max-n value: {v:?}"))?;
            }
            "--center" => {
                let v = it.next().ok_or("--center requires a value")?;
                center = match v.as_str() {
                    "cell" => Center::Cell,
                    "vertex" => Center::Vertex,
                    "both" => Center::Both,
                    other => return Err(format!("invalid --center value: {other:?}")),
                };
            }
            "--verify" => verify = true,
            "--parallel" => parallel = true,
            "--checkpoint" => {
                let v = it.next().ok_or("--checkpoint requires a path")?;
                checkpoint = Some(PathBuf::from(v));
            }
            "-h" | "--help" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other:?}")),
        }
    }

    Ok(Args {
        max_n,
        center,
        verify,
        parallel,
        checkpoint,
    })
}

/// Print `n a(n)` for n = 0..=max_n using the selected center. When
/// `--checkpoint FILE` is set, also resumes from / appends to FILE.
fn print_table(args: &Args) {
    let term = args.center.term(args.parallel);

    // Load any already-computed terms from the checkpoint file; echo them
    // to stdout so the full sequence is visible regardless of resume.
    let (mut already_done, mut ckpt_file) = if let Some(path) = &args.checkpoint {
        let mut done: HashSet<usize> = HashSet::new();
        if path.exists() {
            let contents = std::fs::read_to_string(path).unwrap_or_else(|e| {
                panic!("--checkpoint: cannot read {}: {e}", path.display())
            });
            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let n_str = line.split_whitespace().next().unwrap_or("");
                if let Ok(n) = n_str.parse::<usize>() {
                    done.insert(n);
                    println!("{line}");
                }
            }
        }
        let f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap_or_else(|e| {
                panic!("--checkpoint: cannot open {} for append: {e}", path.display())
            });
        (done, Some(f))
    } else {
        (HashSet::new(), None)
    };

    for n in 0..=args.max_n {
        if already_done.contains(&n) {
            continue;
        }
        let val = term(n);
        println!("{n} {val}");
        if let Some(ref mut f) = ckpt_file {
            // Append + fsync per term so a kill / SIGTERM / preemption
            // loses at most the in-flight term. Per-term I/O cost is
            // ~µs (single ~30-byte append + fsync), negligible vs the
            // seconds-to-days per-term compute at large n.
            writeln!(f, "{n} {val}").unwrap_or_else(|e| {
                panic!("--checkpoint: write at n={n} failed: {e}")
            });
            f.sync_all().unwrap_or_else(|e| {
                panic!("--checkpoint: fsync at n={n} failed: {e}")
            });
            already_done.insert(n);
        }
    }
}

/// Compare `count()` against the embedded reference (and an optional local
/// b-file). Returns the number of mismatches found. Uses the parallel path
/// when `--parallel` is set, so `--parallel --verify` is the byte-identical
/// correctness gate for the parallel enumerator (the reference vector is
/// the independently-verified OEIS oracle).
fn run_verify(args: &Args) -> usize {
    // `--verify` ignores `--center` (always sums both) — pick the right
    // total-count fn based on `--parallel`.
    let term: fn(usize) -> Count = if args.parallel { count_parallel } else { count };
    // Cap at the reference's range and keep the default fast. The parens
    // matter: `.min` must apply to the whole if-expression, not the `else`.
    let upto = (if args.max_n > 0 { args.max_n } else { 40 }).min(REFERENCE.len() - 1);
    let mut mismatches = 0usize;

    for (n, &expected) in REFERENCE.iter().enumerate().take(upto + 1) {
        let got = term(n);
        if got != expected {
            eprintln!("MISMATCH vs reference at n={n}: got {got}, expected {expected}");
            mismatches += 1;
        }
    }
    println!(
        "reference: checked n=0..={upto} ({} terms){}",
        upto + 1,
        if mismatches == 0 {
            " — all match"
        } else {
            ""
        }
    );

    let bpath = Path::new("b142886.txt");
    if bpath.exists() {
        match parse_bfile(bpath) {
            Ok(rows) => {
                let mut checked = 0usize;
                for (n, expected) in rows {
                    if n > upto {
                        continue; // keep within a feasible bound
                    }
                    checked += 1;
                    let got = term(n);
                    if got != expected {
                        eprintln!("MISMATCH vs b-file at n={n}: got {got}, expected {expected}");
                        mismatches += 1;
                    }
                }
                println!(
                    "b142886.txt: checked {checked} terms (n ≤ {upto}){}",
                    if mismatches == 0 {
                        " — all match"
                    } else {
                        ""
                    }
                );
            }
            Err(e) => eprintln!("warning: could not read b142886.txt: {e}"),
        }
    } else {
        println!("b142886.txt: not present (skipped)");
    }

    mismatches
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}\n\n{HELP}");
            return ExitCode::FAILURE;
        }
    };

    if args.verify {
        let mismatches = run_verify(&args);
        if mismatches == 0 {
            println!("OK");
            ExitCode::SUCCESS
        } else {
            eprintln!("FAILED: {mismatches} mismatch(es)");
            ExitCode::FAILURE
        }
    } else {
        print_table(&args);
        ExitCode::SUCCESS
    }
}

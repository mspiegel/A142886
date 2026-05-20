# PLAN: Implementing OEIS A142886

This is the execution plan for the Rust crate specified in [`DESIGN.md`](./DESIGN.md).
It **operationalizes** the design — sequencing, acceptance criteria, file
ownership — and introduces **no new technical decisions**. Every milestone
traces back to a DESIGN.md section (see the Traceability table) and ends with
a green `cargo test`.

**Goal:** compute `a(n)` for OEIS A142886 (polyominoes with full D₈ square
symmetry) as far as feasible, verified against the OEIS b-file
(`https://oeis.org/A142886/b142886.txt`, n = 0..163).

## Conventions

- Count type alias `pub type Count = u64;` everywhere a term value flows; a
  big-integer backend (`num-bigint`) is a single-line swap if ever needed
  (DESIGN §5 argues `u64` suffices through n = 163).
- Lattice cells are `(i32, i32)`; wedge occupancy is a `HashSet<(i32,i32)>`
  (packed bitset is a later optimization, not a correctness concern).
- Errors via `anyhow` in `main`/`verify`; library code returns `Result` or
  panics only in tests.
- Each milestone must leave the tree `cargo fmt`-clean, `cargo clippy`-clean
  (no warnings), and `cargo test` green before the next milestone starts.
- Test labels `(a)`–`(g)` below refer to DESIGN.md §7.

## Milestone M0 — Scaffold

*Design ref: §5, §6.* Files: `Cargo.toml`, `src/main.rs`, `src/symmetry.rs`,
`src/enumerate.rs`, `src/connectivity.rs`, `src/verify.rs`.

- [ ] `Cargo.toml`: package name `a142886`, edition 2021; `[features]
      parallel = ["rayon"]` with `rayon` optional; dev-deps as needed.
- [ ] `src/` skeleton with the five modules; declare `mod` tree in `main.rs`
      (or `lib.rs` + thin `main.rs` so tests can call the library).
- [ ] `pub type Count = u64;` and the public API signatures from §5 as
      `todo!()` stubs: `count`, `count_cell_centered`, `count_vertex_centered`.
- [ ] CLI argument parsing stub: `--max-n N`, `--center cell|vertex|both`
      (default `both`), `--verify`.
- [ ] `#[cfg(test)] mod tests` harness compiling (empty).

**Acceptance:** `cargo build` and `cargo test` (no tests) both succeed.

## Milestone M1 — Symmetry core

*Design ref: §2, §3.1, §3.2, §7(e).* File: `src/symmetry.rs`.

- [ ] The 8 transforms as the closed-form maps from the §2 table
      (`e, r, r², r³, s, sr, sr², sr³`).
- [ ] Cell-centered: `orbit_cell((x,y))`, `representative_in_W` with the
      wedge-edge tie-break, and an orbit-size classifier returning
      1 (apex) / 4 (x-axis or diagonal edge) / 8 (interior) per §3.1.
- [ ] Vertex-centered: orbit + representative + classifier per §3.2
      (diagonal cells `(i,i)` → size 4; others → size 8; 2×2 core cell).

**Acceptance:** test **(e)** `group_axioms_and_orbit_sizes` passes; orbit
sizes equal the §3.1 / §3.2 tables for sampled apex/edge/interior cells.

## Milestone M2 — Connectivity predicate

*Design ref: §4.1, §4.3, §7(f).* File: `src/connectivity.rs`.

- [ ] `slice_is_connected_polyomino(S) -> bool` = **(i)** `S` is one
      4-connected component (BFS or union–find over the wedge cells) **and**
      **(ii)** `S` has an occupied cell on the x-axis edge *and* one on the
      diagonal edge (apex / 2×2-core cell satisfies both alone). This is the
      §4.1 lemma; the only acceptance test on the hot path.
- [ ] Debug-only `reconstruct_then_bfs(S)`: `⋃_{g∈D₈} g·S`, assert
      `|P| == expected_n`, BFS connectivity — gated behind
      `debug_assertions` / used only from tests, never the counting loop.

**Acceptance:** tests **(f)** `slice_predicate_edge_conditions` and
`slice_predicate_matches_reconstruction` pass — the predicate agrees with
brute reconstruction over **all** small slices for **both** center types,
and the §4.1 edge-condition cases (`{(3,1)}`, `{(2,1),(3,1)}`,
`{(1,0),(1,1)}`, `{(0,0),(1,0)}`, `{(2,1),(2,2)}`) resolve as specified.

## Milestone M3 — Enumeration

*Design ref: §3, §4.2, §4.4, §7(b)(c)(d).* File: `src/enumerate.rs`.

- [x] Redelmeier untried-set recursive growth restricted to `W`, **bucketed
      by the slice's minimal x-axis cell** `A=(ax,0)` and grown from the
      pinned root `A`; the per-bucket `blocked` set carries Redelmeier's
      ancestor-exclusion, so injectivity holds *without* the old global
      lex-min `nb>seed` shortcut and each valid slice is generated in
      exactly one bucket exactly once (DESIGN §4.2; ≈2× faster than the
      prior lex-min-seed scheme, counts byte-identical n≤100).
- [x] Orbit-size accounting (§3.1 / §3.2) to track the reconstructed size
      and stop exactly at `n`; prune any branch whose weight exceeds `n`.
- [x] Acceptance via the §4.1 edge-touch conditions on the slice
      (connectivity is by construction; no reconstruction on the hot path).
- [x] Base / edge cases: `count(0) == 1` hard-coded; `n ≡ 2,3 (mod 4) ⇒ 0`;
      cell-centered drives `n ≡ 0,1`, vertex-centered only `n ≡ 0 (mod 4)`.
- [x] Wire `count_cell_centered`, `count_vertex_centered`, and
      `count = cell + vertex` (disjoint per §3.3, summed directly).
- [x] §4.6 count-preserving prunes: residue-class bucket split (skip the
      dead parity's buckets — apex-only for `n ≡ 1`, apex-skipped for
      `n ≡ 0`) and the admissible edge-reachability bound (joint
      cell-budget+gap diagonal term `8·min_gap−4`; DESIGN §4.6(b)). Counts
      byte-identical to the §4.2 baseline (regression vs b-file + reference,
      n = 0..68); measured ≈40–95× speedup, growing with n.

**Acceptance:** tests **(b)** named shapes, **(c)** zero-pattern, **(d)**
split-formula pass; the §3.4 hand cases n = 1,4,5,8,9 reproduce
(`a(9)=2` is the decisive arithmetic-plus-connectivity check). Plus an
always-on `matches_oeis_prefix_to_40` early correctness check. Per the
DESIGN.md §7 baseline-runtime note, always-on heavy `n ≡ 0,1 (mod 4)` checks
are bounded at **n ≤ 40** (`HEAVY_BOUND`); the full `0..=68` prefix and the
b-file are the on-demand deep checks in M5.

## Milestone M4 — Verification & CLI

*Design ref: §6, §7(a).* Files: `src/verify.rs`, `src/main.rs`.

- [x] Embed the 69-term reference vector (§7a, `verify::REFERENCE`, single
      source of truth); always-on `matches_oeis_prefix_to_40` plus
      `#[ignore]`d `matches_oeis_prefix_full` (0..=68) deep check.
- [x] `parse_bfile(path)` → `Vec<(usize, Count)>` (comment/blank-tolerant)
      with a fast unit test.
- [x] `--verify`: compare `count()` to the embedded vector, and to
      `b142886.txt` if present in CWD; clear pass/fail summary + exit code.
- [x] Finish CLI: `--max-n` prints `n a(n)` (center-aware via `Center::term`);
      `--center cell|vertex|both`; `-h/--help`; bad args → help + exit 1.

**Acceptance:** test **(a)** passes; `cargo run -- --max-n 40` prints the
correct table; `cargo run -- --verify` reports all-match on the prefix.

## Milestone M5 — Depth / b-file regression

*Design ref: §4.5, §7(g).* File: tests in `src/verify.rs`.

- [x] Obtain `b142886.txt` (`curl -L https://oeis.org/A142886/b142886.txt`;
      164 lines, n=0..163; the crate never fetches it).
- [x] `#[ignore]`d `matches_oeis_prefix_full` (0..=68) and `matches_bfile`
      (count vs b-file *and* b-file vs `REFERENCE`), bounded at
      `verify::DEEP_BOUND = 68`; absent b-file → skip, not fail.
- [x] Measured release timing: pre-§4.6 n=60 ≈2.1 s, n=64 ≈4.0 s, n=68
      ≈10 s; post-§4.6 n=68 ≈0.11 s. Switching §4.2 to minimal-x-axis-cell
      bucketing is a further ≈2×, and the §4.6(b) joint cell-budget+gap
      diagonal term a further ≈1.33× (n=100 → ≈0.9 s, ratios non-eroding;
      DESIGN §4.6 tables); the full `0..=68` deep `--ignored` sweep is now
      well under 0.1 s.

**Achieved depth:** `count(n) == a(n)` verified for **n = 0..=68** against
both the OEIS b-file and the embedded `REFERENCE`, zero mismatches. `u64`
empirically sufficient (a(161)=29 256 182 414 ≈ 2.9e10 ≪ u64::MAX). The
remaining `69..=163` of the b-file is bounded only by the exponential
enumeration (§4.5); reachable depth is empirical, not a fixed target. (A
two-terminal `(A,B)` variant pinning the minimal diagonal cell was evaluated
and was not faster once §4.6 / x-axis bucketing was in place — see §4.5 — so
it is not pursued. The transfer/kernel reformulation was reopened and
**built/measured under M6 for the n ≫ 110 regime, then rejected** with hard
numbers — see §4.7 / PERFORMANCE.md.)

**Acceptance:** `cargo test --release -- --ignored` matches the b-file to
n=68 with zero mismatches — **met**.

## Milestone M6 — Transfer-matrix enumerator (n ≫ 110) — REJECTED

*Design ref: §4.7. Outcome: **NO-GO**.* The hypothesis that the §4.5
transfer-matrix rejection was merely depth-conditioned (and would invert for
n ≫ 110) was tested by building the engine in full and measuring it. The
`enumerate.rs` split was a pure `git mv`; on NO-GO it was reverted cleanly,
so the repo is back at the M5 state.

- [x] **Phase 1 — scaffolding.** Anti-diagonal `WedgeScan`, slot indexing,
      `max_scan_d`, ported wedge/edge helpers. Geometry tests green.
- [x] **Phase 2 — weight DP.** `(weight, edge-flags)` knapsack; brute-exact
      vs a subset oracle, over-counts `legacy` as expected.
- [x] **Phase 3 — connectivity signature.** Jensen anti-diagonal DP
      (union–find, non-crossing partition, sole-completion retirement).
      **Byte-identical to `legacy`** for all feasible n ≤ 24, and matched
      `reconstruct_then_bfs` independently at small n.
- [x] **Phase 4 — differential + profile.** Byte-identical to `legacy`
      (count/cell/vertex) for every feasible n ≤ 57. Attribution profile
      (cell, release): runtime ≈ 3.0×/+4 (≈1.32ⁿ); **distinct frontier
      states ≈ 2.3×/+4 (≈1.23ⁿ)** vs output / §4.6 engine ≈ 2.0×/+4
      (≈1.19ⁿ). ≈10³× slower than `legacy` at n=48, gap widening.
- [x] **Phase 5 — NO-GO.** The state-count floor (≈1.23ⁿ) is strictly above
      the §4.6 enumerator (≈1.19ⁿ), so even an idealized cell-by-cell rewrite
      is asymptotically *worse*; n=140/160 infeasible. Discarded `transfer`,
      reverted the split, numbers recorded in PERFORMANCE.md.

**Outcome:** correctness was never the issue (byte-identical through n≤57);
the asymptotics are. The transfer matrix on A142886's own D₈ wedge does not
yield an output-sized state space, so the §4.5 rejection **extends to
n ≫ 110**. The b-file's depth is the §3.3 composition of *siblings* (computed
by transfer matrices on far smaller fundamental regions), not a transfer
matrix on this wedge. Repo reverted to M5; DESIGN/PLAN/PERFORMANCE kept in sync.

## Traceability

| Milestone | DESIGN.md sections | Tests |
|---|---|---|
| M0 Scaffold | §5, §6 | build only |
| M1 Symmetry | §2, §3.1, §3.2 | (e) |
| M2 Connectivity | §4.1, §4.3 | (f) |
| M3 Enumeration | §3, §4.2, §4.4, §4.6 | (b), (c), (d) |
| M4 Verify/CLI | §6, §7(a) | (a) |
| M5 Depth | §4.5, §7(g) | (g) |
| M6 Transfer-matrix (rejected) | §4.7 | built, profiled, NO-GO |

Every DESIGN.md component maps to exactly one milestone; nothing is orphaned.
The §4.1 connectivity lemma (M2) is the load-bearing correctness item — its
brute-force agreement test (f) gates everything downstream.

## Risks & notes

- **b-file needs network.** Fetching `b142886.txt` is out-of-band (M5); the
  crate never makes network calls. Suggested: `curl -L
  https://oeis.org/A142886/b142886.txt -o b142886.txt` run by the user.
- **Depth is empirical.** The reachable `n` depends on machine/time budget;
  M5 records the achieved figure rather than promising n = 163. The §4.2/§4.6
  enumeration is exponential (§4.5). The b-file reaches n=163 only as the
  OEIS composition `A351127+A346800(n/4)` (§3.3). The "rejection is only
  depth-conditioned" hypothesis was tested under **M6**: the transfer matrix
  was built and profiled (§4.7) and **rejected with hard numbers** — its
  state-count floor (≈1.23ⁿ) is asymptotically worse than the §4.6 enumerator
  (≈1.19ⁿ). The rejection extends to n ≫ 110; greater depth is more compute,
  not an algorithm swap.
- **`u64` sufficiency.** Argued in DESIGN §5; the `Count` alias is the escape
  hatch to `num-bigint` with no call-site churn.
- **No design drift.** Any change to the algorithm, orbit arithmetic, or the
  connectivity criterion is a DESIGN.md change first, then reflected here —
  PLAN.md must not diverge from DESIGN.md.

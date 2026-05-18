//! Anti-diagonal Jensen transfer-matrix enumerator for OEIS A142886
//! (DESIGN.md §4.7). Counts D8-symmetric polyominoes by sweeping the
//! fundamental wedge `W = {(x,y): 0 ≤ y ≤ x}` along anti-diagonals
//! `d = x + y`, summing a bounded frontier state instead of enumerating each
//! slice — the asymptotic class change targeting the n ≫ 110 regime where
//! the §4.5 *depth-conditioned* rejection of a transfer matrix no longer
//! holds (see `mod.rs`, FUTURE.md, [[a142886-transfer-matrix-reopened]]).
//!
//! **M6 bring-up status: Phase 1 — frontier scaffolding only.** This module
//! provides the anti-diagonal scan geometry, slot indexing, the ported
//! wedge/edge helpers, and the scan-extent bound. The DP core (Phase 2),
//! connectivity signature (Phase 3) and differential gate (Phase 4) build on
//! these. No public counting entry point yet; the live engine is `legacy`.

// Bring-up scaffolding is reachable only from tests until the DP lands;
// removed at GO when this module becomes the live engine.
#![allow(dead_code)]

use crate::symmetry::Cell;

// ── Pure wedge / edge helpers ───────────────────────────────────────────────
// Ported verbatim from the legacy engine (enumerate/legacy.rs §4.6,
// `in_wedge`/`on_x_axis_edge`/`on_diagonal_edge`/`edge_reach_lb`/`NEIGHBOURS`).
// Keeping one shared definition would couple `transfer` to `legacy`, which is
// deleted at GO; the duplication is temporary (NO-GO discards `transfer`, GO
// deletes `legacy` — exactly one copy survives either way) and intentional so
// the weight/edge rules are the *same code*, not a reimplementation.

/// 4-neighbour offsets (legacy `NEIGHBOURS`).
pub(crate) const NEIGHBOURS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Is `c` in the fundamental wedge `W = {(x,y): 0 ≤ y ≤ x}`?
#[inline]
pub(crate) fn in_wedge(c: Cell) -> bool {
    c.0 >= 0 && c.1 >= 0 && c.1 <= c.0
}

/// On the x-axis edge of `W` (the §4.1 condition-2 x-axis cell, `y = 0`).
#[inline]
pub(crate) fn on_x_axis_edge(c: Cell) -> bool {
    c.1 == 0
}

/// On the diagonal edge of `W` (the §4.1 condition-2 diagonal cell, `x = y`).
#[inline]
pub(crate) fn on_diagonal_edge(c: Cell) -> bool {
    c.0 == c.1
}

/// Admissible lower bound on the *extra* orbit-weight any completion still
/// needs to satisfy §4.1 condition 2 (touch both wedge edges). Verbatim port
/// of the legacy joint cell-budget + gap bound (DESIGN §4.6(b)): the two
/// excursions may share cells, so the sound combined bound is `max`, never
/// the sum. `td == 0 ⇒ min_gap ≥ 1`, so `8·min_gap − 4 ≥ 4` (no underflow).
#[inline]
pub(crate) fn edge_reach_lb(tx: u32, td: u32, min_y: i32, min_gap: i32) -> u64 {
    debug_assert!(td > 0 || min_gap >= 1, "td == 0 must imply min_gap ≥ 1");
    let wx = if tx > 0 { 0 } else { 4 * min_y as u64 };
    let wd = if td > 0 { 0 } else { 8 * (min_gap as u64) - 4 };
    wx.max(wd)
}

// ── Anti-diagonal geometry ──────────────────────────────────────────────────
//
// On anti-diagonal `d = x + y`, the wedge cells are exactly
// `{ (d−y, y) : 0 ≤ y ≤ ⌊d/2⌋ }` (need `0 ≤ y` and `y ≤ x = d−y`). So the
// anti-diagonal has `⌊d/2⌋ + 1` cells and width grows by ≤ 1 every two
// anti-diagonals — the property that keeps the frontier (hence the state
// space) small, unlike a column scan whose width is ≈ n/2.

/// Number of wedge cells on anti-diagonal `d` (`d ≥ 0`): `⌊d/2⌋ + 1`.
#[inline]
pub(crate) fn antidiag_len(d: i32) -> usize {
    debug_assert!(d >= 0);
    (d / 2) as usize + 1
}

/// The wedge cell at anti-diagonal `d`, slot `y` (`0 ≤ y ≤ ⌊d/2⌋`). The slot
/// index *is* the `y` coordinate, so a frontier array indexed by `y` aligns
/// across consecutive anti-diagonals (a cell's down-neighbour keeps slot `y`).
#[inline]
pub(crate) fn cell_at(d: i32, y: i32) -> Cell {
    debug_assert!(d >= 0 && 0 <= y && y <= d / 2);
    (d - y, y)
}

/// Rigorous upper bound on the largest anti-diagonal `d = x + y` any
/// weight-≤`n` **valid** wedge slice can occupy.
///
/// Every occupied cell has orbit weight ≥ 4 except the single cell-centered
/// apex `(0,0)` (weight 1, at `d = 0`), so `|S| ≤ n/4 + 1`. For a *valid*
/// slice the bounding box obeys the connected-set inequality
/// `W_x + W_y ≤ |S| + 1` (`W_x,W_y` = box width/height). It touches `y=0`
/// (min_y = 0) and the diagonal at some `(k,k)`, so `W_y ≥ k + 1` and
/// `min_x ≤ k ≤ max_x`. Hence
/// `max_x = min_x + W_x − 1 ≤ k + (|S| − k) − 1 = |S| − 1`, and with `y ≤ x`
/// `d = x + y ≤ 2·max_x ≤ 2(|S| − 1) ≤ 2·(n/4 + 1)`. Truncating the scan
/// here is sound: any partial config needing a farther cell has no valid
/// completion and contributes 0.
///
/// Deliberately loose: the per-state `edge_reach_lb` prune (DESIGN §4.6(b),
/// wired in Phase 2+) tightens the *live* scan dynamically. Correctness only
/// needs a safe over-estimate here.
#[inline]
pub(crate) fn max_scan_d(n: usize) -> i32 {
    if n == 0 {
        return 0;
    }
    2 * (n as i32 / 4 + 1)
}

/// Number of frontier slots that can ever be live for target `n` — the length
/// of the widest anti-diagonal up to [`max_scan_d`]. The DP frontier array is
/// indexed by `y ∈ 0..max_slots(n)`; must stay ≤ 64 for the `u64` occupancy
/// mask (asserted by [`frontier_fits_u64`]).
#[inline]
pub(crate) fn max_slots(n: usize) -> usize {
    antidiag_len(max_scan_d(n))
}

/// Does the n=163 frontier fit the `u64` occupancy mask? (Documents the
/// invariant the DP relies on; checked by a unit test, not at runtime.)
#[inline]
pub(crate) fn frontier_fits_u64(n: usize) -> bool {
    max_slots(n) <= 64
}

/// The anti-diagonal scan order: yields `(d, y, cell)` for every wedge cell
/// with `d ≤ max_d`, in nondecreasing `d` then nondecreasing `y` — the order
/// the transfer-matrix DP processes cells in (a 4-step changes `d` by ±1, so
/// the frontier between processed `x+y < d` and unprocessed cells is a clean
/// staircase).
pub(crate) struct WedgeScan {
    d: i32,
    y: i32,
    max_d: i32,
}

impl WedgeScan {
    #[inline]
    pub(crate) fn new(max_d: i32) -> Self {
        WedgeScan {
            d: 0,
            y: 0,
            max_d,
        }
    }
}

impl Iterator for WedgeScan {
    /// `(d, y, cell)` — `d` the anti-diagonal, `y` the slot (= y-coordinate),
    /// `cell = (d − y, y)`.
    type Item = (i32, i32, Cell);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.d > self.max_d {
                return None;
            }
            if self.y > self.d / 2 {
                self.d += 1;
                self.y = 0;
                continue;
            }
            let (d, y) = (self.d, self.y);
            self.y += 1;
            return Some((d, y, (d - y, y)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// The anti-diagonal scan visits **exactly** the wedge triangle
    /// `{ (x,y) : 0 ≤ y ≤ x, x+y ≤ R }`, each cell once, in
    /// nondecreasing `(d, y)` order, every cell in `W`.
    #[test]
    fn scan_covers_wedge_triangle_in_order() {
        for r in 0..=24 {
            let mut direct: HashSet<Cell> = HashSet::new();
            for x in 0..=r {
                for y in 0..=x {
                    if x + y <= r {
                        direct.insert((x, y));
                    }
                }
            }
            let mut seen: HashSet<Cell> = HashSet::new();
            let mut prev = (-1, -1);
            for (d, y, c) in WedgeScan::new(r) {
                assert!(in_wedge(c), "scan emitted non-wedge {c:?}");
                assert_eq!(c, (d - y, y));
                assert_eq!(d, c.0 + c.1, "d must equal x+y");
                assert!(0 <= y && y <= d / 2, "slot {y} out of range for d={d}");
                assert!((d, y) > prev, "scan not in (d,y) order at {c:?}");
                prev = (d, y);
                assert!(seen.insert(c), "scan emitted {c:?} twice");
            }
            assert_eq!(seen, direct, "scan != wedge triangle at R={r}");
        }
    }

    /// `antidiag_len(d)` equals the number of cells the scan emits on `d`.
    #[test]
    fn antidiag_len_matches_scan() {
        let max_d = 30;
        let mut per_d = vec![0usize; (max_d + 1) as usize];
        for (d, _, _) in WedgeScan::new(max_d) {
            per_d[d as usize] += 1;
        }
        for (d, &cnt) in per_d.iter().enumerate() {
            assert_eq!(cnt, antidiag_len(d as i32), "len mismatch at d={d}");
        }
        // ⌊d/2⌋+1: 1,1,2,2,3,3,…
        assert_eq!(antidiag_len(0), 1);
        assert_eq!(antidiag_len(1), 1);
        assert_eq!(antidiag_len(2), 2);
        assert_eq!(antidiag_len(7), 4);
    }

    /// The §4.1 condition-2 edge predicates at the named geometric cases.
    #[test]
    fn edge_predicates_at_named_cells() {
        assert!(on_x_axis_edge((0, 0)) && on_diagonal_edge((0, 0))); // apex/core
        assert!(on_x_axis_edge((5, 0)) && !on_diagonal_edge((5, 0))); // x-axis
        assert!(!on_x_axis_edge((5, 5)) && on_diagonal_edge((5, 5))); // diagonal
        assert!(!on_x_axis_edge((5, 2)) && !on_diagonal_edge((5, 2))); // interior
    }

    /// `edge_reach_lb` is a verbatim port of the legacy joint bound; anchor a
    /// few hand values so any future drift from `legacy` is caught.
    #[test]
    fn edge_reach_lb_hand_values() {
        assert_eq!(edge_reach_lb(1, 1, 3, 4), 0, "both edges touched ⇒ 0");
        assert_eq!(edge_reach_lb(1, 0, 0, 3), 8 * 3 - 4, "x done, gap=3 ⇒ 20");
        assert_eq!(edge_reach_lb(0, 1, 2, 0), 4 * 2, "diag done, min_y=2 ⇒ 8");
        // both pending: max(4·min_y, 8·min_gap−4) = max(8, 20) = 20
        assert_eq!(edge_reach_lb(0, 0, 2, 3), 20);
    }

    /// `max_scan_d` is monotone, nonnegative, and covers every n≤9 witness
    /// slice's actual max anti-diagonal (DESIGN §3.4 hand cases).
    #[test]
    fn max_scan_d_safe_and_monotone() {
        let mut last = -1;
        for n in 0..=400 {
            let m = max_scan_d(n);
            assert!(m >= 0);
            assert!(m >= last, "max_scan_d not monotone at n={n}");
            last = m;
        }
        // (n, max d of the §3.4 witness slice) — must not be excluded.
        for &(n, witness_max_d) in &[(1, 0), (4, 0), (5, 1), (8, 2), (9, 2)] {
            assert!(
                max_scan_d(n) >= witness_max_d,
                "max_scan_d({n})={} < witness {witness_max_d}",
                max_scan_d(n)
            );
        }
    }

    /// The full b-file frontier fits the `u64` occupancy mask (DP invariant).
    #[test]
    fn frontier_fits_u64_through_bfile() {
        for n in (0..=163).filter(|n| n % 4 == 0 || n % 4 == 1) {
            assert!(frontier_fits_u64(n), "frontier > 64 slots at n={n}");
        }
        // headroom check: even the loose bound stays well under 64.
        assert!(max_slots(163) <= 64);
    }
}

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

use crate::symmetry::{orbit_size, Cell, Center};

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

// ── Phase 2: occupancy + weight DP (no connectivity) ────────────────────────
//
// A pure `(weight, x-touched, diag-touched)` knapsack over the anti-diagonal
// scan: count subsets `S` of the bounded wedge (`d ≤ max_scan_d(n)`) with
// `Σ orbit_size(center,c) == n` that touch the x-axis edge *and* the diagonal
// edge — §4.1 condition 2 only, **without** condition 1 (a single 4-connected
// component). This *over-counts* `a(n)` (it admits disconnected and
// multi-component sets); Phase 3 layers the Jensen connectivity signature on
// top to recover exactly `legacy::enumerate(center, n)`.
//
// It is finite only because the scan bound is the *valid-slice* geometry
// bound (a single weight-4 edge cell can sit arbitrarily far out, so the
// unbounded no-connectivity count is infinite) — so this figure is a
// machinery scaffold, not a combinatorial quantity on its own. It exercises
// the weight accounting, the residue/apex dispatch (§3.3 / §4.6(a)) and the
// edge-flag logic exactly as Phase 3 will, on a state space small enough to
// brute-check.

/// Phase-2 over-count (see module note above). `n > 0`; cell requires
/// `n % 4 ∈ {0,1}`, vertex requires `4 | n` (caller-enforced, as in §3.3).
pub(crate) fn count_phase2(center: Center, n: usize) -> u64 {
    debug_assert!(n > 0, "n=0 is the count(0)=1 convention, handled upstream");
    // Residue/center dispatch — identical to §3.3 / §4.6(a) and to the
    // `legacy` engine, so Phase 3 inherits it unchanged.
    let apex_forced = center == Center::Cell && n % 4 == 1; // (0,0) ∈ S
    let apex_forbidden = center == Center::Cell && n % 4 == 0; // (0,0) ∉ S

    let nn = n as u32;
    // dp indexed by (weight, x-touched, diag-touched).
    let idx = |w: u32, tx: bool, td: bool| (w as usize) * 4 + (tx as usize) * 2 + td as usize;
    let mut dp = vec![0u64; (n + 1) * 4];
    if apex_forced {
        dp[idx(1, true, true)] = 1; // apex: weight 1, on both edges, never branched
    } else {
        dp[idx(0, false, false)] = 1; // empty config
    }

    let mut next = vec![0u64; (n + 1) * 4];
    for (_, _, c) in WedgeScan::new(max_scan_d(n)) {
        if c == (0, 0) && (apex_forced || apex_forbidden) {
            continue; // apex pre-decided (forced in / forbidden out)
        }
        let w = orbit_size(center, c) as u32;
        let ex = on_x_axis_edge(c);
        let ed = on_diagonal_edge(c);
        next.iter_mut().for_each(|s| *s = 0);
        for ww in 0..=nn {
            for tx in [false, true] {
                for td in [false, true] {
                    let cnt = dp[idx(ww, tx, td)];
                    if cnt == 0 {
                        continue;
                    }
                    next[idx(ww, tx, td)] += cnt; // c empty
                    if ww + w <= nn {
                        next[idx(ww + w, tx | ex, td | ed)] += cnt; // c occupied
                    }
                }
            }
        }
        std::mem::swap(&mut dp, &mut next);
    }
    dp[idx(nn, true, true)]
}

// ── Phase 3: connectivity-signature transfer matrix ─────────────────────────
//
// The Jensen device. Sweeping anti-diagonals, a 4-step changes `d=x+y` by ±1,
// so no two cells on the same anti-diagonal are 4-adjacent and every cell's
// only already-decided neighbours are the two cells of anti-diagonal `d−1` at
// slots `y` (cell `(x−1,y)`, in-wedge iff `x>y`) and `y−1` (cell `(x,y−1)`,
// in-wedge iff `y≥1`). So connected components are built incrementally by
// union over backward links; the frontier state is the just-completed
// anti-diagonal's occupancy + a canonical (non-crossing) component partition +
// per-component "touched x-axis / diagonal" bits + accumulated weight.
//
// A component **retires** when no cell of the new anti-diagonal links to it
// (it has no frontier cell, so nothing can ever connect to it again). §4.1
// condition 1 (exactly one 4-connected component) ⇒ the *only* legal
// retirement is the sole remaining component finishing with nothing left on
// the frontier; any other retirement severs a piece ⇒ the branch is dead.
// Distinct partial slices with the same frontier state share all futures, so
// their multiplicities sum — the collapse that makes this output-sized.

const EMPTY: u8 = u8::MAX;

/// A frontier state after a completed anti-diagonal. `occ[y]` = `EMPTY` or the
/// canonical component id of the occupied cell at slot `y`; `cx`/`cd` =
/// per-component "has an x-axis-edge / diagonal-edge cell"; `w` = total orbit
/// weight so far. Canonical labels (ids assigned by first occupied slot in
/// `y` order) make equivalent partitions collapse to one key.
#[derive(Clone, PartialEq, Eq, Hash)]
struct Frontier {
    occ: Vec<u8>,
    cx: Vec<bool>,
    cd: Vec<bool>,
    w: u32,
}

/// Union–find over `old-component nodes (0..ncomp)` ∪ `occupied-slot nodes`.
struct Uf {
    p: Vec<usize>,
}
impl Uf {
    fn new(n: usize) -> Self {
        Uf {
            p: (0..n).collect(),
        }
    }
    fn find(&mut self, mut a: usize) -> usize {
        while self.p[a] != a {
            self.p[a] = self.p[self.p[a]];
            a = self.p[a];
        }
        a
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.p[ra] = rb;
        }
    }
}

/// Debug-only: a canonical frontier partition over a planar staircase must be
/// **non-crossing** — no `A … B … A … B` over occupied slots (`A≠B`). If this
/// ever fires the scan/adjacency model is wrong (and the Catalan state bound
/// with it).
#[cfg(debug_assertions)]
fn assert_non_crossing(occ: &[u8]) {
    let occ_slots: Vec<u8> = occ.iter().copied().filter(|&l| l != EMPTY).collect();
    for i in 0..occ_slots.len() {
        for j in (i + 1)..occ_slots.len() {
            if occ_slots[i] == occ_slots[j] {
                continue;
            }
            for k in (j + 1)..occ_slots.len() {
                if occ_slots[k] != occ_slots[i] {
                    continue;
                }
                for m in (k + 1)..occ_slots.len() {
                    debug_assert!(
                        occ_slots[m] != occ_slots[j],
                        "crossing partition {occ_slots:?} — scan model violated"
                    );
                }
            }
        }
    }
}

/// `legacy::enumerate(center, n)` recomputed by the connectivity-signature
/// transfer matrix (DESIGN §4.7). `n > 0`; cell needs `n%4 ∈ {0,1}`, vertex
/// needs `4|n` — caller-enforced (`count_transfer`), as in §3.3.
pub(crate) fn count_phase3(center: Center, n: usize) -> u64 {
    debug_assert!(n > 0);
    let nn = n as u32;
    let apex_forced = center == Center::Cell && n % 4 == 1;
    let apex_forbidden = center == Center::Cell && n % 4 == 0;
    let max_d = max_scan_d(n);

    let mut map: std::collections::HashMap<Frontier, u64> = std::collections::HashMap::new();
    map.insert(
        Frontier {
            occ: vec![],
            cx: vec![],
            cd: vec![],
            w: 0,
        },
        1,
    );
    let mut accepted: u64 = 0;

    for d in 0..=max_d {
        let len_d = antidiag_len(d);
        let mut next: std::collections::HashMap<Frontier, u64> = std::collections::HashMap::new();

        for (prev, &mult) in &map {
            // Recurse over slot occupancy for anti-diagonal d, pruning on
            // weight; finalize each full assignment.
            let mut cur = vec![false; len_d];
            // Stack-free explicit recursion via a helper closure is awkward in
            // Rust; use an inner recursive fn over an index.
            #[allow(clippy::too_many_arguments)]
            fn rec(
                y: usize,
                len_d: usize,
                d: i32,
                center: Center,
                nn: u32,
                apex_forced: bool,
                apex_forbidden: bool,
                cur: &mut Vec<bool>,
                run_w: u32,
                prev: &Frontier,
                mult: u64,
                accepted: &mut u64,
                next: &mut std::collections::HashMap<Frontier, u64>,
            ) {
                if run_w > nn {
                    return; // weight monotone ⇒ subtree dead
                }
                if y == len_d {
                    finalize(d, len_d, center, nn, cur, prev, mult, accepted, next);
                    return;
                }
                let cell = cell_at(d, y as i32);
                let is_apex = cell == (0, 0);
                let w_cell = orbit_size(center, cell) as u32;

                // empty branch (skip if the apex is forced occupied)
                if !(is_apex && apex_forced) {
                    cur[y] = false;
                    rec(
                        y + 1, len_d, d, center, nn, apex_forced, apex_forbidden, cur, run_w,
                        prev, mult, accepted, next,
                    );
                }
                // occupied branch (skip if the apex is forbidden)
                if !(is_apex && apex_forbidden) {
                    cur[y] = true;
                    rec(
                        y + 1,
                        len_d,
                        d,
                        center,
                        nn,
                        apex_forced,
                        apex_forbidden,
                        cur,
                        run_w + w_cell,
                        prev,
                        mult,
                        accepted,
                        next,
                    );
                    cur[y] = false;
                }
            }

            rec(
                0,
                len_d,
                d,
                center,
                nn,
                apex_forced,
                apex_forbidden,
                &mut cur,
                prev.w,
                prev,
                mult,
                &mut accepted,
                &mut next,
            );
        }
        map = next;
    }

    // Scan end: a surviving state is a valid slice iff exactly one component,
    // weight n, both edges. (Earlier-retiring sole components were already
    // resolved and removed.)
    for (st, &mult) in &map {
        if st.cx.len() == 1 && st.w == nn && st.cx[0] && st.cd[0] {
            accepted += mult;
        }
    }
    accepted
}

/// Process one full anti-diagonal occupancy `cur`: compute components from the
/// backward links into `prev`, apply the retirement rule (accept the sole
/// completing component, or drop a severed branch), else emit the canonical
/// next frontier with `mult`.
#[allow(clippy::too_many_arguments)]
fn finalize(
    d: i32,
    len_d: usize,
    center: Center,
    nn: u32,
    cur: &[bool],
    prev: &Frontier,
    mult: u64,
    accepted: &mut u64,
    next: &mut std::collections::HashMap<Frontier, u64>,
) {
    let mut w = prev.w;
    for (y, &on) in cur.iter().enumerate() {
        if on {
            w += orbit_size(center, cell_at(d, y as i32)) as u32;
        }
    }
    if w > nn {
        return;
    }
    let ncomp = prev.cx.len();
    let occ_slots: Vec<usize> = (0..len_d).filter(|&y| cur[y]).collect();
    let mut uf = Uf::new(ncomp + occ_slots.len());
    let mut ref_old = vec![false; ncomp];

    for (j, &y) in occ_slots.iter().enumerate() {
        let x = d - y as i32;
        let slot_node = ncomp + j;
        // backward A = (x−1, y) on d−1 slot y, in-wedge iff x>y
        if x > y as i32 && y < prev.occ.len() && prev.occ[y] != EMPTY {
            let o = prev.occ[y] as usize;
            uf.union(o, slot_node);
            ref_old[o] = true;
        }
        // backward B = (x, y−1) on d−1 slot y−1, in-wedge iff y≥1
        if y >= 1 && (y - 1) < prev.occ.len() && prev.occ[y - 1] != EMPTY {
            let o = prev.occ[y - 1] as usize;
            uf.union(o, slot_node);
            ref_old[o] = true;
        }
    }

    let retired_any = (0..ncomp).any(|o| !ref_old[o]);
    if retired_any {
        // Legal sole completion ⟺ exactly one old component and nothing on d
        // (so it is unreferenced and no new component is started).
        if ncomp == 1 && occ_slots.is_empty() {
            if prev.w == nn && prev.cx[0] && prev.cd[0] {
                *accepted += mult;
            }
            return; // resolved
        }
        return; // a piece is severed ⇒ ≥2 components ⇒ dead
    }

    // All old components continue. Build the canonical next frontier.
    let mut root_to_canon: std::collections::HashMap<usize, u8> = std::collections::HashMap::new();
    let mut cx: Vec<bool> = Vec::new();
    let mut cd: Vec<bool> = Vec::new();
    let mut occ_lbl = vec![EMPTY; len_d];
    for (j, &y) in occ_slots.iter().enumerate() {
        let r = uf.find(ncomp + j);
        let canon = *root_to_canon.entry(r).or_insert_with(|| {
            let id = cx.len() as u8;
            cx.push(false);
            cd.push(false);
            id
        });
        occ_lbl[y] = canon;
    }
    for o in 0..ncomp {
        let r = uf.find(o);
        if let Some(&canon) = root_to_canon.get(&r) {
            cx[canon as usize] |= prev.cx[o];
            cd[canon as usize] |= prev.cd[o];
        }
    }
    for (j, &y) in occ_slots.iter().enumerate() {
        let r = uf.find(ncomp + j);
        let canon = root_to_canon[&r] as usize;
        let cell = cell_at(d, y as i32);
        cx[canon] |= on_x_axis_edge(cell);
        cd[canon] |= on_diagonal_edge(cell);
    }
    #[cfg(debug_assertions)]
    assert_non_crossing(&occ_lbl);

    *next
        .entry(Frontier {
            occ: occ_lbl,
            cx,
            cd,
            w,
        })
        .or_insert(0) += mult;
}

/// `a(n)` for A142886 via the transfer matrix — same public contract and
/// residue/center dispatch as `legacy::count` (§3.3); the GO swap repoints
/// `mod.rs`'s `pub use` here.
pub(crate) fn count(n: usize) -> crate::Count {
    if n == 0 {
        return 1;
    }
    if n % 4 == 2 || n % 4 == 3 {
        return 0;
    }
    count_phase3(Center::Cell, n) + count_vertex_centered(n)
}

/// Cell-centered contribution (≈ A351127); see `legacy::count_cell_centered`.
pub(crate) fn count_cell_centered(n: usize) -> crate::Count {
    if n == 0 {
        return 1;
    }
    if n % 4 == 2 || n % 4 == 3 {
        return 0;
    }
    count_phase3(Center::Cell, n)
}

/// Vertex-centered contribution (≈ A346800(n/4)); nonzero only when `4|n`.
pub(crate) fn count_vertex_centered(n: usize) -> crate::Count {
    if n == 0 || n % 4 != 0 {
        return 0;
    }
    count_phase3(Center::Vertex, n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enumerate::legacy;
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

    /// Brute subset count over the **same** bounded region and apex rule as
    /// `count_phase2` — an exact oracle for the knapsack/flag/residue
    /// machinery, independent of connectivity (which Phase 2 omits).
    fn brute_phase2(center: Center, n: usize) -> u64 {
        let apex_forced = center == Center::Cell && n % 4 == 1;
        let apex_forbidden = center == Center::Cell && n % 4 == 0;
        let mut cells: Vec<Cell> = Vec::new();
        for (_, _, c) in WedgeScan::new(max_scan_d(n)) {
            if c == (0, 0) && (apex_forced || apex_forbidden) {
                continue;
            }
            cells.push(c);
        }
        assert!(cells.len() <= 20, "brute infeasible: {} cells, n={n}", cells.len());
        let (bw, btx, btd) = if apex_forced {
            (1u32, true, true)
        } else {
            (0u32, false, false)
        };
        let mut total = 0u64;
        for mask in 0u32..(1u32 << cells.len()) {
            let (mut w, mut tx, mut td) = (bw, btx, btd);
            for (b, &c) in cells.iter().enumerate() {
                if mask & (1 << b) != 0 {
                    w += orbit_size(center, c) as u32;
                    tx |= on_x_axis_edge(c);
                    td |= on_diagonal_edge(c);
                    if w > n as u32 {
                        break;
                    }
                }
            }
            if w == n as u32 && tx && td {
                total += 1;
            }
        }
        total
    }

    /// Phase 2 DP == brute subset count on every residue/center it dispatches,
    /// for `n` small enough to brute-enumerate the bounded region.
    #[test]
    fn phase2_matches_brute_subset_machinery() {
        for &n in &[1usize, 5, 9] {
            // cell, n ≡ 1 mod 4 (apex forced)
            assert_eq!(
                count_phase2(Center::Cell, n),
                brute_phase2(Center::Cell, n),
                "cell n={n}"
            );
        }
        for &n in &[4usize, 8] {
            // n ≡ 0 mod 4: cell (apex forbidden) + vertex
            assert_eq!(
                count_phase2(Center::Cell, n),
                brute_phase2(Center::Cell, n),
                "cell n={n}"
            );
            assert_eq!(
                count_phase2(Center::Vertex, n),
                brute_phase2(Center::Vertex, n),
                "vertex n={n}"
            );
        }
    }

    /// Connectivity-free Phase 2 must *over-count* the proven `legacy`
    /// engine per center: every valid (connected, both-edge) slice fits the
    /// scan bound (proven in `max_scan_d`), so it is among the subsets
    /// Phase 2 counts.
    #[test]
    fn phase2_overcounts_legacy_per_center() {
        for n in 1usize..=40 {
            match n % 4 {
                0 => {
                    assert!(
                        count_phase2(Center::Cell, n) >= legacy::count_cell_centered(n),
                        "cell n={n}"
                    );
                    assert!(
                        count_phase2(Center::Vertex, n) >= legacy::count_vertex_centered(n),
                        "vertex n={n}"
                    );
                }
                1 => assert!(
                    count_phase2(Center::Cell, n) >= legacy::count_cell_centered(n),
                    "cell n={n}"
                ),
                _ => {}
            }
        }
    }

    /// Phase 3 connectivity DP must be **byte-identical** to the proven
    /// `legacy` engine — per center *and* total — for every feasible n in a
    /// small range (the strong oracle; the deep sweep is Phase 4).
    #[test]
    fn phase3_byte_identical_to_legacy_small_n() {
        for n in 0usize..=24 {
            assert_eq!(
                super::count(n),
                legacy::count(n),
                "count mismatch at n={n}"
            );
            assert_eq!(
                count_cell_centered(n),
                legacy::count_cell_centered(n),
                "cell mismatch at n={n}"
            );
            assert_eq!(
                count_vertex_centered(n),
                legacy::count_vertex_centered(n),
                "vertex mismatch at n={n}"
            );
        }
    }

    /// The named §3.4 hand cases, through the transfer engine directly.
    #[test]
    fn phase3_named_shapes() {
        assert_eq!(super::count(0), 1); // empty (convention)
        assert_eq!(super::count(1), 1); // monomino
        assert_eq!(super::count(4), 1); // 2×2 (vertex)
        assert_eq!(super::count(5), 1); // X-pentomino
        assert_eq!(super::count(8), 1); // 3×3 ring
        assert_eq!(super::count(9), 2); // 3×3 solid + arm-2 plus
        assert_eq!(count_vertex_centered(4), 1);
        assert_eq!(count_cell_centered(4), 0);
        assert_eq!(count_cell_centered(9), 2);
    }
}

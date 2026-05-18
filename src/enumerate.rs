//! Redelmeier-style recursive enumeration restricted to the D8 fundamental
//! wedge, per symmetry-center case.
//!
//! Milestone **M3** (PLAN.md). DESIGN.md §3, §4.2, §4.4.
//!
//! A D8-symmetric polyomino of `n` cells is in bijection with its wedge slice
//! `S ⊆ W` (DESIGN.md §2). By the §4.1 lemma the polyomino is connected iff
//! `S` is one 4-connected component touching both wedge edges (`y = 0` and
//! `x = y`); Redelmeier growth keeps `S` connected by construction, so only
//! the edge-touch check remains. We bucket the valid slices by their minimal
//! x-axis cell `A = (ax, 0)` and Redelmeier-grow each bucket from the pinned
//! root `A` under the blocked-set discipline, forbidding any x-axis cell
//! strictly left of `A`. Injectivity is the standard fixed-root blocked-set
//! theorem — the baseline's global lex-min `nb > seed` shortcut is
//! deliberately dropped (it has no cheap analog for an edge-pinned root; the
//! known performance trade — see the plan's O2 note). The reconstructed size
//! is `Σ_{c∈S} orbit_size(c)` (orbits are disjoint).

use crate::symmetry::{orbit_size, Cell, Center};
use crate::Count;

/// Dense bitset over the bounded wedge cells `(x, y)` with `0 ≤ y ≤ x ≤ xmax`
/// (DESIGN.md §2; the recursion never produces a cell outside this triangle).
///
/// Replaces the per-`grow` `HashSet<Cell>` used for the slice (`p`) and the
/// Redelmeier excluded set (`blocked`). Cells are bounded integers, so a flat
/// bit-array indexed by `x·stride + y` gives O(1) insert/contains/remove with
/// no hashing and a single allocation per `enumerate` call (reused, not
/// re-grown, down the whole recursion). Profiling attributed ~63% of run time
/// to SipHash on these two sets; this removes that path. The recursion
/// structure — and therefore every count — is unchanged.
struct CellSet {
    words: Vec<u64>,
    stride: usize,
}

impl CellSet {
    /// Allocate a cleared set covering `0 ≤ x, y ≤ xmax`.
    fn new(xmax: i32) -> Self {
        let stride = xmax as usize + 1;
        let bits = stride * stride;
        CellSet {
            words: vec![0u64; bits.div_ceil(64)],
            stride,
        }
    }

    /// Flat bit index of `c`. Caller guarantees `0 ≤ c.1 ≤ c.0 ≤ xmax`
    /// (every call site checks `in_wedge` and `≤ xmax` first).
    #[inline]
    fn index(&self, c: Cell) -> usize {
        c.0 as usize * self.stride + c.1 as usize
    }

    #[inline]
    fn contains(&self, c: Cell) -> bool {
        let i = self.index(c);
        self.words[i / 64] & (1u64 << (i % 64)) != 0
    }

    /// Set the bit; return `true` iff it was newly set (matches the
    /// `HashSet::insert` contract the `blocked` bookkeeping relies on).
    #[inline]
    fn insert(&mut self, c: Cell) -> bool {
        let i = self.index(c);
        let w = &mut self.words[i / 64];
        let mask = 1u64 << (i % 64);
        let was_set = *w & mask != 0;
        *w |= mask;
        !was_set
    }

    #[inline]
    fn remove(&mut self, c: Cell) {
        let i = self.index(c);
        self.words[i / 64] &= !(1u64 << (i % 64));
    }
}

#[inline]
fn in_wedge(c: Cell) -> bool {
    c.0 >= 0 && c.1 >= 0 && c.1 <= c.0
}

/// Admissible lower bound on the *extra* orbit-weight any completion still
/// needs in order to satisfy §4.1 condition 2 (touch both wedge edges).
/// `min_y` = smallest `y`; `min_gap` = smallest `x − y` over the slice.
///
/// **x-axis term** (moot for the shipped A-rooted scheme — the pinned root
/// is on the x-axis so `tx > 0` always ⇒ 0; kept for the general lemma): if
/// `y = 0` is not yet touched, reaching it needs ≥ `min_y` new cells, ≥ 4
/// weight each.
///
/// **Diagonal term — single joint cell-budget + gap bound.** If the
/// diagonal is not yet touched (`td == 0` ⇔ `min_gap ≥ 1`), reaching it
/// needs `min_gap` new cells (each 4-neighbour step changes `x − y` by ≤ 1,
/// so the chain hits every gap value `min_gap−1, …, 1, 0`). The cheap
/// weight-4 route — run along the x-axis to the origin — is **blocked by the
/// forbidden region**: every x-axis cell has gap `x ≥ ax ≥ min_gap` (or is
/// forbidden, `x < ax`), so it can never *reduce* the gap. Hence each of the
/// `min_gap − 1` gap-reducing cells is interior (orbit weight 8, not 4) and
/// only the gap-0 landing is a weight-4 diagonal cell:
///
/// ```text
/// extra_weight ≥ 8·(min_gap − 1) + 4 = 8·min_gap − 4    (td == 0)
/// ```
///
/// Exact (tightest admissible) for both centers (vertex: non-diagonal ⇒ 8,
/// diagonal ⇒ 4; cell-centered: the only weight-1 cell `(0,0)` is
/// x-axis-forbidden whenever this term is active, `ax ≥ 1`). For `ax = 0`
/// buckets (cell `n ≡ 1` apex / vertex 2×2 core) the root is on the diagonal
/// ⇒ `td > 0` ⇒ this term is 0.
///
/// The two excursions may share cells, so the sound combined bound is `max`,
/// never the sum. **Soundness is per-node admissibility, not monotonicity:**
/// `weight + edge_reach_lb` is *not* monotone down the recursion (the
/// diagonal term can fall by 8 while `weight` rises only 4), but if
/// `weight + lb > n` at a node then every completion's total
/// `= weight + added ≥ weight + lb > n` (a valid slice must touch the
/// diagonal), so the subtree holds no weight-`n` slice — independent of
/// descendants. (DESIGN §4.6(b).)
#[inline]
fn edge_reach_lb(tx: u32, td: u32, min_y: i32, min_gap: i32) -> u64 {
    // `td == 0` ⇒ no gap-0 cell ⇒ `min_gap ≥ 1`, so `8·min_gap − 4 ≥ 4`
    // and the `u64` subtraction cannot underflow.
    debug_assert!(td > 0 || min_gap >= 1, "td == 0 must imply min_gap ≥ 1");
    let wx = if tx > 0 { 0 } else { 4 * min_y as u64 };
    let wd = if td > 0 { 0 } else { 8 * (min_gap as u64) - 4 };
    wx.max(wd)
}

#[inline]
fn on_x_axis_edge(c: Cell) -> bool {
    c.1 == 0
}

#[inline]
fn on_diagonal_edge(c: Cell) -> bool {
    c.0 == c.1
}

/// Is `c` an x-axis cell strictly left of the pinned root `A = (ax, 0)`?
/// Banning it makes `A` the slice's canonical minimal x-axis cell, so the
/// buckets partition the valid slices (this replaces the baseline's
/// `nb > seed` lex-min canonicalisation; injectivity itself comes from the
/// blocked-set discipline, which is unaffected — a forbidden cell is never
/// enqueued, hence never blocked or unwound).
#[inline]
fn forbidden(c: Cell, ax: i32) -> bool {
    c.1 == 0 && c.0 < ax
}

const NEIGHBOURS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Count D8-symmetric polyominoes of `n` cells for one center type, by
/// enumerating connected wedge slices of total orbit-weight `n`.
///
/// Returns 0 for `n == 0` (the empty polyomino is a caller-side convention).
fn enumerate(center: Center, n: usize) -> Count {
    if n == 0 {
        return 0;
    }
    let n = n as u64;
    // Any cell of a weight-≤n connected wedge slice has coordinate ≤ n
    // (a slice reaching column X within W needs ≥ X cells, each weight ≥ 1).
    let xmax = n as i32;
    let mut total: Count = 0;

    // Residue-based bucket restriction (DESIGN.md §3.1). A cell-centered
    // slice has weight `c + 4e + 8i` with `c ∈ {0,1}` the apex bit, so its
    // weight is ≡ 1 (mod 4) iff it contains the apex `(0,0)` and ≡ 0
    // otherwise. Hence for the target residue, all slices of the *other*
    // parity have weight that can never equal `n` — their entire Redelmeier
    // subtree is provably empty. `(0,0)` is the only `x = 0` wedge cell, so a
    // slice contains the apex iff its minimal x-axis cell is `(0,0)`, i.e.
    // iff `ax == 0`; skip the dead buckets outright:
    //   n ≡ 1 (mod 4): center occupied — only the apex bucket produces n.
    //   n ≡ 0 (mod 4): center empty (the "ring") — the apex bucket produces
    //                  only n ≡ 1, so skip it.
    // (Vertex-centered has no apex; `n` is always ≡ 0 and every bucket live.)
    let apex_required = center == Center::Cell && n % 4 == 1;
    let apex_forbidden = center == Center::Cell && n % 4 == 0;

    // Partition the valid slices by their minimal x-axis cell `A = (ax, 0)`.
    // Every valid slice touches the x-axis (§4.1), so `A` is well-defined;
    // Redelmeier-grow each bucket from the pinned root `A` under the
    // blocked-set discipline, forbidding any x-axis cell strictly left of
    // `A` (so `A` is the unique canonical representative ⇒ each slice is
    // generated in exactly one bucket, counted exactly once). The variable
    // is still named `seed` (== `(ax, 0)`) so the rest of the recursion is
    // unchanged.
    for ax in 0..=xmax {
        let seed = (ax, 0);
        let is_apex = center == Center::Cell && ax == 0;
        if apex_required && !is_apex {
            continue; // dead parity: weight ≡ 0 (mod 4) ≠ n
        }
        if apex_forbidden && is_apex {
            continue; // dead parity: weight ≡ 1 (mod 4) ≠ n
        }
        let ws = orbit_size(center, seed) as u64;
        if ws > n {
            continue;
        }
        let tx = u32::from(on_x_axis_edge(seed));
        let td = u32::from(on_diagonal_edge(seed));
        if ws == n {
            if tx > 0 && td > 0 {
                total += 1;
            }
            continue; // any extension exceeds n
        }
        // Edge-reachability prune (the "ring" case): if even the cheapest
        // completion cannot touch both wedge edges within the budget, this
        // bucket's whole subtree is dead (§4.1 condition 2). `A` is on the
        // x-axis so only the diagonal term (min_gap = ax) can bind.
        if ws + edge_reach_lb(tx, td, seed.1, seed.0 - seed.1) > n {
            continue;
        }
        // All four scratch structures are allocated once per bucket and
        // reused across the bucket's entire recursion subtree
        // (push/truncate, never a per-node `Vec`): `p` = slice membership,
        // `blocked` = Redelmeier exclusion set, `untried` = the shared
        // growth buffer, `in_untried` = O(1) membership of `untried`,
        // `blk_unwind` = the stack recording which cells each level added to
        // `blocked`.
        let mut p = CellSet::new(xmax);
        p.insert(seed);
        let mut blocked = CellSet::new(xmax);
        let mut untried: Vec<Cell> = Vec::new();
        let mut in_untried = CellSet::new(xmax);
        let mut blk_unwind: Vec<Cell> = Vec::new();
        for (dx, dy) in NEIGHBOURS {
            let nb = (seed.0 + dx, seed.1 + dy);
            if in_wedge(nb) && nb.0 <= xmax && !forbidden(nb, seed.0) && in_untried.insert(nb) {
                untried.push(nb);
            }
        }
        grow(
            center,
            n,
            seed,
            xmax,
            &mut p,
            ws,
            tx,
            td,
            seed.1,
            seed.0 - seed.1,
            0,
            &mut untried,
            &mut in_untried,
            &mut blocked,
            &mut blk_unwind,
            &mut total,
        );
    }
    total
}

/// Redelmeier growth over a **shared** frontier buffer.
///
/// `untried` is one buffer reused down the whole recursion. This level owns
/// the slice `untried[lo..hi]`, where `hi` is `untried.len()` on entry (no
/// sibling has appended yet). Branch `pos` enumerates slices whose next
/// included cell is `untried[pos]`; the still-pending suffix `untried[pos+1..]`
/// plus that cell's fresh neighbours form the child frontier — appended onto
/// the same buffer past `hi`, then truncated back to `hi` once the branch
/// returns, so siblings see exactly the original suffix. This replaces the
/// per-node `untried[i+1..].to_vec()` (the dominant allocation after
/// optimization #1) with push/truncate on a buffer whose capacity stabilizes
/// after the first deep path.
///
/// `in_untried` mirrors the buffer's live membership, making the uniqueness
/// check O(1) instead of a `Vec::contains` scan: a candidate already filtered
/// against `p` and `blocked` is in `in_untried` iff it is in the live suffix
/// (every earlier cell is in `p` or `blocked`, hence rejected already), so the
/// dedup is exactly the original `!child.contains(..)`.
///
/// `untried[pos]` is added to `blocked` after its branch — excluding it for
/// later siblings and their subtrees — and recorded on the shared `blk_unwind`
/// stack so this level can restore precisely its own additions on exit.
#[allow(clippy::too_many_arguments)]
fn grow(
    center: Center,
    n: u64,
    seed: Cell,
    xmax: i32,
    p: &mut CellSet,
    weight: u64,
    tx: u32,
    td: u32,
    min_y: i32,
    min_gap: i32,
    lo: usize,
    untried: &mut Vec<Cell>,
    in_untried: &mut CellSet,
    blocked: &mut CellSet,
    blk_unwind: &mut Vec<Cell>,
    total: &mut Count,
) {
    // Edge-reachability prune (§4.1 condition 2): no descendant of this node
    // can ever touch both wedge edges within the remaining budget. Sound
    // because `weight + edge_reach_lb(..)` only grows down the recursion.
    if weight + edge_reach_lb(tx, td, min_y, min_gap) > n {
        return;
    }
    let hi = untried.len(); // this level's frontier is untried[lo..hi]
    let blk_base = blk_unwind.len();
    for pos in lo..hi {
        let c = untried[pos];
        let w2 = weight + orbit_size(center, c) as u64;
        if w2 <= n {
            let ntx = tx + u32::from(on_x_axis_edge(c));
            let ntd = td + u32::from(on_diagonal_edge(c));
            p.insert(c);
            if w2 == n {
                // §4.1: connected by construction; needs both wedge edges.
                if ntx > 0 && ntd > 0 {
                    *total += 1;
                }
                // cannot extend further (weight would exceed n)
            } else {
                // Every prior branch truncated back to `hi`, so the buffer
                // ends exactly at this level's suffix here.
                debug_assert_eq!(untried.len(), hi);
                // Append c's fresh neighbours after the suffix (not in the
                // slice, not blocked, not already queued).
                for (dx, dy) in NEIGHBOURS {
                    let nb = (c.0 + dx, c.1 + dy);
                    if in_wedge(nb)
                        && nb.0 <= xmax
                        && !forbidden(nb, seed.0)
                        && !p.contains(nb)
                        && !blocked.contains(nb)
                        && in_untried.insert(nb)
                    {
                        untried.push(nb);
                    }
                }
                grow(
                    center,
                    n,
                    seed,
                    xmax,
                    p,
                    w2,
                    ntx,
                    ntd,
                    min_y.min(c.1),
                    min_gap.min(c.0 - c.1),
                    pos + 1,
                    untried,
                    in_untried,
                    blocked,
                    blk_unwind,
                    total,
                );
                // Drop this branch's appended neighbours, keeping
                // `in_untried` in lock-step, so siblings see only the suffix.
                while untried.len() > hi {
                    let nb = untried.pop().unwrap();
                    in_untried.remove(nb);
                }
            }
            p.remove(c);
        }
        // c is now excluded for the remaining branches at this level and
        // everything below them.
        if blocked.insert(c) {
            blk_unwind.push(c);
        }
    }
    while blk_unwind.len() > blk_base {
        let c = blk_unwind.pop().unwrap();
        blocked.remove(c);
    }
}

/// `a(n)` for OEIS A142886: the number of polyominoes with `n` cells whose
/// symmetry group is the full D8 group of the square.
///
/// `count(0) == 1` by OEIS convention; `0` for `n ≡ 2,3 (mod 4)`
/// (DESIGN.md §3.3); otherwise the sum of the two disjoint center cases.
pub fn count(n: usize) -> Count {
    if n == 0 {
        return 1;
    }
    if n % 4 == 2 || n % 4 == 3 {
        return 0;
    }
    count_cell_centered(n) + count_vertex_centered(n)
}

/// Contribution of polyominoes whose D8 center is a lattice **cell** center
/// (≈ OEIS A351127). DESIGN.md §3.1. The empty polyomino (`a(0)=1`) is
/// attributed to this case by convention.
pub fn count_cell_centered(n: usize) -> Count {
    if n == 0 {
        return 1;
    }
    if n % 4 == 2 || n % 4 == 3 {
        return 0;
    }
    enumerate(Center::Cell, n)
}

/// Contribution of polyominoes whose D8 center is a lattice **vertex**
/// (≈ OEIS A346800(n/4)). Nonzero only when `4 | n` (DESIGN.md §3.2/§3.3).
pub fn count_vertex_centered(n: usize) -> Count {
    if n == 0 || n % 4 != 0 {
        return 0;
    }
    enumerate(Center::Vertex, n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::REFERENCE;

    /// Test (b) — DESIGN.md §7(b): named small-shape witnesses, plus the
    /// §3.4 hand-derived cases.
    #[test]
    fn named_shapes_and_hand_cases() {
        assert_eq!(count(0), 1); // empty polyomino (convention)
        assert_eq!(count(1), 1); // monomino
        assert_eq!(count(4), 1); // 2x2 square (vertex-centered)
        assert_eq!(count(5), 1); // X / plus pentomino
        assert_eq!(count(8), 1); // 3x3 ring (3x3 minus center)
        assert_eq!(count(9), 2); // 3x3 solid is one of the two

        // §3.4 decompositions: which center produces each.
        assert_eq!(count_vertex_centered(4), 1); // the 2x2 square
        assert_eq!(count_cell_centered(4), 0); // none cell-centered at n=4
        assert_eq!(count_cell_centered(5), 1); // the plus
        assert_eq!(count_cell_centered(8), 1); // the 3x3 ring
        assert_eq!(count_vertex_centered(8), 0);
        assert_eq!(count_cell_centered(9), 2); // solid square + arm-2 plus
    }

    /// Test (c) — DESIGN.md §7(c): a(n) = 0 unless n ≡ 0 or 1 (mod 4).
    #[test]
    fn zero_unless_0_or_1_mod_4() {
        for n in 0..120 {
            if n % 4 == 2 || n % 4 == 3 {
                assert_eq!(count(n), 0, "a({n}) must be 0");
            }
        }
    }

    /// Test (d) — DESIGN.md §7(d): the cell/vertex split is consistent and
    /// the vertex term vanishes unless 4 | n. The heavy `n ≡ 0,1 (mod 4)`
    /// cases are bounded for baseline runtime (M3 measurement note); the
    /// cheap `n ≡ 2,3` cases cover the full range.
    #[test]
    fn split_formula() {
        const HEAVY_BOUND: usize = 40;
        for n in 0..120 {
            if n % 4 == 2 || n % 4 == 3 {
                assert_eq!(count(n), 0);
                assert_eq!(count_vertex_centered(n), 0);
                continue;
            }
            if n > HEAVY_BOUND {
                continue;
            }
            assert_eq!(
                count_cell_centered(n) + count_vertex_centered(n),
                count(n),
                "split mismatch at n={n}"
            );
            if n % 4 != 0 {
                assert_eq!(count_vertex_centered(n), 0, "vertex term at n={n}");
            }
        }
    }

    /// Test (a), always-on tier — DESIGN.md §7(a): fast OEIS prefix check
    /// (n = 0..=40). The full 0..=68 check is `matches_oeis_prefix_full`.
    #[test]
    fn matches_oeis_prefix_to_40() {
        for (n, &expected) in REFERENCE.iter().enumerate().take(41) {
            assert_eq!(count(n), expected, "a({n}) mismatch");
        }
    }

    /// Test (a), deep tier — DESIGN.md §7(a): full embedded prefix
    /// (n = 0..=68). `#[ignore]`d per the §7 baseline-runtime note; run with
    /// `cargo test --release -- --ignored`.
    #[test]
    #[ignore]
    fn matches_oeis_prefix_full() {
        for (n, &expected) in REFERENCE.iter().enumerate() {
            assert_eq!(count(n), expected, "a({n}) mismatch");
        }
    }
}

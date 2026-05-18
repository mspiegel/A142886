//! Redelmeier-style recursive enumeration restricted to the D8 fundamental
//! wedge, per symmetry-center case.
//!
//! Milestone **M3** (PLAN.md). DESIGN.md §3, §4.2, §4.4.
//!
//! A D8-symmetric polyomino of `n` cells is in bijection with its wedge slice
//! `S ⊆ W` (DESIGN.md §2). We enumerate connected wedge slices, keyed by
//! their minimum cell (a fixed `(x,y)` lexicographic order) so each slice is
//! generated exactly once (Redelmeier's discipline). The reconstructed size
//! is `Σ_{c∈S} orbit_size(c)` (orbits are disjoint), and by the §4.1 lemma
//! the polyomino is connected iff `S` is one 4-connected component touching
//! both wedge edges — Redelmeier growth keeps `S` connected by construction,
//! so only the edge-touch check remains.

use crate::symmetry::{orbit_size, Cell, Center};
use crate::Count;
use std::collections::HashSet;

#[inline]
fn in_wedge(c: Cell) -> bool {
    c.0 >= 0 && c.1 >= 0 && c.1 <= c.0
}

/// Admissible lower bound on the *extra* orbit-weight any completion still
/// needs in order to satisfy §4.1 condition 2 (touch both wedge edges).
///
/// `min_y` = the smallest `y` over the current slice; `min_gap` = the smallest
/// `x − y`. If the x-axis edge is not yet touched (`tx == 0`) a connected
/// addition reaching `y = 0` needs ≥ `min_y` new cells (one per unit step in
/// `−y`); likewise ≥ `min_gap` new cells to reach the diagonal `x = y`. The
/// two excursions may share cells, so the sound lower bound on *new cells* is
/// `max` (never the sum — `max` cannot over-estimate, so it never prunes a
/// branch that still admits a valid polyomino). Every new cell has orbit
/// weight ≥ 4, hence the `4 *` factor.
///
/// `weight + edge_reach_lb(..) ` is non-decreasing down the recursion (weight
/// rises ≥ 4 per added cell; `min_y`/`min_gap` fall ≤ 1), so a node that
/// exceeds `n` has every descendant exceed it — the whole subtree is prunable.
#[inline]
fn edge_reach_lb(tx: u32, td: u32, min_y: i32, min_gap: i32) -> u64 {
    let rx = if tx > 0 { 0 } else { min_y as u64 };
    let rd = if td > 0 { 0 } else { min_gap as u64 };
    4 * rx.max(rd)
}

#[inline]
fn on_x_axis_edge(c: Cell) -> bool {
    c.1 == 0
}

#[inline]
fn on_diagonal_edge(c: Cell) -> bool {
    c.0 == c.1
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

    // Residue-based seed restriction (DESIGN.md §3.1). A cell-centered slice
    // has weight `c + 4e + 8i` with `c ∈ {0,1}` the apex bit, so its weight
    // is ≡ 1 (mod 4) iff it contains the apex `(0,0)` and ≡ 0 otherwise.
    // Hence for the target residue, all slices of the *other* parity have
    // weight that can never equal `n` — their entire Redelmeier subtree is
    // provably empty. Since `(0,0)` is the global lex-minimum, "contains the
    // apex" ⟺ "seed is `(0,0)`", so we skip the dead seeds outright:
    //   n ≡ 1 (mod 4): center occupied — only the apex seed produces n.
    //   n ≡ 0 (mod 4): center empty (the "ring") — the apex seed produces
    //                  only n ≡ 1, so skip it.
    // (Vertex-centered has no apex; `n` is always ≡ 0 and every seed is live.)
    let apex_required = center == Center::Cell && n % 4 == 1;
    let apex_forbidden = center == Center::Cell && n % 4 == 0;

    // Iterate the slice's minimum cell in (x, y) lexicographic order; for
    // each, Redelmeier-grow using only strictly-greater wedge cells. Each
    // connected slice has a unique minimum, so it is counted exactly once.
    for sx in 0..=xmax {
        for sy in 0..=sx {
            let seed = (sx, sy);
            let is_apex = center == Center::Cell && seed == (0, 0);
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
            // Edge-reachability prune (the "ring" case): if even the
            // cheapest completion cannot touch both wedge edges within the
            // budget, this seed's whole subtree is dead (§4.1 condition 2).
            if ws + edge_reach_lb(tx, td, sy, sx - sy) > n {
                continue;
            }
            let mut p: HashSet<Cell> = HashSet::new();
            p.insert(seed);
            let mut blocked: HashSet<Cell> = HashSet::new();
            let mut untried: Vec<Cell> = Vec::new();
            for (dx, dy) in NEIGHBOURS {
                let nb = (seed.0 + dx, seed.1 + dy);
                if in_wedge(nb) && nb.0 <= xmax && nb > seed && !untried.contains(&nb) {
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
                sy,
                sx - sy,
                &untried,
                &mut blocked,
                &mut total,
            );
        }
    }
    total
}

/// Redelmeier growth from a fixed snapshot `untried` of the frontier.
///
/// Branch `i` enumerates slices whose next included cell is `untried[i]`;
/// `untried[0..i]` are excluded for that branch. To keep each slice unique we
/// also forbid those excluded cells (and every ancestor-excluded cell) from
/// re-entering the frontier deeper down — the `blocked` set, unwound when the
/// loop completes so sibling branches above can still use them.
#[allow(clippy::too_many_arguments)]
fn grow(
    center: Center,
    n: u64,
    seed: Cell,
    xmax: i32,
    p: &mut HashSet<Cell>,
    weight: u64,
    tx: u32,
    td: u32,
    min_y: i32,
    min_gap: i32,
    untried: &[Cell],
    blocked: &mut HashSet<Cell>,
    total: &mut Count,
) {
    // Edge-reachability prune (§4.1 condition 2): no descendant of this node
    // can ever touch both wedge edges within the remaining budget. Sound
    // because `weight + edge_reach_lb(..)` only grows down the recursion.
    if weight + edge_reach_lb(tx, td, min_y, min_gap) > n {
        return;
    }
    let mut blocked_here: Vec<Cell> = Vec::new();
    for i in 0..untried.len() {
        let c = untried[i];
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
                // Child frontier: the still-pending suffix plus c's fresh
                // neighbours (not in the slice, not blocked, not already
                // queued).
                let mut child: Vec<Cell> = untried[i + 1..].to_vec();
                for (dx, dy) in NEIGHBOURS {
                    let nb = (c.0 + dx, c.1 + dy);
                    if in_wedge(nb)
                        && nb.0 <= xmax
                        && nb > seed
                        && !p.contains(&nb)
                        && !blocked.contains(&nb)
                        && !child.contains(&nb)
                    {
                        child.push(nb);
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
                    &child,
                    blocked,
                    total,
                );
            }
            p.remove(&c);
        }
        // c is now excluded for the remaining branches at this level and
        // everything below them.
        if blocked.insert(c) {
            blocked_here.push(c);
        }
    }
    for c in blocked_here {
        blocked.remove(&c);
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

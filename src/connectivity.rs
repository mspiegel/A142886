//! The §4.1 slice-local connectivity predicate.
//!
//! Milestone **M2** (PLAN.md). DESIGN.md §4.1, §4.3.
//!
//! A D8-symmetric polyomino `P` reconstructed from a wedge slice `S` is a
//! single connected polyomino **iff** (DESIGN.md §4.1 lemma):
//!
//! 1. `S` is a single 4-connected component, **and**
//! 2. `S` contains an occupied cell on the x-axis edge of `W` *and* one on
//!    the diagonal edge `y = x`.
//!
//! In wedge coordinates both centers share the same edge formulas — x-axis
//! edge: second coord `== 0`; diagonal edge: first `==` second — so the
//! predicate is **center-independent** and never reconstructs `P`. The
//! reconstruction path here ([`reconstruct_then_bfs`]) exists only as a
//! brute-force oracle for tests / debug assertions, never the hot path.
//!
//! Inputs are assumed to be subsets of the fundamental wedge `W` (every cell
//! is its own orbit representative); the enumeration in M3 only ever produces
//! such slices.

use crate::symmetry::{image, Cell, Center, N_SYM};
use std::collections::HashSet;

/// 4-neighbour offsets.
const NEIGHBOURS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Is `cells` a single non-empty 4-connected component?
fn is_4_connected(cells: &HashSet<Cell>) -> bool {
    let Some(&start) = cells.iter().next() else {
        return false; // empty: not a polyomino
    };
    let mut seen: HashSet<Cell> = HashSet::with_capacity(cells.len());
    let mut stack = vec![start];
    seen.insert(start);
    while let Some((x, y)) = stack.pop() {
        for (dx, dy) in NEIGHBOURS {
            let nb = (x + dx, y + dy);
            if cells.contains(&nb) && seen.insert(nb) {
                stack.push(nb);
            }
        }
    }
    seen.len() == cells.len()
}

/// The §4.1 hot-path predicate: does this wedge slice reconstruct to a single
/// connected polyomino?
///
/// `slice` must be a subset of the fundamental wedge `W` (DESIGN.md §2). The
/// empty slice returns `false` (the empty polyomino `a(0)=1` is handled as an
/// explicit base case in enumeration, not here).
pub fn slice_is_connected_polyomino(slice: &HashSet<Cell>) -> bool {
    if slice.is_empty() {
        return false;
    }
    // Condition 2: touches both wedge edges. The apex / 2×2-core seed
    // (second == 0 and first == second) satisfies both on its own.
    let touches_x_axis = slice.iter().any(|&(_, y)| y == 0);
    let touches_diagonal = slice.iter().any(|&(x, y)| x == y);
    if !(touches_x_axis && touches_diagonal) {
        return false;
    }
    // Condition 1: a single 4-connected component.
    is_4_connected(slice)
}

/// Brute-force reconstruction `P = ⋃_{g ∈ D8} g·S` for the given center type.
///
/// Oracle/debug use only (DESIGN.md §4.3) — never called on the counting hot
/// path.
pub fn reconstruct(center: Center, slice: &HashSet<Cell>) -> HashSet<Cell> {
    let mut p = HashSet::with_capacity(slice.len() * N_SYM);
    for &c in slice {
        for g in 0..N_SYM {
            p.insert(image(center, g, c));
        }
    }
    p
}

/// Brute-force connectivity of the reconstructed polyomino: `true` iff `P` is
/// non-empty and 4-connected. The §4.1 lemma asserts this equals
/// [`slice_is_connected_polyomino`] for every wedge slice and either center.
pub fn reconstruct_then_bfs(center: Center, slice: &HashSet<Cell>) -> bool {
    let p = reconstruct(center, slice);
    is_4_connected(&p)
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! cells {
        ($(($x:expr, $y:expr)),* $(,)?) => {{
            let mut s = std::collections::HashSet::new();
            $( s.insert(($x as i32, $y as i32)); )*
            s
        }};
    }

    /// All subsets of the wedge cells `{ 0 ≤ y ≤ x ≤ R }` (same triangular
    /// region for both centers). Used to exhaustively cross-check the
    /// predicate against brute reconstruction.
    fn all_wedge_subsets(r: i32) -> Vec<HashSet<Cell>> {
        let mut wedge: Vec<Cell> = Vec::new();
        for x in 0..=r {
            for y in 0..=x {
                wedge.push((x, y));
            }
        }
        let n = wedge.len();
        (0..(1u32 << n))
            .map(|mask| {
                (0..n)
                    .filter(|&b| mask & (1 << b) != 0)
                    .map(|b| wedge[b])
                    .collect()
            })
            .collect()
    }

    /// Test (f) — DESIGN.md §7(f): the named §4.1 edge-condition cases.
    #[test]
    fn slice_predicate_edge_conditions() {
        // connected but touches NEITHER wedge edge -> rejected, no recon.
        assert!(!slice_is_connected_polyomino(&cells![(3, 1)]));
        assert!(!slice_is_connected_polyomino(&cells![(2, 1), (3, 1)]));
        // connected, spans x-axis edge to diagonal edge -> accepted
        // ({(1,0),(1,1)} = 3x3 ring; {(0,0),(1,0)} = plus pentomino).
        assert!(slice_is_connected_polyomino(&cells![(1, 0), (1, 1)]));
        assert!(slice_is_connected_polyomino(&cells![(0, 0), (1, 0)]));
        // connected slice touching ONLY the diagonal edge -> 4 disjoint pieces.
        assert!(!slice_is_connected_polyomino(&cells![(2, 1), (2, 2)]));
        // the bare apex / 2×2-core seed satisfies both edges on its own.
        assert!(slice_is_connected_polyomino(&cells![(0, 0)]));
    }

    /// Test (f) — DESIGN.md §7(f): the O(|S|) predicate must agree with brute
    /// reconstruct+BFS for every small wedge slice, for BOTH center types.
    #[test]
    fn slice_predicate_matches_reconstruction() {
        let subsets = all_wedge_subsets(4); // 15 wedge cells -> 32768 subsets
        for s in &subsets {
            let predicted = slice_is_connected_polyomino(s);
            for center in [Center::Cell, Center::Vertex] {
                assert_eq!(
                    predicted,
                    reconstruct_then_bfs(center, s),
                    "mismatch for {center:?} on slice {s:?}"
                );
            }
        }
    }
}

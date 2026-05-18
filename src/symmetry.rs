//! The dihedral group of order 8 (full symmetry group of the square) acting
//! on the integer lattice, plus orbit / fundamental-domain logic for the two
//! symmetry-center placements.
//!
//! Milestone **M1** (PLAN.md). DESIGN.md §2, §3.1, §3.2.
//!
//! Group-element indexing (used by every per-element array here) follows the
//! DESIGN.md §2 table:
//!
//! | idx | name | point map (about origin) |
//! |-----|------|--------------------------|
//! | 0   | e    | `( x,  y)`               |
//! | 1   | r    | `(-y,  x)`               |
//! | 2   | r²   | `(-x, -y)`               |
//! | 3   | r³   | `( y, -x)`               |
//! | 4   | s    | `( x, -y)`  (refl x-axis)|
//! | 5   | sr   | `( y,  x)`  (refl y=x)   |
//! | 6   | s r² | `(-x,  y)`  (refl y-axis)|
//! | 7   | s r³ | `(-y, -x)`  (refl y=-x)  |

/// A lattice cell, identified by integer index `(i, j)`.
pub type Cell = (i32, i32);

/// Number of elements in D8.
pub const N_SYM: usize = 8;

/// Which lattice feature the polyomino's D8 center sits on (DESIGN.md §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Center {
    /// Center at a lattice **cell** center (≈ A351127). DESIGN.md §3.1.
    Cell,
    /// Center at a lattice **vertex** (≈ A346800). DESIGN.md §3.2.
    Vertex,
}

/// Orbit-size class of a cell under D8 (DESIGN.md §3.1 / §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrbitClass {
    /// The single fixed cell (cell-centered apex only). Size 1.
    Apex,
    /// On a wedge edge: size-4 orbit.
    Edge,
    /// Strict interior: size-8 orbit.
    Interior,
}

impl OrbitClass {
    /// Number of distinct cells in an orbit of this class.
    pub fn size(self) -> usize {
        match self {
            OrbitClass::Apex => 1,
            OrbitClass::Edge => 4,
            OrbitClass::Interior => 8,
        }
    }
}

/// The 8 D8 transforms as their action on a lattice **point** about the
/// origin (DESIGN.md §2). For [`Center::Cell`] a cell index equals its center
/// point, so these double as the cell-index transforms.
pub const POINT: [fn(Cell) -> Cell; N_SYM] = [
    |(x, y)| (x, y),   // 0  e
    |(x, y)| (-y, x),  // 1  r
    |(x, y)| (-x, -y), // 2  r²
    |(x, y)| (y, -x),  // 3  r³
    |(x, y)| (x, -y),  // 4  s
    |(x, y)| (y, x),   // 5  s r
    |(x, y)| (-x, y),  // 6  s r²
    |(x, y)| (-y, -x), // 7  s r³
];

/// The same 8 transforms expressed as their action on a **cell index** when
/// the D8 center is a lattice vertex. Derived from the point maps applied to
/// the cell center `(i+½, j+½)` (DESIGN.md §3.2), e.g. `r` sends the cell
/// center to `(-(j+½), i+½)`, whose containing cell is `(-j-1, i)`.
pub const VERTEX_CELL: [fn(Cell) -> Cell; N_SYM] = [
    |(i, j)| (i, j),           // 0  e
    |(i, j)| (-j - 1, i),      // 1  r
    |(i, j)| (-i - 1, -j - 1), // 2  r²
    |(i, j)| (j, -i - 1),      // 3  r³
    |(i, j)| (i, -j - 1),      // 4  s
    |(i, j)| (j, i),           // 5  s r
    |(i, j)| (-i - 1, j),      // 6  s r²
    |(i, j)| (-j - 1, -i - 1), // 7  s r³
];

/// Apply group element `g` (0..8) to cell `c` for the given center type.
#[inline]
pub fn image(center: Center, g: usize, c: Cell) -> Cell {
    match center {
        Center::Cell => POINT[g](c),
        Center::Vertex => VERTEX_CELL[g](c),
    }
}

/// The orbit of `c` under D8: every distinct cell reachable by the 8
/// elements, sorted for determinism.
pub fn orbit(center: Center, c: Cell) -> Vec<Cell> {
    let mut o: Vec<Cell> = (0..N_SYM).map(|g| image(center, g, c)).collect();
    o.sort_unstable();
    o.dedup();
    o
}

/// `|i+½|` re-expressed on the integer cell index: the distance class of a
/// vertex-centered cell from the center along one axis. `fold(i) == fold(j)`
/// iff the cell lies on the 45° diagonal through the vertex.
#[inline]
fn fold(i: i32) -> i32 {
    if i >= 0 {
        i
    } else {
        -i - 1
    }
}

/// The unique representative of `c`'s orbit lying in the fundamental wedge
/// `W` (DESIGN.md §2). For both centers this is the orbit member with
/// `0 ≤ Y ≤ X` in the appropriate coordinates.
pub fn representative(center: Center, c: Cell) -> Cell {
    let (a, b) = match center {
        Center::Cell => (c.0.abs(), c.1.abs()),
        Center::Vertex => (fold(c.0), fold(c.1)),
    };
    (a.max(b), a.min(b))
}

/// Is `c` the wedge representative of its orbit (i.e. `c ∈ W`)?
#[inline]
pub fn in_wedge(center: Center, c: Cell) -> bool {
    representative(center, c) == c
}

/// Orbit-size class of `c` (DESIGN.md §3.1 / §3.2).
///
/// - Cell-centered: apex `(0,0)` → 1; on the x-axis edge (`Y=0, X>0`) or the
///   diagonal edge (`X=Y>0`) → 4; strict interior → 8.
/// - Vertex-centered: no apex and no x-axis fixed cells; on the diagonal
///   (`fold(i)=fold(j)`) → 4; otherwise → 8.
pub fn orbit_class(center: Center, c: Cell) -> OrbitClass {
    let (x, y) = representative(center, c);
    match center {
        Center::Cell => {
            if x == 0 {
                OrbitClass::Apex // (0,0)
            } else if y == 0 || x == y {
                OrbitClass::Edge
            } else {
                OrbitClass::Interior
            }
        }
        Center::Vertex => {
            if x == y {
                OrbitClass::Edge // diagonal cells, incl. the 2×2 core (0,0)
            } else {
                OrbitClass::Interior
            }
        }
    }
}

/// Number of distinct cells in `c`'s orbit (1, 4, or 8).
#[inline]
pub fn orbit_size(center: Center, c: Cell) -> usize {
    orbit_class(center, c).size()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// 2×2 integer matrix for the linear part of group element `g`, obtained
    /// from its action on the basis vectors. The linear part is identical for
    /// both center types (the vertex action is the same group, only affine).
    fn matrix(g: usize) -> [i32; 4] {
        let (a, c) = POINT[g]((1, 0));
        let (b, d) = POINT[g]((0, 1));
        [a, b, c, d]
    }

    fn mat_mul(m: [i32; 4], n: [i32; 4]) -> [i32; 4] {
        [
            m[0] * n[0] + m[1] * n[2],
            m[0] * n[1] + m[1] * n[3],
            m[2] * n[0] + m[3] * n[2],
            m[2] * n[1] + m[3] * n[3],
        ]
    }

    fn group_is_closed_with_inverses() -> bool {
        let mats: Vec<[i32; 4]> = (0..N_SYM).map(matrix).collect();
        let set: HashSet<[i32; 4]> = mats.iter().copied().collect();
        if set.len() != N_SYM {
            return false; // 8 distinct elements
        }
        let id = [1, 0, 0, 1];
        if !set.contains(&id) {
            return false;
        }
        for &m in &mats {
            // closure
            for &n in &mats {
                if !set.contains(&mat_mul(m, n)) {
                    return false;
                }
            }
            // every element has an inverse within the set
            if !mats.iter().any(|&n| mat_mul(m, n) == id) {
                return false;
            }
        }
        true
    }

    /// Test (e) — DESIGN.md §7(e): group axioms and orbit sizes.
    #[test]
    fn group_axioms_and_orbit_sizes() {
        assert!(group_is_closed_with_inverses());

        // Cell-centered orbit sizes (DESIGN.md §3.1).
        assert_eq!(orbit_size(Center::Cell, (0, 0)), 1); // apex
        assert_eq!(orbit_size(Center::Cell, (3, 0)), 4); // x-axis edge
        assert_eq!(orbit_size(Center::Cell, (3, 3)), 4); // diagonal edge
        assert_eq!(orbit_size(Center::Cell, (3, 1)), 8); // interior

        // Vertex-centered orbit sizes (DESIGN.md §3.2): no apex, no x-axis
        // fixed cells; the 2×2 core and diagonal cells are size 4.
        assert_eq!(orbit_size(Center::Vertex, (0, 0)), 4); // 2×2 core
        assert_eq!(orbit_size(Center::Vertex, (2, 2)), 4); // diagonal
        assert_eq!(orbit_size(Center::Vertex, (1, 0)), 8); // generic
        assert_eq!(orbit_size(Center::Vertex, (3, 1)), 8); // generic

        // The classifier must agree with the materialized orbit, for both
        // centers, across the wedge and its images.
        for center in [Center::Cell, Center::Vertex] {
            for x in -6..=6 {
                for y in -6..=6 {
                    let c = (x, y);
                    let o = orbit(center, c);
                    assert_eq!(
                        o.len(),
                        orbit_size(center, c),
                        "{center:?} orbit size mismatch at {c:?}: {o:?}"
                    );
                    // Exactly one orbit member is the wedge representative.
                    let reps = o.iter().filter(|&&p| in_wedge(center, p)).count();
                    assert_eq!(reps, 1, "{center:?} {c:?} reps={reps} orbit={o:?}");
                    // representative() is stable across the whole orbit.
                    let r = representative(center, c);
                    for &p in &o {
                        assert_eq!(representative(center, p), r);
                    }
                }
            }
        }
    }
}

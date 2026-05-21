//! OEIS **A142886** — number of polyominoes with `n` cells that have the full
//! D8 (square) symmetry group.
//!
//! See `DESIGN.md` (technical design) and `PLAN.md` (execution plan) in the
//! crate root. Module map:
//!
//! - [`symmetry`]   — the 8 D8 transforms; orbits and representatives (M1).
//! - [`connectivity`] — the §4.1 slice-local connectivity predicate (M2).
//! - [`enumerate`]  — Redelmeier growth in the fundamental wedge (M3).
//! - [`verify`]     — reference vector / b-file checks (M4).

pub mod connectivity;
pub mod enumerate;
pub mod symmetry;
pub mod verify;

/// Term value type for `a(n)`.
///
/// DESIGN.md §5 argues `u64` is sufficient through `n = 163`. This alias is
/// the single swap point to a big-integer backend if that ever changes — no
/// call-site churn.
pub type Count = u64;

pub use enumerate::{
    count, count_cell_centered, count_cell_centered_parallel, count_parallel,
    count_parallel_sharded, count_vertex_centered, count_vertex_centered_parallel,
};

//! Polyomino counting for OEIS **A142886** (full D8 symmetry).
//!
//! Milestone **M6** (PLAN.md) — Jensen transfer-matrix bring-up.
//!
//! Two engines coexist *only during the gated M6 bring-up*, never as a
//! shipped feature-gated variant:
//!
//! - [`legacy`]   — the committed Redelmeier wedge enumerator (DESIGN.md
//!   §4.2/§4.6). Exact and byte-identical to the OEIS b-file for n ≤ 68;
//!   reaches ≈ n=108 before the ≈1.32ⁿ wall. Stays the live engine through
//!   the M6 bring-up so the crate is green at every commit.
//! - [`transfer`] — the new anti-diagonal Jensen transfer-matrix DP
//!   (DESIGN.md §4.7), targeting the n ≫ 110 / full-b-file regime where the
//!   §4.5 *depth-conditioned* rejection no longer holds. Reachable only from
//!   tests until it passes the differential gate.
//!
//! **GO** (transfer byte-identical to legacy for all feasible n and matching
//! the b-file past n≈110): flip the `pub use` below to `transfer`, make
//! `legacy` `#[cfg(test)]`, then delete it in the same commit. **NO-GO**:
//! delete `transfer`, `git mv` `legacy.rs` back to `enumerate.rs` (the split
//! is a pure move — trivial revert), record measured numbers in FUTURE.md.

pub(crate) mod legacy;
mod transfer;

// ── M6 engine dispatch (the single GO flip point) ───────────────────────────
// Bring-up: the public API is the proven `legacy` engine. On GO this single
// line becomes `pub use transfer::{...}` and `legacy` is deleted.
pub use legacy::{count, count_cell_centered, count_vertex_centered};

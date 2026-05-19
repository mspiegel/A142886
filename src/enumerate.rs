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

/// One byte per bounded-wedge cell encoding its full membership: `p`,
/// `blocked`, **and** `in_untried` are folded into a single state machine
/// (replaces all three `CellSet` bitsets — no hashing, no bit-twiddling,
/// one cache-resident array).
///
/// A cell is in exactly one of:
/// - `FREE`    — not in the slice, not blocked, not in the `untried` buffer;
/// - `QUEUED`  — physically in the `untried` buffer, awaiting decision;
/// - `SLICE`   — currently a slice cell (chosen up the call stack);
/// - `BLOCKED` — Redelmeier-excluded for the current frame's later siblings.
///
/// These are mutually exclusive because `in_untried` is only ever *tested*
/// on cells that already passed `is_free` (FREE), so "in the buffer" is a
/// clean sub-state of FREE. The lifecycle, mirroring the buffer discipline
/// exactly (so every count is unchanged — byte-identical, oracle-verified):
///
/// ```text
/// FREE --set_queued--> QUEUED --set_slice--> SLICE --unset_slice--> QUEUED
/// QUEUED --set_blocked--> BLOCKED --unblock--> QUEUED   (frame-exit unwind)
/// QUEUED --dequeue--> FREE                              (truncation pop)
/// FREE --place_seed--> SLICE                            (bucket root only)
/// ```
///
/// The hot `is_free(nb)` test now subsumes `!p.contains && !blocked.contains
/// && !in_untried` in a single byte read; every transition is
/// `debug_assert`-guarded so a stray edge is caught in test builds in
/// addition to the byte-identical oracle.
struct CellState {
    s: Vec<u8>,
    stride: usize,
}

impl CellState {
    const FREE: u8 = 0;
    const QUEUED: u8 = 1;
    const SLICE: u8 = 2;
    const BLOCKED: u8 = 3;

    fn new(xmax: i32) -> Self {
        let stride = xmax as usize + 1;
        CellState {
            s: vec![Self::FREE; stride * stride],
            stride,
        }
    }
    #[inline]
    fn idx(&self, c: Cell) -> usize {
        c.0 as usize * self.stride + c.1 as usize
    }
    // SAFETY contract shared by every `s` accessor below (coordinate and
    // `_at` family alike): the index is always `< s.len()`. Every cell that
    // reaches an accessor first passed the `in_wedge` + `nb.0 <= xmax` +
    // `forbidden` guards, so `0 ≤ y ≤ x ≤ xmax`; with `stride = xmax + 1`,
    // `idx = x*stride + y ≤ xmax*stride + xmax < stride*stride = s.len()`.
    // The `_at` callers additionally `debug_assert_eq!(ni, st.idx(nb))`, so
    // the add-derived index is proven equal to the coordinate form in test
    // builds. Each accessor `debug_assert!`s the bound too, so any stray
    // index is caught by the debug oracle / test suite before this `unsafe`
    // is ever trusted in release.
    #[inline]
    fn is_free(&self, c: Cell) -> bool {
        let i = self.idx(c);
        debug_assert!(i < self.s.len(), "idx {i} OOB (len {})", self.s.len());
        // SAFETY: see the contract above.
        *unsafe { self.s.get_unchecked(i) } == Self::FREE
    }
    #[inline]
    fn step(&mut self, c: Cell, from: u8, to: u8) {
        let i = self.idx(c);
        debug_assert!(i < self.s.len(), "idx {i} OOB (len {})", self.s.len());
        // SAFETY: see the contract above.
        let slot = unsafe { self.s.get_unchecked_mut(i) };
        debug_assert_eq!(*slot, from);
        *slot = to;
    }
    // ── Index (`*_at`) family ─────────────────────────────────────────
    // The `_at` suffix means "operate on a precomputed flat index
    // `i = x*stride + y`" rather than a `Cell` (which would recompute that
    // multiply). The hot recursion (`grow`) derives the 4 neighbour
    // indices as `ci ± 1` / `ci ± stride` (an add — lever #1) and reuses
    // `ci = idx(c)` for `c`'s own state ops; `*_at(idx(x))` is identical
    // in effect to the coordinate form `*(x)`.
    #[inline]
    fn is_free_at(&self, i: usize) -> bool {
        debug_assert!(i < self.s.len(), "idx {i} OOB (len {})", self.s.len());
        // SAFETY: see the contract on `is_free`.
        *unsafe { self.s.get_unchecked(i) } == Self::FREE
    }
    #[inline]
    fn step_at(&mut self, i: usize, from: u8, to: u8) {
        debug_assert!(i < self.s.len(), "idx {i} OOB (len {})", self.s.len());
        // SAFETY: see the contract on `is_free`.
        let slot = unsafe { self.s.get_unchecked_mut(i) };
        debug_assert_eq!(*slot, from);
        *slot = to;
    }
    #[inline]
    fn set_queued_at(&mut self, i: usize) {
        self.step_at(i, Self::FREE, Self::QUEUED);
    }
    #[inline]
    fn set_slice_at(&mut self, i: usize) {
        self.step_at(i, Self::QUEUED, Self::SLICE);
    }
    #[inline]
    fn unset_slice_at(&mut self, i: usize) {
        self.step_at(i, Self::SLICE, Self::QUEUED);
    }
    #[inline]
    fn set_blocked_at(&mut self, i: usize) {
        self.step_at(i, Self::QUEUED, Self::BLOCKED);
    }
    // ── Coordinate family ─────────────────────────────────────────────
    // Cold sites only (bucket-seed frontier build, truncation dequeue,
    // frame-exit unblock): no precomputed index to reuse, so they take a
    // `Cell` and compute the index themselves.
    #[inline]
    fn place_seed(&mut self, c: Cell) {
        self.step(c, Self::FREE, Self::SLICE);
    }
    #[inline]
    fn set_queued(&mut self, c: Cell) {
        self.step(c, Self::FREE, Self::QUEUED);
    }
    #[inline]
    fn unblock(&mut self, c: Cell) {
        self.step(c, Self::BLOCKED, Self::QUEUED);
    }
    #[inline]
    fn dequeue(&mut self, c: Cell) {
        self.step(c, Self::QUEUED, Self::FREE);
    }
}

#[inline]
fn in_wedge(c: Cell) -> bool {
    c.0 >= 0 && c.1 >= 0 && c.1 <= c.0
}

/// Admissible lower bound on the *extra* orbit-weight any completion still
/// needs in order to satisfy §4.1 condition 2 (touch both wedge edges).
/// `min_gap` = smallest `x − y` over the slice.
///
/// **Only the diagonal term remains.** The §4.1 x-axis sub-condition is met
/// at every bucket root — the A-rooted scheme pins every seed on the x-axis
/// (`seed = (ax, 0)` ⇒ `tx ≥ 1` at every node) — so the old x-axis term
/// `wx = if tx>0 {0} else {4·min_y}` was provably 0 everywhere and `min_y`
/// pure dead state; both are dropped.
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
/// **Soundness is per-node admissibility, not monotonicity:**
/// `weight + edge_reach_lb` is *not* monotone down the recursion (the
/// diagonal term can fall by 8 while `weight` rises only 4), but if
/// `weight + lb > n` at a node then every completion's total
/// `= weight + added ≥ weight + lb > n` (a valid slice must touch the
/// diagonal), so the subtree holds no weight-`n` slice — independent of
/// descendants. (DESIGN §4.6(b).)
#[inline]
fn edge_reach_lb(td: u32, min_gap: i32) -> u64 {
    // `td == 0` ⇒ no gap-0 cell ⇒ `min_gap ≥ 1`, so `8·min_gap − 4 ≥ 4`
    // and the `u64` subtraction cannot underflow.
    debug_assert!(td > 0 || min_gap >= 1, "td == 0 must imply min_gap ≥ 1");
    if td > 0 {
        0
    } else {
        8 * (min_gap as u64) - 4
    }
}

// (`on_x_axis_edge` removed: the A-rooted scheme pins every seed on the
// x-axis, so the x-axis touch is met at every bucket root — `tx ≡ 1` — and
// the predicate is never tested. `forbidden` checks `c.1 == 0` directly.)
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
        // `tx ≡ 1`: every bucket seed is `(ax, 0)`, on the x-axis, so the
        // §4.1 x-axis sub-condition is satisfied at the root of every bucket
        // and the predicate reduces to the diagonal touch (`td`).
        let td = u32::from(on_diagonal_edge(seed));
        if ws == n {
            if td > 0 {
                total += 1;
            }
            continue; // any extension exceeds n
        }
        // Edge-reachability prune (the "ring" case): if even the cheapest
        // completion cannot touch both wedge edges within the budget, this
        // bucket's whole subtree is dead (§4.1 condition 2). `A` is on the
        // x-axis so only the diagonal term (min_gap = ax) can bind.
        if ws + edge_reach_lb(td, seed.0 - seed.1) > n {
            continue;
        }
        // Scratch structures allocated once per bucket and reused across
        // the bucket's entire recursion subtree (push/truncate, never a
        // per-node `Vec`): `st` = fused FREE/QUEUED/SLICE/BLOCKED state
        // (`p`, `blocked` *and* `in_untried` collapsed), `untried` = the
        // shared growth buffer. (`blk_unwind` and the `in_untried` bitset
        // are gone — each frame's blocked set is exactly its frontier
        // window `untried[lo..hi]`, and "in the buffer" is the QUEUED state.)
        let mut st = CellState::new(xmax);
        st.place_seed(seed);
        let mut untried: Vec<Cell> = Vec::new();
        for (dx, dy) in NEIGHBOURS {
            let nb = (seed.0 + dx, seed.1 + dy);
            if in_wedge(nb) && nb.0 <= xmax && !forbidden(nb, seed.0) && st.is_free(nb) {
                st.set_queued(nb);
                untried.push(nb);
            }
        }
        if td > 0 {
            // Seed already satisfies §4.1 (ax==0: apex / 2×2 core) — straight
            // into the SAT specialization (td/min_gap unused there ⇒ 0).
            grow::<true>(
                center,
                n,
                seed,
                xmax,
                &mut st,
                ws,
                0,
                0,
                0,
                &mut untried,
                &mut total,
            );
        } else {
            grow::<false>(
                center,
                n,
                seed,
                xmax,
                &mut st,
                ws,
                td,
                seed.0 - seed.1,
                0,
                &mut untried,
                &mut total,
            );
        }
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
/// Slice, blocked, and `untried`-buffer membership are one `CellState`
/// byte per cell (FREE/QUEUED/SLICE/BLOCKED). The uniqueness/dedup check is
/// the single `st.is_free(nb)` read — it rejects already-queued (QUEUED),
/// in-slice (SLICE) and excluded (BLOCKED) candidates at once, subsuming
/// the former `!p && !blocked && !in_untried`.
///
/// `untried[pos]` is BLOCKED after its branch — excluding it for later
/// siblings and their subtrees. This frame's blocked additions are exactly
/// its frontier window `untried[lo..hi]` (every such cell was QUEUED and is
/// blocked exactly once here), so exit unwinds by scanning that window
/// (BLOCKED→QUEUED — they stay in the buffer); no `blk_unwind` stack.
///
/// The x-axis sub-condition of §4.1 is met at every bucket root (`tx ≡ 1`,
/// seed on the x-axis), so it is never tracked; the predicate reduces to the
/// diagonal touch `td` and its budget term is `edge_reach_lb(td, min_gap)`.
///
/// **`const SAT`** specializes on whether the diagonal is already touched
/// (the remaining half of §4.1). It is monotone (`td` only grows), so once
/// `td > 0` the `edge_reach_lb` prune is provably inert and the accept gate
/// is always true. `SAT == false` is the general path; `SAT == true` is the
/// post-satisfaction fast path. The compiler monomorphizes two copies and
/// folds every `!SAT` / `SAT ||` at compile time, so the `SAT == true`
/// machine code is a hand-stripped fast path (no `edge_reach_lb`, no per-cell
/// diagonal test, no `td/min_gap` upkeep, unconditional accept) — one source,
/// zero runtime dispatch. The frontier/`CellState` discipline is identical
/// for both, so the generated slice set — and every count — is unchanged.
/// `td/min_gap` are unused (and DCE'd) when `SAT`; pass 0. `seed` is always
/// needed for `forbidden` (bucketing is independent of §4.1).
#[allow(clippy::too_many_arguments)]
fn grow<const SAT: bool>(
    center: Center,
    n: u64,
    seed: Cell,
    xmax: i32,
    st: &mut CellState,
    weight: u64,
    td: u32,
    min_gap: i32,
    lo: usize,
    untried: &mut Vec<Cell>,
    total: &mut Count,
) {
    // Edge-reachability prune (§4.1 condition 2): no descendant of this node
    // can ever touch both wedge edges within the remaining budget. Sound
    // because `weight + edge_reach_lb(..)` only grows down the recursion.
    // Skipped entirely in the SAT specialization (it would return 0 — the
    // predicate is already satisfied — so the whole check is folded out).
    if !SAT && weight + edge_reach_lb(td, min_gap) > n {
        return;
    }
    // Lever #1: the 4-neighbour `CellState` index is `ci ± 1` / `ci ± stride`
    // (an add), not a fresh `x*stride + y` multiply. `stride` is fixed for
    // the whole `grow` call; `ci = idx(c)` is computed once per `pos`.
    let stride_i = st.stride as isize;
    let hi = untried.len(); // this level's frontier is untried[lo..hi]
    for pos in lo..hi {
        let c = untried[pos];
        let ci = st.idx(c);
        let w2 = weight + orbit_size(center, c) as u64;
        if w2 <= n {
            // tx ≡ 1 globally (seed on x-axis), so §4.1 reduces to `td`.
            // SAT: ntd unused (folded away); 1 keeps the shared accept/
            // dispatch expressions trivially true with no edge-class test.
            let ntd = if SAT {
                1
            } else {
                td + u32::from(on_diagonal_edge(c))
            };
            st.set_slice_at(ci);
            if w2 == n {
                // §4.1: connected by construction; x-axis is met at the seed,
                // so only the diagonal touch (`td`) gates (unconditional SAT).
                if SAT || ntd > 0 {
                    *total += 1;
                }
                // cannot extend further (weight would exceed n)
            } else {
                // Every prior branch truncated back to `hi`, so the buffer
                // ends exactly at this level's suffix here.
                debug_assert_eq!(untried.len(), hi);
                // Append c's fresh neighbours after the suffix. The membership
                // index is `ci + (dx*stride + dy)` (add, not multiply); only
                // formed after the wedge/xmax/forbidden guards pass (so it is
                // provably in-bounds and == `idx(nb)`, asserted in debug).
                for (dx, dy) in NEIGHBOURS {
                    let nb = (c.0 + dx, c.1 + dy);
                    if in_wedge(nb) && nb.0 <= xmax && !forbidden(nb, seed.0) {
                        let ni =
                            ci.wrapping_add_signed(dx as isize * stride_i + dy as isize);
                        debug_assert_eq!(ni, st.idx(nb));
                        if st.is_free_at(ni) {
                            st.set_queued_at(ni);
                            untried.push(nb);
                        }
                    }
                }
                if SAT || ntd > 0 {
                    // §4.1 satisfied (or already) — monotone, so the whole
                    // child subtree takes the SAT specialization. (On the SAT
                    // path this is the only arm; the else is folded out.)
                    if n - w2 == 4 {
                        // R=4 tail-fold (DESIGN/FUTURE lever G). The SAT
                        // child has remaining budget 4. Every cell is
                        // weight ∈ {4,8} (the only weight-1 cell, the apex,
                        // is the seed or forbidden, so never a fresh
                        // frontier cell). A weight-4 cell completes
                        // (`w2'==n`) and is accepted unconditionally (SAT);
                        // a weight-8 cell overshoots (`w2'=n+4>n`) and is
                        // skipped. Either way the child never recurses and
                        // never appends to `untried`, and its `blocked`
                        // inserts are fully self-unwound and never read
                        // (no neighbour expansion). So the child's entire
                        // contribution is exactly the number of weight-4
                        // cells in its frontier `untried[pos+1..]` — fold
                        // it inline, skipping the call + per-node frame and
                        // the blocked/unwind bookkeeping. Provably
                        // count-identical to the recursion.
                        for k in (pos + 1)..untried.len() {
                            if orbit_size(center, untried[k]) as u64 == 4 {
                                *total += 1;
                            }
                        }
                    } else {
                        grow::<true>(
                            center,
                            n,
                            seed,
                            xmax,
                            st,
                            w2,
                            0,
                            0,
                            pos + 1,
                            untried,
                            total,
                        );
                    }
                } else {
                    grow::<false>(
                        center,
                        n,
                        seed,
                        xmax,
                        st,
                        w2,
                        ntd,
                        min_gap.min(c.0 - c.1),
                        pos + 1,
                        untried,
                        total,
                    );
                }
                // Drop this branch's appended neighbours. They leave the
                // buffer here ⇒ QUEUED → FREE (the unique "dequeue" site).
                while untried.len() > hi {
                    let nb = untried.pop().unwrap();
                    st.dequeue(nb);
                }
            }
            st.unset_slice_at(ci); // SLICE → QUEUED (c is still in the buffer)
        }
        // c is now excluded for this frame's later siblings. It was QUEUED
        // (a frontier cell still in the buffer, or just un-chosen above);
        // QUEUED → BLOCKED.
        st.set_blocked_at(ci);
    }
    // Unwind this frame's blocks = exactly `untried[lo..hi]` (untouched:
    // the loop only appends past `hi` and truncates back). These cells stay
    // physically in the buffer for an ancestor frame ⇒ BLOCKED → QUEUED
    // (NOT free — an ancestor's truncation is the only dequeue).
    for pos in lo..hi {
        st.unblock(untried[pos]); // BLOCKED → QUEUED
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
    /// (n = 0..=68). Formerly `#[ignore]`d per the §7 baseline-runtime note;
    /// now always-on — runs in <0.01s on the optimized enumerator.
    #[test]
    fn matches_oeis_prefix_full() {
        for (n, &expected) in REFERENCE.iter().enumerate() {
            assert_eq!(count(n), expected, "a({n}) mismatch");
        }
    }
}

# FUTURE — deferred work for A142886

Parked, not in progress. Captured here so it need not be re-derived or
re-raised in conversation.

_No deferred work is currently parked._

## Resolved / evaluated-and-rejected (do not re-propose)

- **Minimal-x-axis-cell bucketing — shipped.** The §4.2 enumerator now
  buckets by the slice's minimal x-axis cell `A=(ax,0)` and grows from the
  pinned root `A` (injectivity from the blocked-set discipline; the global
  lex-min `nb>seed` shortcut dropped, no cheap analog for an edge-pinned
  root). ≈2× faster than the prior lex-min-seed scheme, ratio non-eroding,
  counts byte-identical (n≤100 vs the prior scheme; n≤68 vs b-file /
  `REFERENCE`). See DESIGN §4.2 / §4.6.
- **Joint cell-budget + gap diagonal reach bound — shipped.** The §4.6(b)
  diagonal term tightened from `4·min_gap` to `8·min_gap−4`: the forbidden
  region blocks the weight-4 x-axis route to the diagonal, so the
  gap-reducing connectors are interior (orbit weight 8) — the exact minimum.
  ≈1.33× faster, ratio non-eroding, byte-identical (n≤100 vs the prior
  engine; n≤68 vs b-file/`REFERENCE`); O(1), no per-node search. Soundness
  is per-node admissibility (the bound is not monotone). See DESIGN §4.6(b).
- **Tighter `xmax` for `CellSet` cache (#2) — evaluated and rejected.**
  Capping `xmax` at `n/2` (rigorous: any cell of a valid both-edges
  connected slice has `x ≤ 2|S|−2 ≤ n/2`) is *correct* (byte-identical
  n≤100 vs `e3dd44e`, n≤68 vs b-file/`REFERENCE`) but **~5–7% slower**, not
  faster, across n=96–112. The cache hypothesis was wrong: the per-bucket
  bitset *working set* is the actual slice extent, not `xmax`, and was
  already cache-resident — shrinking the allocation bound buys nothing and
  adds slight overhead. Profile tail (lines 52/82/58) unchanged.
  **Do not re-attempt** (any `xmax` constant fails for the same reason —
  the bitset is not the bottleneck; the recursion is).
  *Open follow-on (not pursued):* `rayon` over the ~`n` independent buckets
  (≈#cores wall-clock, count-preserving) — the one remaining real lever.
- **Two-terminal `(A,B)` enumerator — rejected.** Also pinning the minimal
  diagonal cell `B=(bx,bx)` and bucketing by the pair `(A,B)` (accept iff
  `B∈S`, so the §4.1 both-edges condition holds by construction instead of
  generate-then-reject) is *correct* (byte-identical n≤68) but ≈O(n)
  slower: every slice is fully re-grown and rejected in each bucket
  `bx = 0..D-1` where `(D,D)` is its true minimal diagonal cell (profiled
  ≥50× slower at n=88, compounding). Dropping the `B` dimension *is* the
  shipped x-axis-rooted scheme above. See DESIGN §4.5.
- **Transfer/kernel reformulation — rejected** (its per-node state
  bookkeeping outweighs the branching it removes; DESIGN §4.5).
- **Column-span / horizontal-extent reach bound — refuted analytically
  (identically 0; not implemented).** Idea: `max`-combine an extra
  admissible term `4·(columns that must be newly occupied to reach the
  diagonal)` with `8·min_gap−4`. An admissible bound must be ≤ the *minimum*
  extra weight over *all* valid completions. From the min-gap cell `(x0,y0)`
  a completion can **always** land the diagonal at a column `≤` the current
  `max_x`: if `y0≥1` walk `x` down to `(y0,y0)` (interior, never forbidden);
  if `y0=0` the x-axis route is forbidden so walk `y` up to `(x0,x0)`,
  `x0 ≤ ax ≤ max_x` — then fill weight `n` with interior cells in existing
  columns. So *some* completion adds **0** new columns ⇒ the only admissible
  column term is `0` ⇒ it can never prune. Any nonzero variant is
  inadmissible (would change counts). Stronger than a measured wash — a
  proof — so no implement/benchmark cycle was spent. This was the lone
  un-refuted single-core candidate; with it refuted the **single-core,
  count-preserving optimization ledger is closed**: #nodes is bounded by the
  residue/bucket split + the *provably exact* diagonal-reach bound, per-node
  cost is irreducible inlined work, and every structural reformulation is
  rejected above. The only remaining lever is **`rayon` over the ~`n`
  independent buckets** (parallelism — count-preserving, not single-core);
  beyond that the honest path to more terms is more compute, not algorithm.
- **Redelmeier 1981 §6 size-reduction identity `HVADR2X(4p)=A'(p)`
  (vertex part) — evaluated, Amdahl-rejected; identity retained as an
  oracle.** Redelmeier's paper (read in full) gives `count_vertex_centered(4p)
  = A'(p)` = #fixed polyominoes of size `p` with a LL-UR diagonal axis =
  **OEIS A346800(p)** (this is A142886's own published formula,
  `a(n)=A351127(n)+A346800(n/4)`; *not* novel). **Phase 1 validated**
  `count_vertex_centered(4p) == A346800(p)` byte-identical for p=1..31
  (n≤124), 0 mismatches — so the A346800 b-file (p≤66 ⇒ n≤264) is now a
  usable independent cross-check oracle for the vertex part (cheap
  `#[ignore]` test, like `matches_bfile`, if ever wanted). **Phase 2
  gate failed:** the runtime split (n=116) is cell 12.42 s / vertex 1.79 s /
  both 14.26 s — the vertex part is only ≈12.5 % of runtime (≈10–19 % of
  `a(n)`). A diagonal A'(p) Redelmeier enumerator optimizes *only* vertex,
  is the *same exponential base* (it produces exactly the same `A346800(p)`
  accepted counts; ~1.5–3× constant edge at most from no §4.1 discard), so
  best-realistic ≈1.05–1.09× overall (Amdahl ceiling ≈1.15× even if vertex
  were free), with **reachable-n unchanged** (dominant ≈87 % cell part
  untouched). Redelmeier's cell-centered (HVADR2I) handling is *filter-
  based* — already inferior to our §4.1 lemma. A whole second enumerator
  for a single-digit-%, non-asymptotic, Amdahl-capped gain is a poor trade
  — not pursued. Confirms the ledger conclusion with hard numbers. (Also
  noted in A346800: Jensen/Knuth transfer-matrix "by diagonals" — same
  rejected class as DESIGN §4.5, not pursued.)

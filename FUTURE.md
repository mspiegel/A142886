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
  *Open follow-ons (same cell-budget fact, not pursued):* tighter `xmax`
  for `CellSet` cache/locality (#2); `rayon` over the ~`n` independent
  buckets (≈#cores wall-clock, count-preserving).
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

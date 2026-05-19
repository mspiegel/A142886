# FUTURE — deferred work for A142886

Parked, not in progress. Captured here so it need not be re-derived or
re-raised in conversation.

_No deferred work is currently parked._

## Proven exact results (retained — not a rejection, not deferred)

- **Boundary-bucket closed form (cell-centered, 8 | n): the top surviving
  bucket contributes exactly `2^(n/8 − 1)`, no recursion.** Cell-centered
  bucket `ax` has seed weight 4 and `edge_reach_lb = 8·ax − 4`, so it
  survives iff `8·ax ≤ n`. Define slack `s = n − 8·ax ≥ 0`. When `s = 0`
  (requires `8 | n`; boundary bucket `ax = n/8`) there is zero slack: every
  weight-`n` slice must hit the diagonal at exactly minimum cost, forcing a
  single monotone gap-descent staircase — gaps `ax → ax−1 → … → 0`, one cell
  per gap (`ax+1` cells), weights `4 + 8(ax−1) + 4 = 8·ax = n` automatically.
  Each step is "up" (`y+1`) or "left" (`x−1`); step 1 cannot be "left"
  (`(ax−1,0)` is x-axis with `x < ax` ⇒ `forbidden`), and after one "up"
  `y ≥ 1` forever so `forbidden` never bites again and all intermediates are
  weight-8 interior automatically. ⇒ count = #{move strings of length `ax`
  over {up,left} with move 1 = up} = **`2^(ax−1) = 2^(n/8−1)`**. Distinct
  staircases ⇒ distinct slices; sole x-axis cell is the seed ⇒ all live only
  in bucket `ax`, each counted once. Verified: n=8→2⁰=1 (the 3×3 ring,
  `a(8)=1`), n=16→2¹=2, n=24→2²=4 (the four staircases), n=32→2³=8.
  Every node on these paths has `weight + edge_reach_lb == n` exactly, never
  `> n`, so the §4.6 prune passes all leaves — consistent, not pruned away.
  **Scope (do not over-claim):** holds *only* for the single boundary bucket
  and *only* when `8 | n`. `n ≡ 4 (mod 8)` ⇒ top bucket has slack `s = 4`,
  no clean power of 2. Interior buckets `ax = 1 … n/8−1` have positive slack
  and stay combinatorial — this does **not** dent the exponential core
  (Amdahl: it's the cheapest, most degenerate bucket). Vertex-centered: same
  *style* of argument, different constant (no apex; x-axis cells weight 8).
  **Use:** a cheap exact per-bucket invariant — a usable differential
  cross-check on the enumerator's top-bucket output if per-bucket counts are
  ever exposed (cf. the diagonal-rooting cross-check idea).

## Measured profiles (empirical, trustworthy — corrects an earlier structural guess)

- **Per-bucket cost distribution (cell-centered) is a broad low-`ax`
  plateau, NOT an `ax=1` spike or a monotone decline.** Measured with
  env-gated per-bucket `Instant` instrumentation around the §4.2 bucket loop
  (gated by `A142886_BUCKET_TIMING`, count-preserving — release build).
  Shape is stable across n=48/56/80/84/88. At **n=88** (cell, `ax=1..11`),
  share of cell-run time: ax1 13.8%, ax2 16.1%, **ax3 16.5% (peak)**, ax4
  16.0%, ax5 14.2%, ax6 10.9%, ax7 6.9%, ax8 3.6%, ax9 1.5%, ax10 0.4%,
  ax11 0.06%. Per-bucket time tracks the per-bucket *slice count* almost
  exactly. So: a flat plateau over `ax = 1…5` (each ~14–16%), peak at
  **`ax ≈ 3`** (`ax=1` is *below* the peak), then a smooth monotone taper to
  a negligible boundary bucket.
  - **Refutes the prior structural guess** that "`ax=1` is the single most
    expensive bucket, cost monotone-decreasing in `ax`." Slack
    `s = n − 8·ax` correctly predicts only the *right-tail taper* (high `ax`
    → low slack → cheap); it does **not** explain the low end (slack is huge
    for all of `ax = 1…5`). The real driver is #slices whose canonical
    minimal-x-axis cell is `(ax,0)`, which itself peaks past `ax=1`. Do not
    re-assert the `ax=1`-spike / monotone story — it is measured false.
  - **Cross-validated against the proven boundary closed form:** boundary
    buckets reproduced `2^(n/8−1)` exactly (n=48→32, 56→64, 80→512,
    88→1024); n=84 (`≡4 mod 8`, slack-4 boundary) gave 768 ≠ 512,
    empirically confirming the `s=0` vs `s=4` distinction. Instrumentation
    reproduces a proven result ⇒ the timing is trustworthy (tier-1).
  - **Re-rates the `rayon`-over-buckets lever upward.** The earlier worry
    that one-bucket-per-task is badly load-imbalanced ("one giant `ax=1`
    task") is **also measured false**: the heavy buckets `ax = 1…6` are
    within ~1.5× of each other — a well-balanced set of comparable tasks
    plus a cheap tail. Naive bucket-parallelism balances *better* than
    feared; the §4.6 "≈#cores wall-clock" lever is more attractive, not
    less. (Modest-`n` measurement: shape qualitative; absolute times tiny.)

- **Cell-vs-vertex per-bucket: the ≈87/13 split is a slice-*count* gap, not
  a per-slice-cost gap.** Same-run measurement at **n=88** (`A142886_BUCKET_TIMING`,
  release): Cell Σ = 50 638 µs over 1 032 245 slices; Vertex Σ = 7 331 µs
  over 118 248 slices ⇒ **6.9× (87.4% / 12.6%)**, reproducing the n=116
  ≈87/13 figure per-bucket. Decisive datum: **µs-per-slice is essentially
  equal** (cell ≈0.049, vertex ≈0.062 — same order). Cell does *not* do
  harder work per slice; it has **~8.7× more slices** (1.03M vs 118k). Root
  cause is the weight structure: vertex cells are weight-8-dominated ⇒
  `|S| ≈ n/8` vs up to `n/4` for cell ⇒ exponentially smaller Redelmeier
  tree. `time ∝ slice-count` holds for *both* centers. Corollary: any lever
  that shrinks per-slice cost helps both centers equally and does not change
  the 87/13 split — only changing slice *count* (or skipping cell residues)
  moves it.
  - **Structural ax-offset now visible in data.** Vertex `ax=0` is a live,
    substantial bucket (338 µs / 7 885 slices — the 2×2-core / self-
    satisfying corner seed); cell has *no* `ax=0` run (apex_forbidden) but
    gains an `ax=11` boundary bucket. Bucket ranges offset by one — the
    "vertex keeps the corner seed cell discards" point, empirically.
  - **Shape differs.** Cell: fast rise to a broad `ax=1…5` plateau, early
    peak `ax≈2`. Vertex: gentler, more symmetric hump centred `ax≈4–5`.
    Both taper to a negligible boundary.
  - **Cross-val.** Cell `ax=11` = 1024 = `2^(88/8−1)` (proven closed form,
    reproduced). Vertex `ax=10` = 256 = 2⁸ but with vertex slack `s=4`
    (vertex `s=0` needs `n ≡ 4 mod 8`, not n=88), so *not* the cell-style
    `2^(ax−1)` — consistent with "vertex boundary: same style, different
    constant," still not derived.

- **`|S|` (slice cell-count) per bucket: valid slices pile at MINIMUM
  `|S|`, not maximum — refutes the "sparse/edge-heavy dominates" guess.**
  Measured with env-gated per-bucket `|S|` histogram (`A142886_SLEN_HIST`,
  release; counts verified unchanged via `--verify`). For **n=88** the
  feasible `|S|` range is `[12, 22]` (12 = 10 weight-8 interior + 2 weight-4
  edge; 22 = all weight-4). Cell: ax1 mean `|S|≈14.7` (peak 14; the all-edge
  `|S|=22` extreme has count **1**), ax5 mean `≈13.2`, ax11 (s=0 boundary)
  **every** slice exactly `|S|=12`. Vertex even tighter: ax1 84 % at
  `|S|=12`, ax5 96 %.
  - **Refutes the prior reasoned claim** ("by lattice-animal entropy the
    typical valid slice is sparse/edge-heavy / large `|S|`"). Measured the
    opposite: mass sits against the *minimum* `|S|` — slices are
    **interior/weight-8-dominated with a minimal edge skeleton**, not
    edge-hugging. Flaw in the old argument: extra cells beyond the minimum
    must be weight-4 *edge* cells, confined to two 1-D boundary lines
    (`O(n)`) vs `O(n²)` interior, so edge-heavy configs are geometrically
    constrained and *rare*; entropy is the 2-D interior arrangement, reached
    with *few* heavy cells. Tier-1 measurement overturns tier-3 reasoning —
    do not re-assert the sparse-dominates story.
  - **Clean structure:** slack ↔ `|S|`-spread. More slack (low `ax`) ⇒
    wider `|S|` distribution, slightly higher mean, tail toward 22; `s=0`
    boundary ⇒ a single cell-count `|S| = ax+1` (the 1024 forced
    staircases, n=88 ax=11, all exactly 12 cells — direct structural
    confirmation of the closed form). Vertex more `|S|`-concentrated than
    cell because it lacks the weight-4 x-axis class (only the diagonal line
    offers weight-4), so less freedom to vary the interior/edge mix.
  - **Caveat (now resolved — see next bullet):** `|S|` measures
    interior-vs-edge *cell economy*, not the compactness/ramification of the
    slice *shape*; that needed a separate metric — measured below.

- **Slice *shape* is ramified/tree-like, not compact — confirms the
  lattice-animal intuition by direct measurement.** Per-bucket compactness
  recorded alongside `|S|` (same `A142886_SLEN_HIST` gate; counts verified
  unchanged via `--verify`; `cx` totals = `sh` totals per bucket ⇒ fires on
  exactly the accepted slices). Metrics: `excess = bonds − (|S|−1)` (0 ⇒
  tree/maximally ramified; higher ⇒ more cycles/compact) and bbox-fill
  `|S|/(W·H)` (→1 ⇒ solid). **n=88:**
  - Cell ax1: meanfill **0.469**, excess 0:41% 1:35% 2:17% (≤2: ~92%),
    ≥5: <0.3%. ax5: fill 0.452, 48% trees. ax11 (s=0 boundary): **100%
    excess-0**, fill 0.308 (the forced staircase *is* a path/tree — sound-
    ness check on the metric). Vertex ≈ identical (ax1 fill 0.456, 45%
    trees; boundary all trees).
  - **Verdict:** ~41–48% of slices are *exact trees*, ~92% have ≤2
    independent cycles, bbox-fill ≈0.45–0.47 (occupy <½ their bounding
    box). Dense/solid slices (high excess, fill→1) are vanishingly rare.
  - **Reconciles the two "sparse" senses.** The pre-measurement claim
    "slices are sparse not dense" was right in the *shape* sense (ramified —
    now directly confirmed) but its *mechanism* (edge-hugging large-`|S|`
    filaments) was wrong (the `|S|` bullet: slices are small/interior-heavy).
    Full picture: **small, interior-weight-8-dominated, tree-like** objects
    — neither solid blocks nor edge filaments.
  - **Structure / center-independence:** slack ↔ shape — tree-fraction rises
    and fill falls monotonically as slack drops (ax1 41 %→ax5 48 %→boundary
    100 %); zero slack forces a pure path. Cell vs vertex within ~1 % on
    both metrics ⇒ ramification is a generic connected-slice (entropy)
    property, not a center artifact. Tier-1 measured.

- **When §4.1 is first satisfied: a slack-gradient readout, plus a vertex
  parity LAW.** Recorded `g = n − (weight at first diagonal touch)` per
  accepted slice (same `A142886_SLEN_HIST` gate; `--verify` OK). x-axis is
  always satisfied at the seed, so `g` measures the diagonal touch; `g=0` ⇒
  satisfied by the final cell ("at the end"), large `g` ⇒ satisfied early.
  - **Cell-centered:** monotone in `ax` — `%` "at the end" (n=88): ax1
    **4.4%**, ax3 11.7%, ax5 29.4%, ax8 67.1%, ax11 (s=0) **100%**. The
    ax1 histogram piles at g=76/80 ⇒ ~61% touch the diagonal at weight
    8–12 (first 1–2 cells). Mechanism: `ax` = seed gap = distance to the
    diagonal; low `ax`+high slack ⇒ dart to the diagonal immediately then
    grow the bulk; `s=0` ⇒ whole budget is the forced gap-descent so the
    diagonal is reached *exactly* by the last cell (100%). Just the slack
    gradient from another angle.
  - **Vertex-centered: 0 % "at the end", EVERY bucket — a parity law, not
    statistics.** Vertex weights are 8 (all) and 4 (diagonal `x=y` only),
    so `8i+4d=n` ⇒ for `n=88`, `d` (diagonal-cell count) is **even** (≥2).
    A finishing first-diagonal cell would mean `d=1` (odd) — impossible.
    So vertex **structurally cannot** satisfy §4.1 with its last cell; the
    diagonal is always entered strictly before completion. (Cell-centered
    has no such obstruction: one finishing diagonal cell gives `e=2` edge
    cells, even — allowed.) Holds for all `n ≡ 0 (mod 8)`; the residue
    arithmetic generalises (vertex `d` parity is fixed by `n mod 8`).
  - Net: weighted by bucket size (low-`ax` dominate) the large majority of
    cell slices satisfy §4.1 almost immediately; "at the end" is the
    high-`ax`/low-slack/few-slice tail, total only at the closed-form
    boundary. Vertex satisfies early always, by the even-`d` parity.

## Resolved / evaluated-and-rejected (do not re-propose)

- **Transfer matrix for n ≫ 110 (M6) — BUILT, PROFILED, REJECTED.** The
  hypothesis that the transfer/kernel rejection below was merely
  *depth-conditioned* (n ≲ 110) and would invert for the n ≫ 110 / full-b-file
  regime was tested by building the engine in full (anti-diagonal Jensen DP:
  scaffolding → weight knapsack → connectivity-signature DP with non-crossing
  partition + sole-completion retirement) and measuring it. It was
  **byte-identical to the §4.6 engine for all feasible n ≤ 57** and matched
  `reconstruct_then_bfs` independently — *correctness was never the problem*.
  Attribution profile (cell-centered, release): runtime/finalize-leaves
  ≈ 3.0×/+4 (≈1.32ⁿ); **distinct frontier states ≈ 2.3×/+4 (≈1.23ⁿ)** vs
  output `a(n)` and the shipped §4.6 enumerator ≈ 2.0×/+4 (≈1.19ⁿ); ≈10³×
  slower than the enumerator at n=48 with the gap widening. The
  **state-count floor itself (≈1.23ⁿ)** — the bound an idealized cell-by-cell
  rewrite cannot beat — is asymptotically *worse* than the §4.6 enumerator
  (≈1.19ⁿ): (1.23/1.19)ⁿ ≈ 1.03ⁿ, ≈10²–10³× more work by n=160 and diverging.
  The D₈-wedge frontier (width ≈ n/4 × non-crossing partitions × per-component
  edge bits × exact weight) does **not** collapse to output size. So the §4.5
  rejection **extends to n ≫ 110**; the b-file's depth is the §3.3
  composition of *siblings* (transfer matrices on far smaller fundamental
  regions), not a transfer matrix on A142886's own wedge. Built behind a pure
  `git mv` split, reverted cleanly to the M5 state on NO-GO. Confirms the
  closed-ledger conclusion with hard numbers: **more terms = more compute,
  not a transfer-matrix algorithm swap. Do not re-propose** (including
  "but a cell-by-cell rewrite would fix it" — the profiled state floor refutes
  that). DESIGN §4.7 / PLAN M6 carry the same record.

- **Post-§4.1-satisfaction `grow<const SAT: bool>` split — shipped (~4%).**
  §4.1 (touch both wedge edges) is monotone (`tx`,`td` only grow), so once
  `tx>0 && td>0` the `edge_reach_lb` prune is provably inert (returns 0) and
  the `ntx>0&&ntd>0` accept gate is always true for the whole subtree. `grow`
  is specialized on `const SAT`: `SAT=true` folds out `edge_reach_lb`, the
  per-cell edge-class tests, and the `tx/td/min_y/min_gap` upkeep, accepting
  unconditionally — one source body, monomorphized to two paths, zero runtime
  dispatch; frontier/`blocked` discipline byte-identical so counts unchanged.
  **Measured (release, controlled A/B):** `a(n)` byte-identical to the b-file
  n=0..120; **~4% faster, non-eroding** (≈3.4% n=80 → ~4% n=96–112, rising
  slightly with n because deeper post-satisfaction subtrees amortize better —
  the `gw` measured profile showed §4.1 is satisfied very early for the
  dominant low-`ax` buckets, so most nodes run the stripped path). **Small-n
  caveat:** for n≲68 the workload is dominated by fixed overhead and the
  result is within noise (one point came out slightly negative — plausibly
  the dispatch branch not amortized by shallow satisfied subtrees, not
  reliably measurable by wall-clock). Net: a win exactly where runtime
  matters, neutral where it doesn't. The const-generic form measured
  identical to a hand-written two-function version, so it carries the gain
  with **no code duplication** (one `const SAT` parameter). Tier-1 measured.
  - **CORRECTION (tier-1 measured, independent sampling profiler).** The
    *"rises slightly with n because deeper post-satisfaction subtrees
    amortize better"* clause is **directionally right but plateaus — it does
    not extrapolate to a majority**, and the underlying "most nodes ⇒ most
    time" leap is false. Method: shipped release binary (LTO, `cgu=1`) built
    `-C symbol-mangling-version=v0` so the two monomorphizations are distinct
    symbols (`…enumerate4growKb0_` = `grow::<false>` = pre-§4.1,
    `…growKb1_` = `grow::<true>` = post-§4.1); macOS `sample(1)` at 1 ms,
    self-time (top-of-stack — the correct metric for a recursive fn),
    ~18k grow samples/point, `count(n)` looped to steady state. **Measured
    post-§4.1 share of grow CPU time:** n=44 32.8%, 52 37.6%, 60 38.8%,
    68 40.5%, 76 44.0%, 84 44.8%, 88 45.4%, 92 45.3%, 96 46.9%, 100 46.0%
    (`grow` is 90.8% of total at n=44 — the small-n fixed-overhead caveat,
    quantified — rising to 100% by n≥88). So the share **rises steeply
    through the mid-range then saturates at ~46% for n≥84** — a stable
    *minority*; it never approaches or crosses 50%. Consequences: (1) the
    ~4% gain's growth with n is real only in the n≈80–112 window the A/B
    used and should **plateau, not keep widening** — the optimized fraction
    stops growing by n≈85; (2) the `gw` "§4.1 satisfied very early ⇒ most
    nodes run the stripped path" observation is about *node count*, and
    does **not** imply most *time* — pre-§4.1 stays the larger time sink
    (~54%) at every n=44..100 even though post-SAT dominates by node count.
    State the claim as: *post-§4.1 is a large, n-saturating ~46% minority of
    grow time*, not "most." (Throwaway `examples/prof88.rs` driver since
    removed; rebuild from this note to reproduce.)
  - **Candidate single-core levers for the post-§4.1 body (B, C) — not yet
    evaluated.** Recorded so they are not lost; exact / count-preserving by
    construction. Bounded by the measured ~46% post-§4.1 time share (and by
    Amdahl on the pre-§4.1 ~54%), so each is at best a low-single-digit
    constant-factor win that stacks with the others — not a growth-class
    change (the §4.5/§4.7 floor stands; A — `rayon` over independent
    post-§4.1 subtrees — remains the open parallel lever, deferred per
    "single-core only for now").
    - **(B) Constant-factor work inside `grow::<true>`.** It is ~46% of
      runtime, ~100% self-time, zero allocation — the cleanest micro-arch
      target left after the const-generic fold. Ideas: grow boundary-first
      to minimize the live `untried` working set (smaller hot bitset ⇒
      fewer `in_untried`/`blocked` probes, better cache residency);
      branchless membership; cache-line-packed `CellSet`; hoist the
      `forbidden`/`xmax`/`in_wedge` invariants out of the `NEIGHBOURS`
      loop. Byte-identical (pure lowering). Expect low single digits;
      measure A/B like the §4.6 entries.
    - **(C) Analytic short-circuit of exactly-solvable sub-buckets —
      measured; broader than "one bucket", but a diminishing factor.**
      The proven `s=0` boundary closed form `2^(n/8−1)` is the degenerate
      end of a *family*: bucket count is a closed form in `ax` for **each
      fixed slack** `s = n − 8·ax` (these buckets are 1-D staircases + an
      O(`s`) local perturbation ⇒ a bounded-width transfer matrix
      legitimately applies — the opposite end of the slack axis from E's
      2-D blob). Empirically (per-bucket exact counts, `A142886_BUCKET`,
      `--verify OK`, n=80/88/96): `s=0` → `2^(ax−1)` (proven); `s=8` →
      `5·ax·2^(ax−3)+1` (fits all three n exactly); `s=16` → degree-2
      poly·`2^ax` (consistent). `deg(P_s) ≈ s/8`. **Payoff (cumulative
      node-visits by slack):** capturing `s≤32` (a ≤~5-state matrix) is
      **14.4% at n=88, 20.9% n=80, 9.8% n=96**; `s≤56` ≈ 31–45%. So the
      old "very small fraction / one bucket" guess is **wrong at computed
      n** for any non-trivial cutoff `S*` (a real exact count-preserving
      ~1.1–1.2× at `s≤32`), but **directionally right asymptotically**:
      the `s≤S*` share **shrinks ≈0.7×/+8 in n** (mass migrates to the
      high-slack low-`ax` 2-D core) ⇒ a *diminishing* constant factor, no
      growth-class change. **Synthesis: C and E are one object along the
      slack axis** — small `s` = bounded-width, closed-form, correct/easy
      but cheap & vanishing (C); `s→n` (low `ax`) = unbounded-width 2-D,
      the ≈1.23ⁿ §4.7 floor (E). No clean cutoff; `S*` trades derivation
      effort for a shrinking work slice. Verdict: a legitimate exact lever
      worth ~10–20% **only at the n-range we actually compute** and only
      if a slack-parameterised recurrence is derived (s=0/8 done above;
      higher `s` is real algebra, not enumeration). Instrumentation
      reverted; numbers reproducible (`A142886_BUCKET`).
    - **(D) Post-§4.1 residual-budget feasibility prune — evaluated, NO-GO
      (tier-1 measured).** Hypothesis: since §4.6's `edge_reach_lb` is folded
      out in `SAT=true`, a cheap admissible prune of dead post-§4.1 subtrees
      (no weight-exactly-`n` completion) might recover work. Measured the
      *ceiling* first (project discipline, like §4.7's state-floor): throwaway
      instrumentation on `grow::<true>` tagging each node dead iff its subtree
      adds 0, and sizing every *maximal* dead root (dead node whose parent is
      alive / pre-§4.1). Self-checks: `pruned_visits == dead_nodes` exactly
      ∀n, `--verify OK`, `a(n)` byte-identical. Result, n=76..100 (`SAT_CEILING`
      env): **deadFrac ≈ 32–33%, stable in n** — so D is *not* inert (this
      *refutes* the a-priori column-span-style guess that it would be ~0;
      worth having measured). **But the dead mass has no exploitable
      structure:** mean maximal-dead-subtree ≈ **1.3 nodes**; at n=100,
      2,162,603 / 2,174,825 dead roots are ≤8 nodes and **0 exceed 512**.
      The ~32% dead visits are the *shallowest, cheapest* nodes — subtrees
      that self-terminate in ~1 step via the existing `w2≤n` gate + frontier
      exhaustion. So a **free perfect oracle** saves ≈0.3 node-visits per
      fire, and any *admissible* detector must be an ≈O(`n`) connected-
      reachable-weight flood fill (every O(1) box/parity bound is inert —
      the wedge is `xmax=n`, the §4.6 column-span lesson), i.e. detection
      cost ≫ savings and savings ≈ 0. The §4.7 "detection ≥ savings" trap
      in a sharper form. **Do not re-propose** any post-§4.1 deadness prune
      (the dead subtrees are individually ~1 node — there is nothing fat to
      lop). Instrumentation reverted; numbers reproducible from this note.
    - **(F) Phase-split + drop-the-blocked-set "core reuse" — evaluated,
      NO-GO (tier-1 measured); refines the §4.7 "any DP" generalization.**
      Idea: run all pre-§4.1 search first, then post-§4.1, hoping the
      blocked set's path-dependence (which exists only for global
      injectivity) could be dropped so identical post-§4.1 subproblems
      collapse and are counted once (`total = Σ_{distinct cores C} comp(C)`).
      Decisive datum, ceiling-first: instrument every §4.1-satisfaction
      crossing (false→true + root `td>0`), fingerprint the boundary state
      two ways — **KA = slice cells only** (the drop-blocked hypothesis),
      KB = slice+blocked (contrast) — distinct vs total, n=76..100
      (`SAT_CORES` env; `--verify OK`, `a(n)` byte-identical). Result:
      **KA multiplicity ≡ 1.0000** (n≤96 `distinct == crossings` *exactly*;
      n=100 / KB sub-1.0 deviations are 64-bit fold-hash collisions —
      structurally KB refines KA so real reuse cannot push KB-distinct
      *below* KA-distinct, yet it does ⇒ noise). I.e. **every §4.1-boundary
      slice is already pairwise-distinct**: dropping the blocked set
      coalesces nothing. This is the Redelmeier exactly-once property
      (each connected partial slice containing the seed is one recursion
      node), empirically confirmed to survive the §4.6 x-axis bucketing.
      So the recursion already shares every common prefix and visits each
      node once; post-§4.1 subtrees of distinct boundary nodes enumerate
      *disjoint* slice sets — there is no duplicate work for a phase-split
      to remove. Any reuse needs a *coarser* key than the (already-unique)
      slice — but a coarser key merges boundary states with *different*
      `comp`, which is unsound unless it is exactly the frontier-state DP
      abstraction whose distinct-state floor §4.7 measured at ≈1.23ⁿ >
      output ≈1.19ⁿ. **The user's idea, fully unpacked, reduces to E and
      is closed by the §4.7 floor — now with the added empirical fact that
      the finest (slice-level) key already has zero reuse.** Caveat logged
      per "FUTURE.md is not proof": §4.7 only *measured* the anti-diagonal
      cut; this probe supplies the missing measurement for the slice-level
      cut (mult ≡ 1) so the "any DP" closure is now empirically anchored at
      both ends, not asserted. Instrumentation reverted.
    - **(G) Tail-fold the R=4 level (Redelmeier "last-cell") — SHIPPED
      (~8%, count-preserving).** Profile (`SAT_RHIST`, n=80/88/96, stable
      in n): **R=4 = 50.4% of all post-§4.1 node-visits with 0.00%
      child-spawns** — pure non-recursive leaf-loops (a `grow()` call +
      `hi`/`blk_base`/`blk_unwind` frame wrapping a loop that just counts
      weight-4 frontier cells). **Implemented:** at the SAT recursive call
      site, when the child's remaining budget `n − w2 == 4`, skip the
      `grow::<true>` call and instead `*total += #{k in pos+1..untried.len()
      : orbit_size(center, untried[k]) == 4}`. **Provably count-identical**
      — every fresh frontier cell is weight ∈ {4,8} (the only weight-1
      cell, the apex, is the seed or `forbidden`, never a fresh frontier
      cell); a weight-4 cell completes (`w2'==n`, accepted unconditionally
      under SAT), a weight-8 overshoots and is skipped, so the child never
      recurses, never appends to `untried`, and its `blocked` inserts are
      fully self-unwound and never read ⇒ its whole contribution is exactly
      that frontier weight-4 count. **Measured (release, controlled A/B,
      best-of-3/5):** `a(n)` **byte-identical to pre-G `main` for
      n=0..120**; `--verify OK` (n≤68 vs b-file/`REFERENCE`); tests
      **10/10**. **~8% faster, non-eroding:** ratio (G / pre-G) 0.912
      (n=80), 0.926 (88), 0.921 (96), 0.920 (100), 0.919 (104) — no
      small-n regression, ≈2× the const-generic split's ~4% as expected
      (it removes call/frame overhead for ~half of all post-§4.1 nodes).
      **Scope/limits:** folded only the SAT (`grow::<true>`) R=4 child —
      the post-§4.1 scope that was measured; the pre-§4.1 (`grow::<false>`)
      R=4 child and the negligible root-bucket R=4 case are intentionally
      untouched. Recursion-overhead amortization, *not* algorithmic: the
      expensive neighbour-expansion (82% at `R≥12`) is untouched and the
      R≤8 frontier work is relocated, not removed — a B-class constant
      factor that stacks with B/I, not a growth-class change. The R=8
      extension (cum. 74.4% of nodes, but only 18.3% of child-spawns +
      the rare connected-weight-4-pair correctness burden) remains a
      smaller, riskier follow-on, not done.
    - **(I) Recursion → explicit-stack iterative DFS — candidate,
      B-family, not yet built.** The sampling profile is ~46% of total
      time, ~100% self-time, in a deeply recursive `grow`. Replacing the
      recursion with an explicit work-stack removes the per-node call
      prologue/epilogue and the `hi`/`blk_base`/`blk_unwind` frame setup,
      and typically improves register allocation. Mechanical, **no count
      risk** (structure-preserving transform), exact; a B-class constant
      factor that **stacks with B and G**. Not a growth-class change.
      Safe to do; measure A/B like the §4.6 entries. (Standard Redelmeier
      implementation technique; complements G — G removes ~half the
      *nodes*, I removes the *per-node* recursion cost of the rest.)
    - **(H) Local-window endgame table — measured, NO-GO (premise
      *confirmed* but no bounded table + memo is the §4.7 trap).** G
      short-circuits the last level (R=4); H asked whether the bottom
      layers' completion count is reusable via a *local windowed* key
      even though F killed the *global* slice key (mult ≡ 1.0). Probe
      (`SAT_WIN`, post-G, n=80/88/96, a(n) byte-identical, `--verify
      OK`): a **sound** sufficient-statistic key — the labelled box of
      radius ⌈R/4⌉+1 around the frontier (per-cell weight-class +
      slice/blocked/forbidden/wall; translation- & order-invariant given
      the labels). **The distinguishing premise is TRUE:** unlike F's
      global mult ≡ 1.0, the windowed key *repeats* — R=8 mult **1.9 / 2.3
      / 2.7** (n=80/88/96, rising in n); R=12 mult 1.3 / 1.4 / 1.6. So
      F-vs-H was a real distinction, validated. **But it still does not
      pay, two independent ways:** (1) *No finite table* — `distinct`
      ≈ 0.4 × node-count and grows **exponentially** in n (R=8: 36k →
      120k → 396k), so the windowed state space is not bounded; the
      "precompute a fixed endgame table, O(1) lookup" form is dead.
      (2) *Runtime memo = the §4.7 trap, localised* — post-G an R=8
      node's own work is tiny (count weight-8 frontier cells + the
      G-folded weight-4 handling, O(few)), whereas computing the window
      key is a radius-3 box scan + hash (≈30–100 cells) — **the key
      costs ≥ the work it would memoise**, paid on 100% of nodes to cache
      ~56% repeats (mult 2–3 ≪ break-even). Net loss, same shape as
      §4.7/D. The realistic per-node-overhead goal is again **I-
      dominated** (mechanical, no key, no map, zero count risk; the
      cross-window-`blocked` correctness obligation also never has to be
      discharged). **Do not build H.** Throwaway probe reverted; numbers
      reproducible (`SAT_WIN`).
    - **(J) Forced-move chaining (unit-propagation analog) — measured,
      NO-GO (premise refuted; I-dominated).** Idea: skip nodes whose
      frontier offers exactly one budget-feasible continuation and chain
      the forced cell, hoping to collapse "rigid staircase" segments.
      Measured post-G (`SAT_FORCED`, n=80/88/96, a(n) byte-identical,
      `--verify OK`) — per-SAT-node census of `live = #{frontier cells
      with w+orbit ≤ n}`: **the structural premise is false.** Branching
      is genuinely spread — `live = 1/2/3` are each ≈23–28%; the `s=0`
      staircase is *binary* (up/left), a binary tree, **not a forced
      tunnel**. Only ≈27% of nodes are forced at all; the *chainable*
      subset (forced-**recursive**: the lone live cell recurses) is just
      **≈18–20%, declining with n** (19.9→18.9→18.0% at n=80/88/96),
      vs the dead ≈4% and forced-complete ≈8% (terminal, not chainable).
      And it is overhead-only with a real correctness burden: a chained
      node still must replay its `blocked` side-effects (it blocks its
      overshoot/other frontier cells, which propagate into the child's
      canonical generation — *unlike* G's R=4 case where `blocked` is
      provably never read), so chaining removes only the frame/call
      (~19% of nodes), not the frontier scan or the blocking, and needs
      a two-state correctness proof. **Lever I (iterative DFS) removes
      per-node frame/call overhead for 100% of nodes, mechanically, with
      zero count risk — it strictly dominates J's entire purpose** (as it
      does the R=8 fold). Do not pursue J; do **I** instead. Throwaway
      instrumentation reverted; numbers reproducible (`SAT_FORCED`).
    - **(K) Revive Redelmeier index-canonicalisation (drop the explicit
      `blocked` set) — measured ceiling LARGE, but design-gated; the top
      unexplored single-core prize.** The classic Redelmeier uniqueness
      rule is an O(1) cell-index test, *not* an explicit excluded-set with
      push/unwind. This codebase **deliberately traded that away** for the
      A-rooted x-axis-bucketed scheme (`enumerate.rs` docstring: "the
      baseline's global lex-min `nb > seed` shortcut is deliberately
      dropped (no cheap analog for an edge-pinned root; known performance
      trade — plan O2)"). That rationale is *asserted, not proven* — the
      one standard Redelmeier optimisation not on this ledger. **Measured
      cost of the bookkeeping it reintroduced** (the three blocked-only
      ops isolated behind `#[inline(never)]`, profiled with the v0-symbol
      + `sample` workflow, n=88, byte-identical to `main` n=0..120,
      `--verify OK`, ~100% attributed / trustworthy): self-time of
      `blk_mark` 12.5% + `blk_unwind_level` 9.5% + `blk_contains` 5.6%
      = **≤ 27.6% of total runtime**. **Upper bound** — the inline
      barrier inflates these tiny hot bitset ops; true cost realistically
      ~15–20%, but decisively **not negligible and larger than any single
      lever found this session** (G ≈ 8%). The per-node teardown
      (`blk_unwind_level` ≈ 9.5% upper) also partly bounds lever I.
      **Status:** not a shippable delta — `blocked` is load-bearing for
      correctness; capturing this prize requires solving the *open design
      question* the docstring shelved: does a cheap O(1) canonical-index
      rule exist that is compatible with minimal-x-axis-cell bucketing
      (so each slice still lands in exactly one bucket, counted once,
      without the excluded-set)? That is an algorithm-design problem, not
      a measurement — but the payoff ceiling now justifies spending real
      design effort on it. Pairs with I (both attack per-node
      bookkeeping; together they'd compound). Throwaway helpers + driver
      reverted; reproducible via this note.
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
  - **CORRECTION (tier-1 measured, supersedes the two claims above).** "The
    single-core ledger is closed" and "per-node cost is irreducible inlined
    work" are **empirically false by ~4%**. The closed-ledger reasoning
    *asserted* per-node irreducibility without measuring the post-§4.1-
    satisfaction specialization (shipped above): once §4.1 is satisfied —
    which the `gw` profile shows happens very early for the dominant buckets
    — `edge_reach_lb` + the `tx/td/min_y/min_gap` upkeep are pure overhead,
    and removing them is a measured ~4% single-core, count-preserving,
    non-eroding gain. So the ledger was *not* closed; a single-core lever
    existed and its absence was assumed, not proven. Do not re-cite "ledger
    closed / per-node irreducible" as settled — the bound on *#nodes* still
    holds, but *per-node cost* was reducible. (`rayon`-over-buckets remains a
    separate, still-open parallel lever.)
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

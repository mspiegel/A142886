# DESIGN: Computing OEIS A142886

## 1. Problem statement

**A142886** — *"Number of polyominoes with n cells that have the symmetry
group D₈."*

> **Notation hazard, read this first.** OEIS follows Sloane in writing `D_8`
> for the **dihedral group of order 8** — the full symmetry group of the
> square: the identity, three rotations (90°, 180°, 270°), and four
> reflections (horizontal, vertical, and the two diagonals). Most geometry
> texts call this group **D₄** (indexed by the 4-fold rotation). They are the
> same group; this document writes **D₈** to match OEIS. It is the *maximal*
> symmetry group any polyomino can have, which simplifies the algorithm
> (§4.4): there is no larger group to exclude.

A polyomino is counted iff its symmetry group is exactly D₈ (equivalently,
since D₈ is maximal, iff it is *at least* D₈-symmetric).

### Reference data (offset 0, from oeis.org/A142886)

```
n :  0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19 20 ...
a :  1  1  0  0  1  1  0  0  1  2  0  0  3  2  0  0  5  4  0  0 12 ...
```

Full known prefix used for testing (n = 0..68):

```
1,1,0,0,1,1,0,0,1,2,0,0,3,2,0,0,5,4,0,0,12,7,0,0,20,11,0,0,45,20,0,0,
80,36,0,0,173,65,0,0,310,117,0,0,664,216,0,0,1210,396,0,0,2570,736,0,0,
4728,1369,0,0,9976,2558,0,0,18468,4787,0,0,38840
```

- `a(0) = 1`: the empty polyomino, by OEIS convention.
- `a(1) = 1`: the monomino.
- `a(4) = 1`: the 2×2 square.
- `a(5) = 1`: the X / plus pentomino.
- `a(8) = 1`: the 3×3 ring (3×3 square minus its center).
- `a(9) = 2`: the 3×3 solid square is one of the two.

**Verification oracle:** the b-file `https://oeis.org/A142886/b142886.txt`
gives `n, a(n)` for n = 0..163 (R. A. Russell). This is the target depth
("as far as feasible").

**Primary references:**
- D. H. Redelmeier, *Counting polyominoes: yet another attack*, Discrete
  Math. **36** (1981) 191–203 — Table 3 tabulates polyominoes by symmetry
  class; the algorithmic foundation here.
- T. Oliveira e Silva, *Enumeration of polyominoes*,
  `http://sweet.ua.pt/tos/animals.html`.
- Related OEIS: A351127 (D₈ about a cell center), A346800 (D₈ about a
  vertex), and the symmetry-class family A000105, A006746–A006749, A056877,
  A056878, A144553, A144554.

## 2. Symmetry reduction (formalizing the fundamental-domain insight)

The guiding observation: a D₈-symmetric polyomino is completely determined by
its intersection with **one fundamental domain** of D₈, namely the closed
45° wedge between the positive x-axis and the diagonal `y = x`:

```
W = { (x, y) : 0 ≤ y ≤ x }
```

```
   y
   ^                 /  y = x  (wedge edge, included)
   |               /
   |             /  . . . . . interior of W (0 < y < x)
   |           / . . . . . .
   |         / . . . . . . .
   +-------========================>  x   (positive x-axis, wedge edge, included)
            origin
```

Every D₈-orbit of lattice cells has **exactly one** representative in `W`,
once a tie-break is fixed on the two wedge edges (a cell on the x-axis edge
and a cell on the diagonal edge are each their orbit's unique representative;
the apex cell is its own orbit). Therefore:

> A D₈-symmetric polyomino `P` is in bijection with the set `P ∩ W`. We
> enumerate **occupancy sets within `W`** and reconstruct `P` by applying all
> 8 group elements. Distinct wedge sets give distinct polyominoes, so the
> class is counted **directly** — no Burnside averaging, no canonicalization,
> no deduplication pass.

This is the core efficiency: the wedge holds only ≈ n/8 of the cells.

The 8 group elements, acting on a lattice point `(x, y)` about the origin:

| element | map | element | map |
|---|---|---|---|
| `e`   | `(x, y)`   | `s` (refl. x-axis)   | `(x, -y)` |
| `r`   | `(-y, x)`  | `sr` (refl. y=x)     | `(y, x)`  |
| `r²`  | `(-x, -y)` | `sr²` (refl. y-axis) | `(-x, y)` |
| `r³`  | `(y, -x)`  | `sr³` (refl. y=-x)   | `(-y, -x)`|

## 3. The two symmetry-center placements

The one-line "just use the wedge" argument omits a critical point: **where
the center of symmetry sits relative to the lattice**. D₈ has a *unique*
fixed point (its center), and for it to map the square lattice to itself that
point must be either:

1. the **center of a lattice cell** ("cell-centered"), or
2. a **lattice vertex** — a corner shared by four cells ("vertex-centered").

An edge-midpoint center is impossible: a 90° rotation about an edge midpoint
does not preserve the lattice, so it can yield at most D₂/D₄-rectangular
symmetry, never D₈. These two cases have different wedge geometry and
different orbit-size arithmetic, and must be enumerated separately. A given
polyomino has a unique centroid, which for a D₈-symmetric shape coincides
with the symmetry center; hence **no shape is produced by both cases** (they
are disjoint — see §3.3).

### 3.1 Cell-centered

Put the center cell at `(0, 0)`; cell `(i, j)` has its center at the integer
point `(i, j)`. Orbit sizes of cells with representative in `W`:

| representative | locus | orbit size |
|---|---|---|
| `(0, 0)` | apex (the center cell) | **1** |
| `(x, 0)`, `x > 0` | x-axis edge of `W` | **4** |
| `(x, x)`, `x > 0` | diagonal edge of `W` | **4** |
| `(x, y)`, `0 < y < x` | strict interior of `W` | **8** |

So `n = c + 4·e + 8·i`, where `c ∈ {0, 1}` records whether the center cell
is occupied, `e` = number of edge-orbit representatives (x-axis *or*
diagonal), `i` = number of interior representatives. Thus:

- center **occupied** (`c = 1`)  ⟹  `n ≡ 1 (mod 4)`
- center **empty**    (`c = 0`)  ⟹  `n ≡ 0 (mod 4)`

This case corresponds to OEIS **A351127**.

### 3.2 Vertex-centered

Put the center at the lattice vertex shared by cells `(0,0)`, `(-1,0)`,
`(0,-1)`, `(-1,-1)` (cell `(i,j)` occupies the unit square
`[i,i+1]×[j,j+1]`, so its center is `(i+½, j+½)`). The fundamental domain in
cell indices is `{ (i, j) : i ≥ 0, 0 ≤ j ≤ i }`, i.e. cell centers in `W`.

- The x-axis line through the center runs *along a grid line* — no cell
  center lies on it, so reflection across it has **no fixed cells**.
- The diagonal `y = x` passes through cell centers `(i+½, i+½)`: the
  "diagonal cells" `(i, i)` for `i ≥ 0`. These form size-**4** orbits.
- All other wedge cells have size-**8** orbits.

So `n = 4·d + 8·g`, where `d` = number of diagonal cells `(i,i)` and `g` =
number of strictly-interior cells. The minimal configuration is `d=1, g=0`:
the four innermost cells = the **2×2 square**, `n = 4`. Always
`n ≡ 0 (mod 4)`. This case corresponds to OEIS **A346800**, indexed by
`n/4`.

### 3.3 Consequences (built-in sanity checks)

- `a(n) = 0` for `n ≡ 2, 3 (mod 4)` — falls out of the arithmetic above; no
  configuration of any kind produces those sizes.
- For `n ≡ 1 (mod 4)`: only cell-centered (center occupied) ⟹
  `a(n) = A351127(n)`.
- For `n ≡ 0 (mod 4)`, `n > 0`: cell-centered-empty **plus** vertex-centered
  ⟹ `a(n) = A351127(n) + A346800(n/4)`.
- Cell-centered shapes have their center at a cell-center *point*;
  vertex-centered shapes at a *vertex point*. A shape's centroid is unique,
  so the two enumerations are disjoint and **summed without deduplication**.
- `n = 0`: the empty polyomino, `a(0) = 1` by convention; handled as an
  explicit base case, not produced by enumeration.

These exactly reproduce John Mason's OEIS formula
`a(n) = A351127(n) + A346800(n/4)` (4 | n) else `a(n) = A351127(n)`.

### 3.4 Worked examples (hand-derivable, and the basis for §7(b))

- **n = 1:** cell-centered, `c=1, e=0, i=0` → the monomino. `a(1)=1`.
- **n = 4:** vertex-centered, `d=1, g=0` → the 2×2 square. Cell-centered with
  empty center and `4e=4` gives only disconnected sets (four cells with no
  common edge), so contributes nothing. `a(4)=1`.
- **n = 5:** cell-centered, `c=1`, one x-axis edge orbit at distance 1 →
  the plus/X pentomino. The diagonal-orbit alternative is disconnected from
  the center. `a(5)=1`.
- **n = 8:** cell-centered, center empty, one x-axis orbit (dist 1) + one
  diagonal orbit (dist 1) → the **3×3 ring**. Vertex-centered `n/4=2`
  candidates are all disconnected. `a(8)=1`.
- **n = 9:** cell-centered, `c=1`. Two distinct connected shapes:
  (x-axis orbit d1) + (diagonal orbit d1) + center = **3×3 solid square**;
  (x-axis orbit d1) + (x-axis orbit d2) + center = **plus with arm length
  2**. `a(9)=2`.

The n=9 case in particular validates the orbit model: the arithmetic *and*
the connectivity filter must both be correct to get exactly 2.

## 4. Enumeration algorithm

### 4.1 Connectivity is a slice-local predicate

A polyomino must be a single edge-connected (4-neighbour) region. This does
**not** require reconstructing `P` and running a graph search on it. By
symmetry it reduces to a check on the wedge slice alone.

**Lemma.** Let `S = P ∩ W` be the occupied wedge cells (including the
apex / 2×2-core cell). `P` is a connected polyomino **iff**

1. `S` is a single 4-connected component, **and**
2. `S` contains an occupied cell on the **x-axis edge** of `W` *and* an
   occupied cell on the **diagonal edge** `y = x` (the apex / 2×2-core,
   when present, lies on both edges simultaneously and satisfies this on
   its own; the pinned root `A` always lies on the x-axis edge).

**Sufficiency.** The 8 group images of `W` tile the plane as 8 octants in a
cycle about the origin; consecutive octants share either an x-axis-type ray
or a diagonal-type ray. An occupied x-axis-edge cell is edge-adjacent to its
reflected copy across that ray — cell-centered: the cell lies *on* the mirror
line `y=0`; vertex-centered: the bottom-row cell `(a,0)` shares the unit edge
`y=0` with its image `(a,-1)`. So the `W`-copy links to its x-axis-neighbour
octant; likewise the diagonal-edge cell links it to the diagonal-neighbour
octant. Each octant copy is internally connected (an isometry of the
connected `S`), so all 8 fuse into one connected cycle ⇒ `P` connected.

**Necessity.** `P` connected ⟹ `S` connected: map any `P`-edge by the group
element carrying one endpoint into `W`; interior cells have all neighbours in
`W`, and a boundary cell's only out-of-`W` neighbour is the mirror of some
occupied `c ∈ W` (occupied because D₈ forces the mirror of an occupied cell
to be occupied) that is edge-adjacent to `c` *within* `W` — so every `P`-edge
projects to a slice edge between occupied representatives. And `P` must place
cells on the axis and diagonal lines for the octants to link around the
origin; those are images of edge cells of `S`, forcing condition 2. If `S`
splits, or touches only one edge type, the octant copies join only in
isolated pairs and `P` is disconnected (e.g. a slice touching only the
diagonal yields 4 disjoint pieces).

Consequently the slices `{(3,1)}` and `{(2,1),(3,1)}` are rejected purely by
condition 2 (they touch neither wedge edge) — no reconstruction needed.
Connectivity is therefore an **O(|S|)** test on the ≈ n/8 wedge cells, not a
graph search over the full n-cell figure. Reconstruction of `P` is used only
as an optional debug assertion (§4.3), never on the hot path.

### 4.2 Baseline: Redelmeier growth in the wedge

Use Redelmeier's recursive cell-growth scheme (the canonical
untried-neighbour enumeration that visits each shape exactly once without
deduplication), adapted to the quotient and **bucketed by the slice's
minimal x-axis cell**:

```
For each center type (cell, vertex):
  For target size n with the right residue mod 4:
    For each bucket = the slice's minimal x-axis cell A = (ax, 0):
      Skip buckets of the dead parity (§4.6 residue split).
      Grow an occupancy set restricted to W, cell by cell, from the
      pinned root A, using Redelmeier's "untried set" frontier with the
      blocked-set (ancestor-exclusion) discipline, so each connected
      slice containing A is generated exactly once.
      Forbid adding any x-axis cell strictly left of A (so A is the
      slice's canonical minimal x-axis cell ⇒ each valid slice falls in
      exactly one bucket).
      When the orbit-size accounting (§3) totals exactly n cells:
        accept iff S is 4-connected AND S touches both wedge edges (§4.1).
        (no reconstruction; the §4.1 lemma makes this exact)
      Prune any branch whose minimal completed size already exceeds n.
      Prune any branch that can no longer touch both wedge edges within
      the remaining weight budget (§4.6 edge-reachability bound).
```

Because distinct wedge sets ↔ distinct symmetric polyominoes (§2), the
running count is exact with no canonical-form pass and no Burnside sum.
The two extra prunes (§4.6) only delete provably-empty subtrees, so the
count is unchanged.

**Why bucket by the minimal x-axis cell.** Every valid slice touches the
x-axis (§4.1), so its minimal x-axis cell `A` is well-defined; the
forbid-left-of-`A` rule makes `A` the slice's unique canonical
representative, and injectivity *within* a bucket is the standard
fixed-root Redelmeier blocked-set theorem (it does **not** need a
lex-ordering shortcut). An earlier scheme instead keyed on the slice's
*global lex-minimum cell* and grew only to strictly-greater cells
(`nb > seed`). That `nb > seed` shortcut has no cheap analog for an
edge-pinned root and is dropped; nevertheless, with the §4.6 prunes the
x-axis-rooted bucketing is **measured ≈2× faster** than the lex-min-seed
scheme at n = 72..104, counts byte-identical (n ≤ 100 vs the prior
scheme; n ≤ 68 vs the b-file and embedded reference). The reason: there
are only `≈n` x-axis roots versus `≈n²/2` lex-min seeds, and the
blocked-set discipline plus the x-axis forbid more than compensate for
the lost `nb > seed` pruning. A two-terminal `(A,B)` variant that also
pins the minimal diagonal cell was evaluated and rejected — see §4.5.

### 4.3 Connectivity test

Hot path: the §4.1 predicate on the slice `S` — (i) `S` is one 4-connected
component (BFS/union–find over the ≈ n/8 occupied wedge cells, `(i32,i32)`),
and (ii) `S` has an occupied cell with `y = 0` and one with `y = x` (the
pinned root `A` gives (i)-half for free; the apex/2×2-core, when present,
satisfies both). O(|S|); no `P` materialized.

Debug only: `reconstruct(S) = ⋃_{g ∈ D₈} g·S`, then assert `|P| == n` and that
a BFS over `P` agrees with the predicate. Gated behind `debug_assertions` /
tests (§7f), never run in the counting loop.

### 4.4 Correctness notes

- **No down-classification.** D₈ is maximal: an accepted shape cannot have a
  *larger* symmetry group, so we never need to subtract over-symmetric
  shapes. We only enforce: (i) exact size `n`, (ii) connectivity, (iii) the
  shape's center is the assumed center (automatic — we build the shape *by*
  symmetrizing about it, and a D₈ set's centroid is its unique symmetry
  center).
- **Disjointness.** Cell- and vertex-centered runs cannot collide (§3.3);
  their counts are simply added.
- **Empty polyomino.** `count(0) = 1` is hard-coded.

### 4.5 Depth limits

Because connectivity is an O(|S|) slice-local predicate (§4.1) on only
≈ n/8 cells — no per-candidate reconstruction or full-figure BFS — the only
remaining cost is the wedge enumeration itself, sharply reduced by the §4.6
prunes. That enumeration is still exponential, so reachable `n` is empirical
(machine- and time-budget-dependent), not a fixed guarantee; this design does
not promise the full b-file (n ≤ 163).

A transfer/kernel reformulation — processing the wedge by diagonals with a
bounded frontier state carrying the connectivity signature — was evaluated as
the route to polynomial-per-term depth. In practice it was **not faster** than
the §4.6-pruned enumerator (its per-node state bookkeeping outweighed the
branching it removed once §4.6 was in place). The hypothesis that this verdict
was merely *depth-conditioned* (and would invert for n ≫ 110, since the b-file
reaches n=163 only as the OEIS composition `a(n)=A351127(n)+A346800(n/4)`,
§3.3) was **built and measured under milestone M6 and rejected with hard
numbers** — see §4.7. The separately-refuted variants below (two-terminal
`(A,B)`, and in §4.6 the column-span bound / xmax cache-cap) remain rejected.

A **two-terminal `(A,B)` variant** was also evaluated: pin *both* the minimal
x-axis cell `A` and the minimal diagonal cell `B = (bx, bx)`, bucket by the
pair `(A, B)`, and accept iff `B ∈ S` (making the §4.1 both-edges condition
hold by construction rather than by a generate-then-reject filter). It is
*correct* (byte-identical n ≤ 68) but **≈O(n) slower**: a slice whose true
minimal-diagonal cell is `(D, D)` is fully grown and then rejected in every
bucket `bx = 0..D-1` and counted only in `bx = D`, an `≈n`-fold redundant
re-exploration that compounds with `n` (profiled ≥ 50× slower at n = 88). It
is rejected; dropping the `B` dimension entirely recovers the shipped
§4.2 x-axis-rooted scheme. The shipped algorithm is the §4.2 Redelmeier
enumeration bucketed by the minimal x-axis cell with the §4.6 prunes; greater
depth comes from running it longer, not from an algorithm swap.

### 4.6 Residue-class bucket split and edge-reachability pruning

Two exact, count-preserving prunes sit on top of the §4.2 baseline. Both
only delete subtrees that provably contribute zero, so every `count(n)` is
byte-identical to §4.2 (regression-checked n = 0..68 against the b-file and
the embedded reference).

**(a) Residue-class bucket split (the "path-vs-ring" decomposition).** From
§3.1 a cell-centered slice has weight `c + 4e + 8i` with `c ∈ {0,1}` the apex
bit, so

```
slice contains the apex (0,0)   ⟺   weight ≡ 1 (mod 4)
slice omits the apex            ⟺   weight ≡ 0 (mod 4)
```

and since `(0,0)` is the only `x = 0` wedge cell, "contains the apex" ⟺
"the slice's minimal x-axis cell is `(0,0)`" ⟺ "the `ax = 0` bucket". For a
fixed target `n` only one parity can ever sum to `n`, so:

- **n ≡ 1 (mod 4) — center occupied ("a path out from the center").**
  Every counted slice contains the apex, hence lies in the `ax = 0` bucket;
  *every other bucket's entire Redelmeier subtree is provably empty* and is
  skipped. The apex lies on both wedge edges, so §4.1 condition 2 is
  satisfied for free — the rest of the slice is unconstrained by it (it may
  touch zero, one, or both edges) and acceptance reduces to "weight == n".
- **n ≡ 0 (mod 4), cell-centered — center empty ("a ring from the x-axis
  edge to the diagonal edge").** No counted slice contains the apex, so the
  `ax = 0` bucket (the single largest one) is skipped; condition 2 now binds
  and is the operative filter.
- **Vertex-centered** has no apex and is always `n ≡ 0`; every bucket is
  live (the `ax = 0` root is the size-4 2×2 core, an ordinary x-axis cell).

These two cases are not merely separable, they are **mutually exclusive per
`n`**: the residue of `n` mod 4 forces exactly one. So there is *no*
"configurations that have both a center path and a ring" inclusion–exclusion
term — it is identically zero. A center-occupied slice may itself wind into
an annulus, but it is still one connected slice counted once by the
minimal-x-axis-cell bucketing; nothing ever adds a separate "ring count" to
correct against. (The cell-empty and vertex-centered counts, both at `n ≡ 0`, are
summed without correction because their centroids differ — §3.3.)

**(b) Edge-reachability bound — single joint cell-budget + gap bound
(admissible prune for the ring case).** Track, over the current slice,
`min_y` (smallest `y`) and `min_gap` (smallest `x − y`). The two excursions
(to `y = 0` and to `x = y`) may share cells, so the combined bound is the
`max` of the two terms, never the sum.

*x-axis term* (moot for the shipped A-rooted scheme — the pinned root is on
the x-axis so it is always touched ⇒ 0; kept for the general lemma): if
`y = 0` is not yet touched, reaching it needs ≥ `min_y` new cells, ≥ 4
weight each.

*Diagonal term.* If the diagonal is not yet touched (`td = 0`, equivalently
`min_gap ≥ 1`), reaching it needs `min_gap` new cells — each 4-neighbour
step changes `x − y` by ≤ 1, so the connecting chain hits every gap value
`min_gap−1, …, 1, 0`. The cheap weight-4 route (run the x-axis to the
origin) is **blocked by the canonical forbidden region**: every x-axis cell
has gap `x ≥ ax ≥ min_gap` or is forbidden (`x < ax`), so no x-axis cell can
*reduce* the gap. Hence each of the `min_gap − 1` gap-reducing cells is
*interior* (orbit weight 8, not 4) and only the gap-0 landing is a weight-4
diagonal cell — the exact (tightest admissible) minimum, for both centers:

```
extra_weight_needed ≥ max( tx? 0 : 4·min_y ,  td? 0 : 8·min_gap − 4 )
```

(For `ax = 0` buckets — cell `n ≡ 1` apex / vertex 2×2 core — the root is on
the diagonal ⇒ `td > 0` ⇒ the diagonal term is 0, unaffected.)

Prune the subtree when `weight + extra_weight_needed > n`. **Soundness is
per-node admissibility, not monotonicity.** The tightened `weight + LB` is
*not* monotone down the recursion (the diagonal term can fall by 8 while
`weight` rises only 4). It does not need to be: any valid weight-`n`
completion must touch the diagonal, so its total `= weight + added ≥
weight + LB`; if `weight + LB > n` *at this node* then every completion
exceeds `n`, so the subtree holds no countable slice and the cut is sound —
independently of any descendant's bound. Non-monotonicity only affects *how
early* a doomed subtree is cut (performance), never correctness.

**Measured effect of §4.6 (release, cumulative `--max-n`).** The speedup
grows with `n` (the prune lowers the effective branching, not just a constant
factor); numbers for the original lex-min-seed §4.2 enumerator:

| n  | §4.2 (un-pruned) | with §4.6 | speedup |
|----|------------------|-----------|---------|
| 56 | 0.83 s           | 0.02 s    | ≈ 40×   |
| 60 | 2.13 s           | 0.03 s    | ≈ 70×   |
| 64 | 4.03 s           | 0.05 s    | ≈ 80×   |
| 68 | 10.44 s          | 0.11 s    | ≈ 95×   |

This is a large constant/branching improvement, not an asymptotic change: the
wedge enumeration is still exponential (§4.5), so this lowers the constant and
pushes the reachable `n` out without changing the growth class.

**Minimal-x-axis-cell bucketing vs the prior lex-min-seed scheme (both with
§4.6, release, cumulative `--max-n`, best of 3).** Switching the §4.2
enumeration scheme (see §4.2 "Why bucket by the minimal x-axis cell") is a
further ≈2×, with a *non-eroding* ratio and byte-identical counts:

| n   | lex-min seed | x-axis bucket | ratio |
|-----|--------------|---------------|-------|
| 72  | 0.03 s       | 0.01 s        | 0.33  |
| 88  | 0.37 s       | 0.17 s        | 0.46  |
| 100 | 2.72 s       | 1.33 s        | 0.49  |
| 104 | 5.29 s       | 2.61 s        | 0.49  |

The full `0..=68` `--ignored` regression sweep is well under 0.1 s. Still
exponential (§4.5) — a constant-factor win, not a growth-class change.

**Joint cell-budget + gap diagonal term vs the prior `4·min_gap` term (both
on the x-axis-bucket scheme, release, cumulative `--max-n`, best of 3).**
Tightening the diagonal term from `4·min_gap` to `8·min_gap − 4` is a
further ≈1.33×, ratio non-eroding, counts byte-identical (n ≤ 100 vs the
prior engine; n ≤ 68 vs b-file / `REFERENCE`):

| n   | `4·min_gap` | joint `8·min_gap−4` | ratio |
|-----|-------------|---------------------|-------|
| 88  | 0.16 s      | 0.12 s              | 0.75  |
| 100 | 1.24 s      | 0.93 s              | 0.75  |
| 104 | 2.42 s      | 1.84 s              | 0.76  |
| 108 | 4.79 s      | 3.64 s              | 0.76  |

Pure node-count reduction at unchanged O(1) per-node cost (profile stays
~100% self-time in `grow`); still exponential (§4.5).

### 4.7 Transfer-matrix enumerator (M6) — evaluated and rejected

A Jensen-style anti-diagonal transfer matrix was built (M6 Phases 1–4) to
test whether the §4.5 rejection was merely *depth-conditioned* and would
invert for n ≫ 110. It was implemented in full (scaffolding → weight DP →
connectivity-signature DP with the non-crossing partition + sole-completion
retirement rule), validated **byte-identical to the §4.2/§4.6 engine for all
feasible n ≤ 57** (and independently against `reconstruct_then_bfs` at small
n), then **profiled and rejected**. Cell-centered measurements (release):

| metric | growth per +4 | base |
|--------|---------------|------|
| runtime / `finalize` leaves | ≈ 3.0× | ≈ **1.32ⁿ** |
| **distinct frontier states** (DP floor) | ≈ 2.3× | ≈ **1.23ⁿ** |
| output `a(n)` and the §4.6 enumerator | ≈ 2.0× | ≈ 1.19ⁿ |

Two findings, both fatal: (i) runtime tracked `finalize` leaves (≈1.32ⁿ) —
the per-anti-diagonal occupancy recursion reintroduced enumeration cost
inside the transfer step; (ii) decisively, the **distinct-state count
itself** — the floor for *any* DP over this state, the quantity an idealized
cell-by-cell rewrite is bounded by — grows ≈1.23ⁿ, strictly *above* both the
output and the shipped enumerator (≈1.19ⁿ). So even a flawless transfer
matrix is asymptotically **worse** than §4.6 here: (1.23/1.19)ⁿ ≈ 1.03ⁿ,
≈10²–10³× more work by n=160 and diverging. The D₈-wedge frontier (width
≈ n/4 × non-crossing partitions × per-component edge bits × exact weight)
does not collapse to output size. The §4.5 rejection therefore **extends to
n ≫ 110**; the b-file's depth comes from the §3.3 composition of *siblings*
(themselves computed by transfer matrices on far smaller fundamental
regions), not from a transfer matrix on A142886's own wedge. The split was
a pure `git mv`, reverted cleanly; numbers recorded in FUTURE.md. This
confirms the closed-ledger conclusion: more terms = more compute, not a
transfer-matrix algorithm swap.

## 5. Rust implementation sketch

Crate layout (the crate itself is a follow-up; this document is the design):

```
A142886/
  Cargo.toml
  src/
    main.rs          # CLI: --max-n N, --center cell|vertex|both, --verify
    symmetry.rs      # the 8 transforms; orbit + representative logic
    enumerate.rs     # Redelmeier growth per center type
    connectivity.rs  # §4.1 slice predicate; reconstruct+BFS (debug only)
    verify.rs        # reference vector + optional b-file comparison
```

- Cell coordinates: `(i32, i32)`. Wedge occupancy: a `HashSet<(i32,i32)>`
  (or a packed bitset once a bounding box is fixed) keyed to `W`.
- Connectivity: §4.1 slice predicate — BFS/union–find over the ≈ n/8
  occupied wedge cells plus the two edge-touch flags. Reconstruction (the 8
  closed-form maps from §2) is compiled only into the debug assertion.
- Term magnitude: values stay well within `u64` for n ≤ 163 (growth is
  far slower than the ~4.06ⁿ of all polyominoes); expose the count type as a
  single alias so a big-integer backend (`num-bigint`) can be swapped in if
  ever needed.
- Parallelism: independent recursion subtrees parallelize cleanly with
  `rayon` (optional, behind a feature flag).
- Public API the tests below assume:
  - `count(n: usize) -> u64` — `a(n)` (sum of both center types).
  - `count_cell_centered(n: usize) -> u64` — A351127-type contribution.
  - `count_vertex_centered(n: usize) -> u64` — A346800-type contribution.

## 6. CLI behaviour

- `a142886 --max-n 40` prints `n a(n)` for n = 0..40.
- `--center cell|vertex|both` restricts/selects the enumeration (for
  cross-checking against A351127 / A346800 individually).
- `--verify` compares output against the embedded reference vector (§7a),
  and against `b142886.txt` if present in the working directory.

## 7. Verification & unit tests

Embed the following as `#[cfg(test)] mod tests` in the crate.
`cargo test` runs (a)–(f); `cargo test -- --ignored` runs the b-file
regression (g).

> **Baseline runtime note (measured, M3; updated for §4.6).** The §4.2
> enumerator visits all connected wedge slices of weight ≤ n, growing
> exponentially: original debug-build cost ≈0.26 s at n=40, ≈1.2 s at n=48,
> ≈17 s at n=60. The §4.6 prunes (count-preserving) cut this ≈40–95×
> (release: n=68 from ≈10 s to ≈0.1 s), switching the §4.2 scheme to
> minimal-x-axis-cell bucketing is a further ≈2×, and the §4.6(b) joint
> cell-budget+gap diagonal term a further ≈1.33× (n=100 ≈0.9 s; the full
> `0..=68` `--ignored` sweep well under 0.1 s) — still exponential, just a
> much lower branching factor. The
> *always-on* `cargo test` checks of the heavy `n ≡ 0,1 (mod 4)` cases remain
> bounded at **n ≤ 40** (constant `HEAVY_BOUND`) and the reference-vector
> always-on tier at `0..=40`; after §4.6 this is a comfortable safety margin
> (it absorbs the slower debug build and any future regression) rather than a
> tight ceiling. The cheap `n ≡ 2,3 (mod 4)` zero cases still cover the full
> `0..120`; the full `0..=68` and the b-file `0..=163` stay the on-demand
> `#[ignore]` deep checks ((a-deep)/(g), milestone M5). The enumeration stays
> exponential (§4.5); reachable depth is empirical, not a fixed target. These
> bounds are a runtime accommodation only — the algorithm is unchanged and
> counts are byte-identical.

**(a) Reference-vector test — known prefix.** The OEIS prefix
(`n = 0..=68`) is the oracle. Per the baseline-runtime note it is checked in
two tiers from the same constant:

```rust
const A142886: [u64; 69] = [
    1,1,0,0,1,1,0,0,1,2,0,0,3,2,0,0,5,4,0,0,12,7,0,0,20,11,0,0,
    45,20,0,0,80,36,0,0,173,65,0,0,310,117,0,0,664,216,0,0,1210,
    396,0,0,2570,736,0,0,4728,1369,0,0,9976,2558,0,0,18468,4787,0,0,38840,
];

#[test] // always-on: fast prefix (M3)
fn matches_oeis_prefix_to_40() {
    for n in 0..=40 {
        assert_eq!(count(n), A142886[n], "a({n}) mismatch");
    }
}

#[test] #[ignore] // on-demand deep check (M5): cargo test -- --ignored
fn matches_oeis_prefix_full() {
    for n in 0..A142886.len() {
        assert_eq!(count(n), A142886[n], "a({n}) mismatch");
    }
}
```

**(b) Named small-shape tests — assert the witnesses, not just the count:**

```rust
#[test] fn empty()          { assert_eq!(count(0), 1); } // OEIS convention
#[test] fn monomino()       { assert_eq!(count(1), 1); } // single cell
#[test] fn square_2x2()     { assert_eq!(count(4), 1); } // 2x2, vertex-centered
#[test] fn plus_pentomino() { assert_eq!(count(5), 1); } // X-pentomino
#[test] fn ring_3x3()       { assert_eq!(count(8), 1); } // 3x3 minus center
#[test] fn square_3x3()     { assert_eq!(count(9), 2); } // 3x3 solid is one of two
```

**(c) Zero-pattern property test:**

```rust
#[test]
fn zero_unless_0_or_1_mod_4() {
    for n in 0..120 {
        if n % 4 == 2 || n % 4 == 3 {
            assert_eq!(count(n), 0, "a({n}) must be 0");
        }
    }
}
```

**(d) Split-formula consistency (cell- vs vertex-centered sub-counts):**

```rust
#[test]
fn split_formula() {
    const HEAVY_BOUND: usize = 40; // baseline-runtime note
    for n in 0..120 {
        if n % 4 == 2 || n % 4 == 3 {
            // cheap cases: full range
            assert_eq!(count(n), 0);
            assert_eq!(count_vertex_centered(n), 0);
            continue;
        }
        if n > HEAVY_BOUND {
            continue; // heavy n ≡ 0,1 (mod 4): deferred to (a-deep)/(g)
        }
        assert_eq!(count_cell_centered(n) + count_vertex_centered(n), count(n));
        if n % 4 != 0 {
            // A346800 (vertex-centered) contributes only when 4 | n
            assert_eq!(count_vertex_centered(n), 0, "vertex term at n={n}");
        }
    }
}
```

**(e) Symmetry-group unit tests:**

```rust
#[test]
fn group_axioms_and_orbit_sizes() {
    // the 8 transforms are closed under composition, each has an inverse,
    // and the identity is present
    assert!(group_is_closed_with_inverses());
    // cell-centered orbit sizes (§3.1)
    assert_eq!(orbit_cell((0, 0)).len(), 1);  // apex
    assert_eq!(orbit_cell((3, 0)).len(), 4);  // x-axis edge
    assert_eq!(orbit_cell((3, 3)).len(), 4);  // diagonal edge
    assert_eq!(orbit_cell((3, 1)).len(), 8);  // interior
}
```

**(f) Connectivity unit tests:**

```rust
#[test]
fn slice_predicate_matches_reconstruction() {
    // §4.1 predicate (hot path) must agree with a brute reconstruct+BFS
    // over many small slices and both center types.
    for s in enumerate_small_slices(/* up to ~12 wedge cells */) {
        assert_eq!(slice_is_connected_polyomino(&s), reconstruct_then_bfs(&s));
    }
}

#[test]
fn slice_predicate_edge_conditions() {
    // connected but touches NEITHER wedge edge -> rejected, no recon needed
    assert!(!slice_is_connected_polyomino(&cells![(3, 1)]));            // §4.1
    assert!(!slice_is_connected_polyomino(&cells![(2, 1), (3, 1)]));    // §4.1
    // connected and spans x-axis edge to diagonal edge -> accepted
    assert!(slice_is_connected_polyomino(&cells![(1, 0), (1, 1)]));     // 3x3 ring
    assert!(slice_is_connected_polyomino(&cells![(0, 0), (1, 0)]));     // plus
    // connected slice touching ONLY the diagonal edge -> 4 disjoint pieces
    assert!(!slice_is_connected_polyomino(&cells![(2, 1), (2, 2)]));
}
```

**(g) b-file regression (ignored by default).** Cross-checks `count(n)`
against the authoritative OEIS b-file *and* the embedded `REFERENCE`, bounded
at `DEEP_BOUND` (the baseline-runtime note: the literal `0..=163` is
infeasible — the enumeration is exponential, §4.5). Absent data is a skip,
not a failure.

```rust
#[test]
#[ignore] // cargo test --release -- --ignored matches_bfile
fn matches_bfile() {
    let path = Path::new("b142886.txt"); // crate root; not fetched by the crate
    if !path.exists() { eprintln!("skip: b142886.txt absent"); return; }
    for (n, a) in parse_bfile(path).expect("parse b-file") {
        if n > DEEP_BOUND { break; }
        if n < REFERENCE.len() { assert_eq!(a, REFERENCE[n]); } // oracle x-check
        assert_eq!(count(n), a, "b-file mismatch at n={n}");
    }
}
```

Status (M5): verified — `count(n) == a(n)` for **n = 0..=68** against both
the OEIS b-file and `REFERENCE`, zero mismatches (release `--ignored` ≈0.2 s
after §4.6). Deeper `n` is bounded only by the exponential enumeration
(§4.5).

## 8. References

- OEIS A142886 (and A351127, A346800, plus the A056877 symmetry-class
  family) — `https://oeis.org/A142886`.
- D. H. Redelmeier, *Counting polyominoes: yet another attack*, Discrete
  Mathematics 36 (1981) 191–203.
- T. Oliveira e Silva, *Enumeration of polyominoes*,
  `http://sweet.ua.pt/tos/animals.html`.

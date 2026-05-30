# a142886

Compute **OEIS [A142886](https://oeis.org/A142886)** — the number of
polyominoes of `n` cells whose symmetry group is the full dihedral group of
the square.

A **polyomino** is a plane figure built by joining unit squares edge to
edge — the generalization of the domino (two squares) to arbitrary cell
counts. The shapes of size `n` are called `n`-ominoes: monominoes
(`n = 1`), dominoes (`n = 2`), trominoes (`n = 3`), tetrominoes (the four
Tetris pieces, `n = 4`), pentominoes (`n = 5`), and so on. Two polyominoes
are considered the same shape if one can be carried to the other by a
rotation or reflection — equivalently, by an element of the **dihedral
group of the square**, an eight-element group containing the identity,
three rotations (90°, 180°, 270°), and four reflections (horizontal,
vertical, and the two diagonals). OEIS [A000105](https://oeis.org/A000105)
counts polyominoes under this equivalence; the count grows roughly as
`~4.06ⁿ`.

A given polyomino's *own* symmetry group is some subgroup of those eight
elements. The maximal case — when every one of the eight rotations and
reflections sends the shape exactly back to itself — is what OEIS writes as
**D₈** (the dihedral group of order 8; some references write `D₄`, but the
group is the same). A142886 counts the polyominoes of size `n` that achieve
this maximum. They are the most visually symmetric polyominoes possible on
the square lattice: identical under quarter turns, half turns, horizontal
and vertical reflection, and both diagonal reflections.

Examples that **do** have D₈ symmetry:

```
 n=1         n=4         n=5         n=8         n=9

                         .#.         ###         ###
  #          ##          ###         #.#         ###
             ##          .#.         ###         ###

monomino   2x2 square  X-pentomino  3x3 ring   3x3 square
```

Examples that **do not** have D₈ symmetry (each is counted in a different
symmetry class, not under A142886):

```
 n=3         n=4         n=4         n=4

                         ###         #.
 ###        ####         .#.         #.
                                     ##

I-tromino  I-tetromino  T-tetromino  L-tetromino
order 4    order 4      order 2      order 1
```

The I-tromino and I-tetromino are reflection-symmetric across their long
axis and their perpendicular bisector and rotation-symmetric by 180°, but
a 90° rotation turns each into a column — a different shape on the lattice
— so neither attains D₈. The T-tetromino has only the single vertical
mirror; the L-tetromino has only the identity. A142886's defining property
is therefore much stronger than "looks symmetric": it requires that
*every* element of the square's symmetry group fix the shape.

The first terms of `a(n)` for `n = 0, 1, 2, …` are `1, 1, 0, 0, 1, 1, 0, 0,
1, 2, 0, 0, 3, 2, …`. The zeros at `n ≡ 2, 3 (mod 4)` are forced by orbit
arithmetic: under D₈ a polyomino's cells split into orbits of size 1, 4,
or 8 (the center cell, the cells on a mirror axis, and the strict-interior
cells respectively), so `n` is always `0` or `1` mod 4. The sequence grows
much more slowly than the all-polyomino count. The *search space* of candidate shapes that must
be examined to certify each `a(n)` still grows exponentially.

The CLI prints `n a(n)` lines suitable for diff-checking against the OEIS
b-file. The reference b-file `b142886.txt` (R. A. Russell, OEIS,
`n = 0..163`) is bundled at the crate root as the verification oracle;
`--verify` compares against it line by line.

## Building and running

Requires a stable Rust toolchain (the crate uses `edition = "2021"`; tested
on 1.75+).

```bash
# Release build is mandatory for any non-trivial n; the release profile
# turns on LTO, single codegen unit, and panic=abort (see Cargo.toml).
cargo build --release

# Print n a(n) for n = 0..=N (default 0). Each stdout line is prefixed
# with a [YYYY-MM-DD HH:MM:SS] wall-clock timestamp; a 3-line `#` header
# echoes the invocation, start time, and column legend so `awk '!/^#/'`
# yields a clean numeric stream.
./target/release/a142886 --max-n 60

# Bucket-level parallelism across the x-axis buckets, with the cell- and
# vertex-centered totals run concurrently via rayon::join. Counts are
# byte-identical to the serial path; set RAYON_NUM_THREADS to bound the
# pool.
./target/release/a142886 --max-n 80 --parallel

# Cross-check against the embedded OEIS prefix and against b142886.txt if
# present in the working directory. Ignores --center; sums both centers.
./target/release/a142886 --verify

# Restrict to one symmetry-center case (useful for cross-checking against
# the sibling sequences A351127 / A346800).
./target/release/a142886 --max-n 40 --center cell
./target/release/a142886 --max-n 40 --center vertex

# Per-term checkpoint for multi-day runs. At startup, the file is parsed
# and every n already present is skipped (and echoed for continuity). Each
# newly computed term is appended and fsync'd, so a kill, SIGTERM, or spot
# preemption loses at most the one in-flight term. Resume by re-running
# with the same flag.
./target/release/a142886 --max-n 120 --parallel --checkpoint run.txt

# Run the unit-test tier (always-on prefix to n=40, plus connectivity,
# symmetry, and split-formula tests).
cargo test --release

# Run the deep on-demand tier: full embedded prefix n = 0..=68 and the
# b-file regression. Cost grows steeply with n.
cargo test --release -- --ignored
```

## Algorithm

The naive route — enumerate every `n`-polyomino with Redelmeier's growth and
keep those whose 8-element symmetry group is the full one — is hopeless past
roughly `n = 40`. The algorithm avoids that by enumerating only the cells
inside a **fundamental domain** of the symmetry group: the closed `45°` wedge
`W = { (x, y) : 0 ≤ y ≤ x }`. A symmetric polyomino is uniquely determined by
its intersection `S = P ∩ W` with this wedge (the other seven octants are
forced by symmetry), so the search space shrinks by a factor of roughly
eight in cell count and dramatically more in branching.

There are exactly two ways the center of symmetry can sit on the integer
lattice — the center of a cell (`cell-centered`) or a lattice vertex
(`vertex-centered`); any other placement is incompatible with a 90° rotation
of the lattice. The two cases have different wedge geometry and different
orbit-size arithmetic, so they are enumerated separately and added. A
shape's centroid is unique and coincides with its symmetry center, so the
two enumerations produce disjoint sets and never double-count.

Inside a chosen wedge the algorithm uses **Redelmeier's recursive cell
growth** — the canonical untried-frontier enumeration that visits each
connected shape exactly once without a deduplication pass. Each slice is
bucketed by its **minimal x-axis cell** `A = (ax, 0)`; growth starts pinned
at `A`, and any x-axis cell strictly to the left of `A` is forbidden. This
makes `A` the slice's unique canonical representative, so every valid slice
falls in exactly one bucket and the per-bucket counts simply sum.

A polyomino must be a single connected piece, but the algorithm decides
this **without ever building the full `n`-cell shape**. The test is run on
the wedge slice `S` alone, and it has just two parts: (1) `S` is itself
one connected piece, and (2) `S` includes at least one cell on each wedge
edge (the x-axis edge `y = 0` and the diagonal edge `y = x`).

The reason this works: the eight octant copies of the wedge meet along
those two edges. A cell on the x-axis edge is glued to its mirror image in
the next octant, and a cell on the diagonal edge is glued to its mirror
in the octant on the other side. So if `S` is connected and reaches both
edges, the eight copies link up into one connected ring around the
origin. If `S` misses an edge or splits in two, the copies cannot fuse and
the full shape falls apart.

This check looks at only the roughly `n/8` cells in `S`, never the full
`n` cells of the polyomino.

The growth therefore runs in **two phases**, separated by the moment the
slice first reaches the diagonal edge (the x-axis edge is touched for free,
because every bucket starts pinned at an x-axis cell). Before that moment
— the **pre-connected phase** — the slice is still failing the
edge-touch test, so the algorithm carries an extra lower bound on how many
more cells it would take to reach the diagonal. Any branch whose current
weight plus that lower bound already exceeds `n` is pruned: the slice
provably cannot finish at size `n` with the diagonal touched, so its
entire subtree is dead. Once a diagonal cell is added — the
**connected phase** — that lower bound is provably zero and the
bookkeeping for it falls away. The only remaining test is the weight
budget: keep growing until the orbit-weighted total hits `n`, then count
the slice. The implementation specializes the recursion on this phase
(via a compile-time `SAT` flag), so the second phase carries none of the
diagonal-tracking overhead.

A shape is counted when its orbit-weighted cell total `Σ_{c ∈ S} orbit_size(c)`
equals exactly `n` and the slice passes the edge-touch test (connectivity
is preserved by Redelmeier growth by construction). Total counts use a
plain `u64`. Empirically A142886 doubles roughly every `+4` in `n` in the
live range (a(200) ≈ 2.55·10¹⁴), so `u64::MAX ≈ 1.84·10¹⁹` is not reached
until around `n ≈ 264..268`; the count fits comfortably through `n ≈ 263`.

## Optimizations

Every optimization below is **count-preserving**: enabled or disabled, the
output is byte-identical across the verification ranges. They lower the
constant and the effective branching factor; the wedge enumeration remains
exponential, so achievable `n` is empirical, not guaranteed.

- **Residue-class bucket split.** A cell-centered slice has weight
  `c + 4e + 8i` where `c ∈ {0,1}` records the apex, so the apex is occupied
  iff `n ≡ 1 (mod 4)`. The target residue forces exactly one case per `n`:
  for `n ≡ 1 (mod 4)` only the `ax = 0` bucket can contribute and every
  other bucket's entire subtree is provably empty; for cell-centered
  `n ≡ 0 (mod 4)` the `ax = 0` (apex) bucket is provably empty and is
  skipped wholesale.
- **Edge-reachability lower bound.** For the ring case (cell-centered with
  empty apex; also vertex-centered) the slice must touch the diagonal. The
  enumerator tracks `min_gap = min(x − y)` over the current slice and
  computes the minimum additional weight needed to reach the diagonal:
  `8·min_gap − 4` (each of the `min_gap − 1` gap-reducing cells is interior
  with orbit-weight 8, and only the final gap-0 landing is weight-4
  diagonal; the cheap x-axis-rail route is blocked by the
  forbid-left-of-`A` rule). The two excursions to the wedge edges may share
  cells, so the bound is the `max` of the two terms, not their sum —
  tightest admissible. Any node with `weight + lb > n` is cut. Soundness is
  per-node admissibility (every valid completion exceeds the bound) and
  does not require the bound to be monotone down the recursion.
- **Minimal-x-axis-cell bucketing.** Replaces an earlier scheme that keyed
  on the slice's global lex-min cell and grew only to strictly-greater
  cells. There are roughly `n` x-axis roots versus `~n²/2` lex-min seeds, and
  the blocked-set discipline plus the forbid-left-of-`A` rule recover the
  injectivity the dropped `nb > seed` shortcut gave the prior scheme.
  Measured roughly 2× faster with byte-identical counts.
- **Packed cell-state grid.** All three of `in_slice`, `blocked`, and
  `in_untried` collapse into a single `u8`-per-cell state machine
  (`FREE` / `QUEUED` / `SLICE` / `BLOCKED`) backed by a contiguous
  cache-resident array indexed by `(x · stride + y)`. The hot `is_free(nb)`
  test becomes a single byte read; every transition is `debug_assert`-guarded.
- **Boundary-first neighbour order.** The four neighbours of the current
  cell are tried in an order that visits boundary-leaning candidates first,
  improving the rate at which the edge-reachability bound starts biting on
  doomed branches.
- **Bucket-level rayon parallelism (`--parallel`).** Buckets are independent
  search problems, so they parallelize cleanly. The cell- and vertex-centered
  totals run concurrently via `rayon::join`. Counts are byte-identical to
  the serial path; the pool size is `RAYON_NUM_THREADS`.
- **Recursive depth-N fan-out.** Within each top-level bucket the
  enumerator fans out further at recursion depths bounded by a remaining
  weight budget (`D1_MAX_DEPTH = 4`, `D1_MIN_BUDGET = 48`), so a few
  large buckets do not bottleneck a many-core box. (A previous attempt to
  bypass a per-pool ceiling by running multiple rayon pools was reverted
  after measurement at 32-core scale — the depth-N fan-out alone suffices.)
- **Release profile.** LTO, `codegen-units = 1`, and `panic = abort`. Panic
  is a bug in a pure batch enumerator, so dropping unwind tables and
  landing pads frees the optimizer from preserving unwind state across the
  thousands of bounds-check and allocation sites in the recursion.
- **Per-term checkpoint (`--checkpoint FILE`).** Each completed `n a(n)`
  line is appended and fsync'd before moving on, so a kill, SIGTERM, or
  spot preemption loses at most the in-flight term. On restart the file is
  parsed and every present `n` is skipped. The file stays in plain
  `n a(n)` format (no timestamps) so it remains parseable and matches the
  b-file format.

Several other levers were measured and rejected — notably a Jensen-style
anti-diagonal transfer matrix, a two-terminal `(A, B)` bucketing variant
that pins both the minimal x-axis and minimal diagonal cells, and the
multi-pool rayon sharding mentioned above. The DESIGN.md and PERFORMANCE.md
files at the crate root carry the full ledger and the numbers behind each
decision.

## License

This project is released under the MIT License. See [LICENSE](LICENSE) for
the full text. This project was written with Claude (model: Claude Opus 4.7).

# Creusot serving experiment

This is a local experiment, not a Motet build, test, or CI command. It uses
`git show` to copy `crates/spaces/src/serving.rs` from a named commit into a
standalone package. `prepare.py` adds a Creusot import and `ensures` attribute
without changing the decision's implementation. The default commit is
`dcd2c1292375c1b8920a566be4d471c38d6da56f`; the source blob is
`847c314913d7e6351671cc7cce0e3829a4574b8c`.

## Replay

The run used an existing x86_64 Linux Nix package. The store path below
identifies the tested closure; it does not install that closure on another
machine. The wrapper selects Creusot 0.14.0 and its matching `creusot-std`,
Rust nightly-2026-08-03 (rustc `11177f2235f0c842b00f82c558ad9480c0c3a895`),
Why3 1.8.2+git (`05bf4ddf55cb3042f34c3bc0ed3aff56a226978a`), Why3find
1.3.0+dev (`0f054b93c86ac7ba20bca7df5d1563a9a4434a30`) and Z3 4.16.0.
The package supplies other solvers; this experiment uses only Z3.

From the root of this checkout, run:

```sh
creusot=/nix/store/mabzgikmfmq55n55mdsfvq8lqzc6hy4f-creusot-wrapped
scratch=$(mktemp -d)/serving
python3 tools/creusot-serving/prepare.py "$scratch"
(
  cd "$scratch"
  unset CARGO_TARGET_DIR CARGO_BUILD_BUILD_DIR
  unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER
  export PATH="$creusot/bin:$PATH" CARGO_NET_OFFLINE=true
  export XDG_CONFIG_HOME="$scratch/config" XDG_CACHE_HOME="$scratch/cache"
  "$creusot/bin/cargo-creusot" creusot decide --no-cache -j 2 \
    --why3find-arg=--goals --why3find-arg=--proofs -- --locked
)
```

The package needs the crates.io dependencies pinned in `Cargo.lock.in` in
its Cargo cache. If they're absent, run `"$creusot/bin/cargo" fetch --locked`
from the scratch package without offline mode. Allow time for the first
compile of `creusot-std`. Compiling to Coma without running the solver is not
a proof.

Neither attempt reached the solver on this shared host. The first reached its
300-second deadline compiling `creusot-std`. The second was stopped after
821 seconds in the same dependency, before Creusot generated any goals for
`decide`. The recipe reproduced the annotated input byte for byte, but there
is no Creusot proof or counterexample from these runs. Try a cached build of
`creusot-std` before comparing solver times.

## What the contract checks

There are no preconditions. A missing or revoked verification returns
`Refuse`. Otherwise the inclusive interval from the verification time to
its deadline determines `Serve` versus `Stale`. The contract adds the bound
in mathematical integers, unlike the implementation's `u64` age
subtraction. Creusot translates that subtraction to an operation with a
range obligation even though its compiler disables Rust's overflow checks.
The function has no loops or calls to other Motet functions.

The result relies on the Rust-to-Coma translation, Creusot's model of Rust
and bounded integers, Why3/Why3find and Z3. It covers this decision, not clock
synchronization, credential validity, the callers' use of the result,
serialization, or private AppView serving. It does not choose a verifier for
Motet or replace any tests.

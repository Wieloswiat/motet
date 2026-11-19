# 0023. Try Kani on the Spaces access check

Status: Proposed.

## Context

`spaces::serving::decide` decides whether an access check is recent enough to
serve private content. Property tests try many inputs, but not every possible
timestamp. This small function is a useful place to try a proof tool.

The testing guide originally named [Creusot](https://creusot.rs/) as a candidate.
No Creusot proofs were implemented. Kani harnesses and Nix packaging were added
later, but no comparison with Creusot was recorded. Their presence does not
settle which tool Motet should adopt.

## Proposal

Keep the existing Kani experiment while the tool choice is open. Do not require
it for image publication. The CI `proofs` job still reports failures; the image
jobs depend only on `checks` and `gate`.

The `proofs` command runs four harnesses against the production function. A
harness supplies inputs and checks assertions about the result. Here, Kani
checks all possible `u64` values for the timestamps and freshness limit, rather
than sampling them. It checks revocation, old checks, future timestamps and the
exact freshness cutoff.

The function has no loops, so the check does not depend on a loop-iteration
limit. It establishes that the results meet the harness assertions, assuming
Kani and its underlying checker, CBMC, work correctly. It does not establish
that a caller's clock is correct, or that private pages enforce the result.
Property tests and tests of the real services remain necessary.

## Cost and limits

`nix/pkgs/kani.nix` pins Kani 0.68.0, CBMC and the nightly Rust compiler Kani
needs. Production code still builds with `rust-toolchain.toml`. Updating Kani
can also require updating its compiler. This adds CI time, downloads and a
separate toolchain to maintain. The job saves its results in
`target/kani/proofs.json`.

The package has releases for x86_64 Linux, aarch64 Linux and aarch64 macOS. Other
platforms need both an upstream release and Nix packaging.

## Before adopting a tool

Compare Kani and Creusot on the properties Motet needs to check. Record the
assumptions, specification work, supported code and toolchain maintenance for
each. Four passing harnesses are evidence about one function, not a reason to
choose a verifier for the whole project.

Revisit this experiment if compiler upgrades make it unreliable, its CI cost
outweighs its value, a required platform is unsupported, or a new property needs
a different proof method. Any future publication requirement needs a separate,
explicit decision.

# 0002. One Nix flake for development and CI

Status: Accepted; shell entry point superseded in part by [0025](0025-native-devenv.md)

## Context

Developers and CI need the same Rust, Node, PostgreSQL with pg_search,
S3-compatible storage and tools. They also need to run the same checks;
otherwise a local success may not mean a CI success.

## Decision

Use `flake.nix` as the single entry point for development, builds and CI.

- Read `rust-toolchain.toml` through rust-overlay so all three use the same
  Rust toolchain.
- Define the dev shell through [devenv](https://devenv.sh)'s flake-parts
  module in `nix/devenv.nix`. It supplies packages, PostgreSQL and SeaweedFS
  processes, git hooks and commands. Define commands as devenv scripts with
  descriptions shown by `devenv info`, not in a justfile or Makefile.
- Run hermetic checks with `nix flake check --no-pure-eval`. These are defined
  with crane in `nix/rust.nix`: formatting, clippy, default tests, cargo-deny
  and generated lexicon freshness.
- Run the full service-backed gate through `devenv test` and its `enterTest`
  hook. CI runs this and the hermetic checks, with no separate check commands.
- Build container images with Nix's `dockerTools.streamLayeredImage`.
  Do not maintain a second build recipe in a Dockerfile.

## Consequences

Contributors need Nix for this workflow. They can use `cargo` and `pnpm`
directly with the repository's pinned versions, but CI remains the reference.
The devenv flake integration requires `--no-pure-eval`.

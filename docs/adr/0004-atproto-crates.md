# 0004. Build on atproto-crates

Status: Accepted; the crates.io choice is superseded by
[0013](0013-tenor-hosts-worlds.md).

## Context

[atproto-crates](https://tangled.org/ngerakines.me/atproto-crates) supplies the
identity, lexicon, OAuth and record libraries needed by the backend plan.
It also has the Spaces and Tap clients Motet needs, which have no other Rust
implementation. Some crates, including `atproto-tap`, are on crates.io.
The Spaces and testkit crates are not yet released.

## Decision

Use atproto-crates rather than implement these protocols in Motet.

- Take released crates from crates.io at one version. Bump them together;
  Renovate groups their updates.
- Add unreleased crates from git at a pinned revision when code first needs
  them. At that point, move the released crates to the same git revision and
  add the repository to `deny.toml`'s `allow-git`. Mixing sources would give
  Motet incompatible copies of the core types.
- If an upstream crate lacks a needed feature, implement it in the Motet
  crate responsible for it and remove that code when upstream supplies it.
  For example, at version 0.14.5, `motet-xrpc::service_auth` verifies service
  tokens using upstream key and JWT primitives, with tests using real keys.
- Match `reqwest`'s major version to atproto-crates so its clients can use
  `motet-net`'s hardened HTTP client.

## Checked from a machine that can reach tangled.org (2026-10-19)

- Cargo can fetch a pinned git `rev` through Tangled's redirect to its knot.
  This uses `net.git-fetch-with-cli = true`, already in `.cargo/config.toml`.
- Nix's `builtins.fetchGit`, used by crane to vendor git dependencies, can
  fetch the same revision. Flake checks and image builds need no
  `outputHashes` for it.
- Combining the git Spaces crates at 0.15.0-rc.6 with crates.io's
  `atproto-identity = "0.14.5"` puts two identity versions in the lockfile.
  Adding Spaces therefore requires moving every atproto crate to git at once.

## Consequences

Keep changes caused by the evolving Spaces API in `motet-spaces`, rather
than spread them through Motet. Once git dependencies are added, CI needs
network access to tangled.org.

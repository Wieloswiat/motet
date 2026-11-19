# 0017. Start the local network with only the Rust PDS

Status: Accepted. Supersedes [0016](0016-local-atproto-network.md)'s two-PDS
requirement and its patch scope; the rest of 0016 stands.

## Context

[0016](0016-local-atproto-network.md) chose both the reference TypeScript PDS
at the 2026-10-22 Spaces alpha and atproto-crates' PDS. The Nix build of the
reference PDS's pinned pnpm workspace exhausted the build host's memory
during OAuth UI message extraction. No working binary was verified.

0016 also planned private-host patches for both indigo's relay and Tap.
At the pinned revision, Tap's HTTP and WebSocket clients already allow
private hosts.

## Decision

Start with only the Rust PDS and patch only the relay.

- Run PLC, the relay, Tap and atproto-crates' PDS in both native and compose
  environments. Do not ship the reference PDS package, process or image until
  its build and runtime are verified. Do not substitute runtime `npx` or an
  unpinned image.
- Keep the relay's private-host patch off by default. On the first local
  subscription, replay from cursor zero so crawling also delivers records
  written before the subscription started.
- Have `network-smoke` create two accounts on the Rust PDS and verify their
  pilot profiles through PLC, relay crawling and Tap's acknowledged channel.

## Consequences

The smoke test checks local ingestion, not compatibility between independent
PDS implementations or private Spaces. Those checks still need the reference
PDS.

## Revisit when

The reference PDS builds reproducibly within development machines' memory,
or the Spaces alpha publishes an image that can be pinned by digest.

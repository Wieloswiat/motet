# 0016. Run an atproto network locally

Status: Accepted

## Context

Local tests need a relay to crawl the tenor, Tap to deliver records, PDS
accounts with Spaces for players and a PLC identity directory for Worlds.
Depending on the public network would make those tests unsuitable for
repeatable development and CI.

Only Tap is packaged in nixpkgs. The PLC server, relay, reference PDS Spaces
alpha and atproto-crates PDS are neither packaged nor published as images.
Indigo's relay and Tap reject loopback hosts and ports other than 80 or 443
as protection against server-side request forgery (SSRF), so they cannot
crawl or backfill the local services without changes.

## Decision

Run a pinned local atproto network alongside PostgreSQL and SeaweedFS.

- Make `devenv up` start an in-memory PLC directory, indigo's relay and Tap
  from one pinned indigo revision, the reference PDS at the 2026-10-22
  `permissioned-data` Spaces-alpha merge, and atproto-crates' PDS at the same
  revision as Motet's libraries. Use the same builds in the compose stack.
- Give players accounts on both PDSes so tests exercise two independent
  Spaces implementations from the start.
- Patch the relay and Tap to allow private hosts only with an explicit
  development flag. Keep it off by default, store the patch in `nix/patches/`
  and review it whenever the indigo pin changes.
- Build everything with Nix from pinned sources. Fetch nothing at runtime.

## Consequences

The dev environment and CI must build Go and Node workspaces. First builds
are slow; later builds use the cache. Motet must maintain the private-host
patch until upstream provides an equivalent option.

## Revisit when

Indigo supports private hosts in a development mode, the Spaces alpha merges
to `main` with published images, or upstream packages atproto-crates' PDS.

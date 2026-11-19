# 0010. Generate Rust and TypeScript types from lexicons

Status: Accepted

## Context

Lexicons describe the records and methods shared by Motet, tenors and
independent clients. Rust and TypeScript both need types generated from
them. Those types must retain unknown fields from newer clients, and the
generator must reject references to missing definitions.

## Decision

Use `motet-lexgen` to generate Rust types into `crates/lexicons` and TypeScript
types into `packages/lexicons` from `lexicons/`.

- Commit generated code for review. Make `lexgen-check` and a flake check
  fail if it no longer matches the lexicons.
- Preserve unknown fields when reading and writing records.
- Keep Motet's lexicons under `at.motet.pilot.*` until stable. Do not turn a
  pilot definition into a stable NSID in place; an NSID is the protocol name
  that clients use to identify the definition.
- Put authorization rules in the XRPC registry, not the lexicons.

## Revisit when

atproto-crates can generate the types we need in both Rust and TypeScript.

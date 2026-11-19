# 0001. Motet's licence

Status: Accepted

## Context

Motet needs a code licence before dependencies and distribution rely on it.
The choice affects tenor self-hosters and the use of ParadeDB's AGPL-3.0
community edition. It does not settle the rights to users' writing.

## Decision

License Motet's own code under MIT. The root [LICENSE](../../LICENSE)
contains the standard MIT text, copyright 2026 the Motet contributors.

- Keep crates at `publish = false` and packages at `"private": true` until
  a separate decision to publish them.
- Keep the licence allow list in `deny.toml`. Permissive licences and weak
  copyleft licences such as MPL-2.0 allow Motet to be distributed under MIT.
- Run ParadeDB's AGPL-3.0 community edition as a separate database service.
  This does not change Motet's licence. Operators must still meet ParadeDB's
  AGPL obligations, including applicable distribution and source obligations.
- Preserve vendored code's own licences. `tools/oxlint/anti-slop` is MIT and
  retains its LICENSE files.
- Leave the missing upstream licence for the vendored opensocial.group
  lexicons unresolved; see [lexicons/README.md](../../lexicons/README.md).
- Keep the legal review of users' writing in Running Worlds. It must cover
  hosting, rendering, migration and archives. The code licence grants no
  rights to that content.

## Revisit when

A contributor agreement or a need to relicense requires different terms.

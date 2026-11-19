# 0007. Rebuild database views from the archive

Status: Accepted

## Context

The AppView builds pages from projections: database tables derived from its
archive. It can recreate those tables, but it cannot recreate service state
such as queued writes, sessions and read positions.

## Decision

Keep derived data and service state in separate PostgreSQL schemas.

- `projection` has no migrations. A change to `projection/schema.sql` bumps
  `PROJECTION_VERSION`. The AppView replays the archive into a fresh schema
  and swaps it in.
- `service` holds state that cannot be rebuilt and uses ordinary sqlx
  migrations.
- Use `projection::apply` as the only projection writer, both during live
  indexing and rebuilds. Test that replay produces the same tables; sharing
  this code avoids separate implementations drifting apart.
- Keep the newest applied repo revision for each record. A late event must
  not restore an older version on screen.
- Use `sqlx::query` and check SQL at runtime in integration tests. Compile-time
  SQL checking would require a live database or a committed query cache for
  every build, while the projection schema is replaced rather than migrated.
- Keep the outbox in PostgreSQL, with leases and `SKIP LOCKED` so workers can
  claim different jobs. Add a message broker only if measurements justify it.

## Consequences

Every query needs an integration test that executes it. Search indexes using
pg_search belong to the projection schema and rebuild with it.

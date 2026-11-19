# 0006. Keep durable logs in object storage

Status: Accepted

## Context

A tenor must keep working after its node is replaced. The AppView must be
able to rebuild its database without crawling the network again. Both need
an ordered, durable history in storage a self-hoster can rent, such as R2 or
S3.

## Decision

Use `motet-log` to store that history as segments in an object-storage bucket.

- Append segments with conditional writes so only one writer can occupy a
  given position. When a second writer takes over, the old writer must be
  unable to continue; this is fencing. If a write's result is unknown, read
  it back to find out whether it succeeded rather than retry it blindly.
- Batch entries into segments, also called group commit. Do not acknowledge
  an entry until its segment is durable.
- Store each tenor shard's World commands in this log (`tenor::shard`).
  Rebuild its in-memory state by replay. Add SlateDB for current state and
  replay checkpoints later without changing the durability rule.
- Archive every verified AppView record to a log before applying it to
  PostgreSQL.
- Use SeaweedFS's S3 gateway in development and CI. MinIO was abandoned
  upstream and nixpkgs marks it insecure. The fencing tests also check that
  the store enforces conditional writes.

## Revisit when

Tenors need takeover between multiple nodes using one bucket, or log
compaction must account for erasure requests.

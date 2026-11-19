# 0013. Build the tenor account host from atproto-crates

Status: Accepted. Supersedes the crates.io half of [0004](0004-atproto-crates.md).

## Context

The backend plan makes each World a group account hosted by its tenor. The
tenor must serve the World's `did:plc` identity, signed public repository,
relay-readable firehose and Spaces. GMs must be able to act as the World
using their own accounts.

Groundwork compared two starting points on 2026-10-26:

- **vlpds v1.2.3 (MIT)** is a complete personal data server (PDS) with all
  durable storage in a bucket. Its storage design fits the tenor well.
  However, it is one server crate built on its own `vlatproto` implementation,
  with incompatible CID, Merkle Search Tree (MST) and CAR archive types.
  Using it would require adopting that implementation throughout the host.
  Its Spaces implement the October 1 alpha, without STAR repositories, KEM
  keys or MAC headers, and it has no support for one account acting as another.
- **atproto-crates 0.15.0-rc.6 at `09a868cd` (MIT)** implements the 2026-10-22
  Spaces alpha and separates protocol libraries from storage and servers.
  These include repo trees and commits in `atproto-repo`, Space authority
  and repo operations behind storage traits in `atproto-space`, PLC identity
  construction in `atproto-identity`, OAuth primitives in `atproto-oauth`,
  and `atproto-xrpcs` and `atproto-tap`. Its unpublished `atproto-pds` server
  is less reusable: the repo writer, sequencer, Space services and OAuth
  server require its SQLite account database and HTTP state.
- Neither implements group accounts. The opensocial.group proposal requires
  the group host to be the group's OAuth authorization server and check each
  member's roles on every write ([0015](0015-opensocial-group.md)).

## Decision

Build the tenor's account host from atproto-crates protocol libraries on
Motet's own storage ([0014](0014-tenor-state-in-slatedb.md)).

The tenor serves each World's repository, sync methods, firehose, Spaces as
both authority and host, handle resolution and OAuth authorization for the
World DID.

- Use vlpds as a storage reference, not a dependency. Follow its use of
  SlateDB with SlateDB's write-ahead log (WAL) disabled, the node log as the
  only WAL, an applied marker for each batch and memtable flushes for
  checkpoints.
- Port only small, pure parts of `atproto-pds`: the `#commit` CAR builder,
  firehose frame and payload encoding, and PLC genesis ordering. Attribute
  them and record the source revision and every local change in an
  `UPSTREAM.md`, as `packages/design` does. A dependency on the whole crate
  would bring mandatory SQLite and HTTP features and an API its README
  describes as unstable.
- Write Motet's Space-host and OAuth-server code on the protocol libraries,
  using `atproto-pds` handlers as references.
- Pin every atproto crate to the same git revision in the workspace and
  allow that source in `deny.toml`. Version 0.15 is not on crates.io; mixing
  it with crates.io's 0.14.5 duplicates core types. Move reqwest to 0.13 with
  atproto-crates so upstream clients can still use `motet-net`'s hardened
  HTTP client.
- Put the owner's rotation key first in the World's PLC genesis. Include a
  tenor-held recovery key after it only if the owner opts in. The tenor
  reserves the World's signing and key-encapsulation (KEM) keys and returns
  an unsigned genesis operation. The owner signs it on their device; the
  tenor then submits it to PLC.
- Let GMs use service auth for Motet's typed World methods. Also issue OAuth
  sessions for the World's DID after the GM authenticates at their own PDS.
  Record the GM on every session and check their current roles on each write.

## Consequences

Motet must maintain several thousand lines that a reusable PDS library would
otherwise provide: commit processing, sync endpoints, Space-host handlers
and an OAuth server. In return, it uses one set of atproto types and one
storage design.

Ported modules can diverge from upstream. Review their `UPSTREAM.md` files
when changing the pinned revision. CI and image builds need network access
to tangled.org.

atproto-crates has no Sync 1.1 inductive verification, which checks a repo
update against the previously known tree. The tenor publishes `prevData` and
`since`; consumers must verify them.

## Revisit when

atproto-crates provides a repo writer, sequencer, Space host or OAuth server
independent of storage, so Motet can remove the corresponding port or local
implementation. Also reconsider if vlpds supports the current Spaces alpha
and provides libraries using shared types.

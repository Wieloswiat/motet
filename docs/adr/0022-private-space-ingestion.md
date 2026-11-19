# 0022. Let the AppView read private Spaces and check each reader’s access

Status: Proposed

## Context

The AppView needs to read private posts and show them only to people who still
have access. The tenor already hosts Spaces, issues credentials, tracks writers
and publishes private admissions outside the public repo. The AppView reads only
public data. Its `SpaceRecordEvent` represents a public snapshot and must not
carry private records.

This proposal would let the AppView copy a Space using one authorized member's
session. It would check each reader's access separately, at least every 60
seconds. Alice must lose access when removed, even if Bob's session continues
syncing the Space. The requirements below remain proposed. Local reader-lease code now exists in
`appview::private_spaces::access`, but it does not authenticate credentials, check
the authority or coordinate invalidation across instances. Private ingestion and
serving are not enabled. The [owner decisions](#owner-decisions-before-acceptance) remain open.

This extends [0006](0006-storage.md) and [0007](0007-projections.md). It keeps
member attribution from [0018](0018-world-service-auth.md) and
[0021](0021-single-use-member-tokens.md), and the audience rules in
[the plan](../plan.md#audience-lifecycle). An audience identifies who may read
a record.

If accepted, it would supersede ADR 0018's statement that the tenor “does not
mint a World token for downstream services”. `getDelegationToken` already issues
tokens with `iss=World`. The proposal would allow those tokens only when the
local authority can enforce their link to the requesting member. It would refuse
issuance to a remote authority that cannot check that link.

This does not introduce World OAuth sessions or end-to-end encryption. Space
hosts, the tenor and the AppView can read private content. Space message
authentication codes (MACs) authenticate reads but allow deniability: they are not
publicly verifiable author signatures.

### What upstream supplies and what Motet must check

These observations use atproto-crates revision
`09a868cdde938c789326f1db16ffc30bbf38c15e`, pinned in `Cargo.lock`:

- `atproto-space-client::{credential,attestation}` supplies member-PDS
  delegation, client attestation, credentials tied to a key, and notification
  registration. A delegation lets the AppView request access on a member's
  behalf; the client attestation identifies the AppView. `SpaceHosts` distinguishes
  the member's PDS from the Space authority.
- `sync` supplies `SpaceCheckpoint`, `list_repos`, `list_repo_ops`, snapshot
  reads in Spaces’ STAR format, and `OplogGap`. A STAR snapshot contains a repo’s
  complete record set. `listRepos` learns from notifications; it is not a complete
  list of members. `catch_up` keeps all pages in memory, so a durable worker needs
  bounded paging instead.
- Credential-based `get_repo` verifies the commit MAC and the digest of the
  complete record set. `get_repo_streaming` is complete only when its record stream
  ends. Its response byte limit does not bound the whole stream; Motet must do so.
- `list_repo_ops` authenticates the supplied final commit, not each operation.
  Motet must check that applying the operations produces the set that commit
  describes. Motet must also hash fetched blobs to verify their CIDs (content
  identifiers); the blob fetcher does not do this.
- `tenor::spaces::credentials` remembers which member requested a locally issued
  World delegation. Another authority cannot read that information.
  `tenor::spaces::http::Delegation` currently allows remote targets.
- `appview::{archive,indexer,projection}` archives data before projecting it and
  acknowledges delivery only afterward. It does not yet check every record
  against its lexicon, implement private retention, or authorize private queries.

## Proposed approach

### Obtain credentials through a member’s own PDS

The tenor decides access and admission. `motet-spaces` keeps the pure identity
and serving decisions. An `appview::private_spaces` worker would renew credentials,
discover repos and ingest verified data. It must use `motet-net` for all outbound
requests. The browser continues to talk only to the AppView.

For a private read, the AppView must:

1. Use the signed-in person's OAuth grant to call
   `com.atproto.space.getDelegationToken` on **that person's own PDS**.
2. Present that delegation and an AppView client attestation to the World's
   Space authority at `getSpaceCredential`.
3. Require the delegation issuer to be the person, not the World. The authority
   checks that person's current opensocial roles and Space grants. App identity
   satisfies the application's access policy; it cannot replace membership.

Add the Space OAuth scopes explicitly. Ask members to consent again when their
existing sessions lack those scopes. Never forward their OAuth bearer tokens to
the tenor. World mutations keep their exact-method, single-use service tokens.
Adapt the SDK's credential helper to the AppView's serialized session refresh and
nonce handling; do not start a second refresh loop.

Verify the returned credential's authority signature, Space, expiry and binding
key. Constructing a `SpaceCredentialGrant` does not verify its signature. Generate
a new binding key each time a credential is issued. Seal credentials and keys in
service storage, bound to their identity and purpose; keep them outside the
archive. Persist the delegating member, session reference, credential JTI (token
identifier), expiry and subscription expiry. Never log tokens, keys or private
record bodies.

One authorized member with a usable session may supply the credentials for
syncing a Space. Call that member its **sponsor**. On renewal, the worker may
choose another currently authorized member with a live, consented session. This
does not renew any reader's access check.

Renew before expiry, with fresh single-use delegation and attestation tokens.
Refresh notification registration separately. If credential issuance may have
succeeded but its response was lost, use new assertions rather than replaying
possibly spent ones. Losing the last usable session stops both syncing and
serving that Space, even if a cached credential has not expired. Whether to keep
the cached content is a separate retention decision.

### Find repos without reusing deleted Spaces

A World DID or invite tells the AppView where to look, not who may read. A private
World must work without a public declaration. Use a member credential to read the
known `members` Space, then its verified Space index, memberships, roles and
permissions. Read a scene's access facts only with a credential for that scene's
Space. Resolve each author's current PDS from its DID, using the destination
checks in `public_space`.

Keep a private inventory of repos that may need reading. Build it from current
and historical membership and access facts, observed writers, World admissions
and previous syncs. Include members who have never written and former writers.
`listRepos` can add candidates and revisions but cannot remove inventory entries.
The tenor needs its own durable inventory for admission and owner recovery; it
cannot depend on the AppView being available. Do not publish privileged inventory
in the members-visible index.

Identify a record by `(Space URI, generation, repo DID, collection, rkey)` and a
version by that identity plus its CID. Here `rkey` is the record's key within its
collection; a generation distinguishes lifetimes of a Space internally. Keep
operations separate from content. Identical content in two Spaces must not share
access, deletion or cache state. Tie every job, subscription and verification to
its generation.

For First World, the tenor must never reuse a deleted Space key:

- A new private scene gets a fresh scene rkey and therefore a fresh Space URI.
- Keep an irreversible inventory of retired keys across restart, export and
  recovery. Client-supplied creation keys cannot bypass it.
- The standard Space reference needs no new lifecycle descriptor. The internal
  generation is an extra safeguard; it does not change reader lease rules.
- The well-known `meta` and `members` Spaces use `self`. Refuse deletion and
  recreation of those Spaces in First World; update their records instead.

Restoring the same logical Space from an export is not recreation. Preserve its
identity and retired keys, declare the recovery epoch separately, and invalidate
old leases. Recovery must reconcile missing retirement evidence before creating
Spaces. Keys with uncertain histories remain unavailable. Reusing a URI in the
future would need an authenticated lifecycle descriptor and a separate decision.

A deleted Space remains denied. A delayed deletion notice names its retired URI,
so it cannot erase a replacement scene with a new URI. Do not implicitly move
content between public and private storage or widen its audience. Ordinary private
scenes reveal their cast through roles. Secret scenes remain unsupported until
their separate access extension exists.

### Require PDS writes that check for concurrent changes

Private posting must wait for atomic `swapRecord`/`swapCommit` checks on the
applicable `createRecord`, `putRecord` and `applyWrites` Space endpoints. These
checks prevent a write from silently overwriting a concurrent change. The pinned
atproto-pds does not implement the required checks.

Require a documented capability contract and tests with real concurrent clients.
The AppView must refuse hosts without this support. Plan slice 4a owns the work.
A temporary PDS patch needs recorded upstream provenance and a new ADR
superseding ADR 0017's relay-only patch scope. Prefer an upstream fix and a
coordinated pin update, not a permanent private protocol fork.

### Verify records, then archive and project them

#### Verify a complete snapshot before using it

Start with bounded full STAR snapshots of each Space repo. Authenticate the
request to the actual repo owner and obtain its current key encapsulation
mechanism (KEM) key. Verify the commit's Space, repo, revision and MAC, every
record, and the digest of the final record set. Check record bytes, CIDs,
lexicons and audience references between records.

Keep incomplete snapshots in temporary storage that cannot serve readers. Do not
advance a cursor or expose part of a snapshot. Reject older revisions and
inconsistent contents at the same revision; mark the repo unresolved. A verified
current snapshot establishes current state, not every change made during an outage.

Later, incremental sync may apply pages of operations to a staged copy of the
last verified set. Check previous CIDs and record hashes, then compare the final
set digest with the authenticated final commit. Fall back to a full STAR snapshot
on a gap, missing values or a mismatch.

Never substitute today's content for an admitted version. Missing dependencies
remain unresolved. A post becomes visible only through an exact World-authored
admission in the same Space and generation. Order posts by the World’s epoch,
then by the admission’s position (`place`) within that epoch. Former members’
admitted posts remain, but their new edits need admission.

#### Store private content separately and encrypted

Keep private payloads **out of the public archive log and public tables**. Store
them under a separate private object-store prefix with restricted credentials and
application-layer authenticated encryption. A separate prefix alone does not
encrypt anything.

The existing fenced AppView archive writer remains the single source of ordering.
Its versioned log entries reference encrypted objects containing private evidence
and payloads. Make those objects durable before appending the reference. Project
only committed entries. Log entries must not contain private prose, member names
or scene titles. Keep private evidence objects access-restricted too.

Use a distinct encryption key for each payload object that may need erasure.
Wrap those keys with a persistent archive master key kept outside PostgreSQL and
the bucket. Include object identity, Space and generation in the encryption's
authenticated data. Keep removable key references separate from immutable evidence.
Do not deduplicate plaintext or keys across audiences. Private PostgreSQL
projections and backups need encrypted disks and restricted access: encrypting
objects does not protect plaintext SQL data.

#### Delete content without losing the evidence needed for replay

Keep admissions, version identities, observed deletions and policy decisions as
replay evidence, separate from erasable prose and media. By default, deletion
removes content and leaves a tombstone at its place. A World-authored copy may
remain only under an explicit retention policy shown before joining. Removing
membership does not erase content.

Space deletion must remove its retained data and derived copies as the protocol
requires. Keep only minimal permitted erasure receipts, not a hidden recoverable
archive of deleted content.

An erasure ledger must take precedence over every replay and restore. Its entries
can only advance. Store them in an append-only hash chain in a dedicated,
versioned object-store bucket, under a separate administrative account from the
AppView archive and PostgreSQL backups. The application may append and read, but
cannot roll back or delete entries. Each entry records the deployment identity,
ledger epoch, increasing sequence, previous-entry hash and erased payload or
generation identities, without prose.

A separate off-box **witness service** must durably remember the latest
`(deployment, epoch, sequence, head hash)`. It accepts only compare-and-swap
advances, so a stale writer cannot replace the current head. Keep its account and
signing key outside the application's backups and restore process. A new epoch
needs explicit operator authorization and a link to the previous witnessed head;
resetting a database cannot create one.

Acknowledge erasure only after both the ledger entry and witness advance are
durable. If an advance is uncertain, stop acknowledgement and serving until it
is reconciled. Never roll the witness back.

Before a restore can serve traffic:

1. Query the witness directly over authenticated TLS.
2. Read the ledger through that witnessed head and verify its hash chain.
3. Apply every erasure before exposing projections or unwrapping restored
   payload keys.

Missing entries, an older head, an epoch/hash mismatch or an unreachable witness
must block traffic. PostgreSQL may cache the ledger but cannot establish that it
is current. An old signed witness receipt can be replayed; it does not establish
the current head. An offline rebuild may use a fixed, verified ledger cut, but
serving still requires the online check above.

Test restoring old data **and an old ledger** while the witness remembers a later
erasure. Restore must refuse to serve until it fetches and applies the missing
entries.

Erasure removes live payload keys, objects, renders, search rows and caches.
Storage versions and backups need an explicit purge schedule. Deleting a live key
is not cryptographic erasure if a backup can restore it. Disclose that window
until backup purge completes, and require the current ledger before any restore
serves traffic. Neither a separate prefix nor soft deletion counts as erasure.

#### Rebuild projections from committed evidence

Use `projection::apply` for private events in both live ingestion and rebuild.
Add private tables that distinguish audiences, bump `PROJECTION_VERSION`, and
version the archive format with compatibility fixtures. Credentials, worker leases
and refresh state need service migrations. Derived inventory and verified
checkpoints must be reconstructible from committed archive evidence. Rebuild
scheduling from those checkpoints, not checkpoints from scheduling state.

### Check each reader’s access and stop output after revocation

A credential used to sync a Space says nothing about the current reader. Identify
the reader from the AppView session, never a query DID.

Keep a separate, short-lived access check, called a **lease**, for each
`(reader DID, Space URI, generation)`. Store the authority identity, verification
start time, local invalidation generation, credential JTI and credential expiry.
The invalidation generation is a counter that changes when access may have been
removed; it prevents an older check from restoring revoked access.

Initially, establish a lease with a fresh delegation for that member and a new
credential from the authority. Issuance checks current access. Reusing an
hour-long credential is not a fresh check.

#### Renewal cost and freshness

For every reader/Space pair being continuously served, renew at least once every
60 seconds. Each renewal needs a new binding key, one OAuth-authenticated PDS
delegation request and one tenor credential request. Session refresh, nonce
challenges and notification registration add work.

At 1,000 active pairs, this is approximately 16.7 credential issues and 33.3 base
HTTP requests per second, before retries. These are arithmetic estimates, not
measured capacity. Measure complete renewal latency and authority request rate.
Consider a dedicated authenticated access-check query if renewal p95 exceeds
500 ms, or credential issuance exceeds 10 requests/second per tenor for 15 minutes
under representative load. Do not lengthen the freshness bound to hide cost.

The lease adapter takes a verified result and its start time, not a fixed number
of network hops. A different verification transport must preserve the same
freshness and invalidation rules.

Read the serving clock at verification **start**, not when the response arrives.
Otherwise network delay would extend permission age. Before installing a result,
compare the invalidation generation captured at start with the current one. A
removal observed during verification must defeat an older successful response.
Discard restored leases at startup and discard leases on clock discontinuity.

#### Check again before sending private output

At query time, require a live local session, a usable Space sponsor and the
current generation. Check credential expiry. Pass this reader's lease to
`spaces::serving::decide` as `Verified { at_millis, revoked }`, with the current
serving clock and `DEFAULT_BOUND_MILLIS` (60 seconds). Only `Serve` permits output.
A missing, future, expired or revoked verification denies output. Failed renewal
must not extend the old timestamp. Share concurrent renewals for one reader/Space
pair, with bounded waiters and timeouts.

Immediately invalidate affected leases on authoritative denial, membership or
access removal, Space deletion, or credential revocation. If a policy change's
effect is uncertain, invalidate every potentially affected lease until checked
again. Do not infer a reader's revocation solely from another sponsor's JTI.
The standard credential-revocation notice targets repo hosts; the AppView cannot
rely on it to learn every reader-access change. Verified authority changes and
AppView-mediated changes allow early invalidation. Fresh reader checks enforce
the bound when notices are lost. Bob's continued sync cannot preserve Alice's
access.

Persist invalidation generations centrally and check them across instances when
sending output; a process-local cache is not enough. Serialize lease installation
against invalidation. Recheck before each streamed chunk or queued delivery, and
close live subscriptions on invalidation. Delivered bytes cannot be recalled;
the guarantee applies to later output after observed revocation.

Filter authorized audiences in SQL **before** counting, ranking or pagination.
Use the same access checks for historical versions, media, exports, downloads,
search, server-sent event (SSE) replay and queued notifications. Public endpoints
remain public-only. Private responses use `Cache-Control: private, no-store`.
Do not use a public CDN or bearer media URL that outlives access. Switch reader
identity on every account change. Refusals must not let callers distinguish
unknown private resources from denied ones. Report stale authorization as
unavailable without disclosing content.

### Find missed changes and limit background work

Notifications are authenticated hints with no content. Persist a bounded task to
resync the changed repo before acknowledging the notice. That acknowledgement
does not promise that content has been projected. Advance verified checkpoints
only after encrypted archive objects, log entries and projections commit. After
a crash, resume from archived checkpoints and use event identities that make
repeated application harmless.

Reconcile at startup, after notification gaps or outages, on membership changes
and periodically. Page through the durable candidate inventory independently of
`listRepos`, including never-writers and former writers. The tenor needs the
same discovery and verified reads to admit independent clients' posts. The AppView
cannot invent admissions. Find a first post made during a 24-hour notification
outage even if no notification ever registered its writer. Current permission
still governs admission after the outage.

A former writer's verified deletion removes the displayed post without waiting
for a tenor closing record. Verified absence from a full snapshot can tombstone
a known record. Timeout, access refusal, `RepoNotFound` and uncertainty about
account migration are not verified deletions. Keep the last verified state as
stale and retry.

A snapshot cannot reveal a deletion followed by byte-identical recreation if
both occurred inside a lost operation log. Mark that history gap unresolved. Do
not claim that the old admission authenticates a recreation that was never observed.

Bound concurrent fetches, bytes, staging disk, pages per job and retry rate.
Persist continuation and schedule fairly across Worlds. On quota exhaustion,
visibly pause affected sync. Do not drop inventory or acknowledge unpersisted
work. An expired retry budget is not grounds to remove a former writer. Retire
an inventory entry only when no surviving contribution or retained history needs
it, and record the retention or erasure decision durably. Before sweeping abandoned
staging objects, exclude anything referenced by committed log entries.

### Rebuild without restoring access; recover without changing authority

Rebuild from a fixed archive cut, format/rules version and current erasure ledger,
without network or side effects. Build fresh private and public projections
alongside the old ones; swap only when complete. Rebuild restores content and
historical access facts, **not** usable credentials or fresh reader leases.
Removed readers remain denied. Private content can be rebuilt offline but cannot
be served without live access checks.

Owner recovery is different from an AppView rebuild. The owner's encrypted export
must include the tenor's privileged Space and historical-writer inventory,
audience generations, recovery epoch, authoritative decisions and retained copies
allowed by policy. The AppView may supply archived evidence to an explicitly
authorized recovery process. Its cache must never become a new World authority.

Recovery on a fresh tenor rotates keys, reacquires credentials and reconciles
surviving repos. Old leases are invalid. An older export declares a new recovery
epoch. Unresolved admissions remain unresolved, and possibly used roll slots must
not reopen. If both the old tenor and an author's PDS are gone, recovery can
return only policy-permitted copies in a surviving export.

Direct member delegation preserves member identity on the AppView path. It does
**not** fix World-issued delegations sent to another authority. Before private
release, the tenor must refuse those remote delegations. Keep local issuance only
where the authority can enforce the link to the requesting member. A self-hosted
tenor acting as both World authority and Space host remains supported. The
restriction is on delegation across authorities, not self-hosting.

Portable delegation between groups needs an upstream-compatible way to
authenticate the originating member and revoke access. A member DID in the request
body or an AppView allowlist cannot fix a remote host. Defer that arrangement and
refuse it explicitly, rather than offering revocation that may not work.

## Rejected alternatives

- **App-only, owner-wide or World-wide read credentials:** bypass member expiry,
  expose more content if compromised, and can make Alice's access depend on Bob's
  activity.
- **Use cached membership to authorize live reads:** delayed sync and rebuilds
  could restore permissions that the authority removed.
- **Treat notifications or `listRepos` as a complete inventory:** misses first
  posts during outages and deletions by former writers.
- **Put private snapshots in public `SpaceRecordEvent` entries:** consumers assume
  those entries are safe to serve anonymously.
- **Separate storage without encryption or erasure-ledger support:** backups could
  silently restore content that should have disappeared.
- **Treat STAR MACs as permanent public signatures:** contradicts Spaces'
  deniable authentication model.
- **Start with incremental sync:** adds verification and crash-recovery complexity
  before the full private path works.

## Owner decisions before acceptance

1. Approve direct member delegation and stopping service when the last usable
   session disappears. Members must consent again with Space scopes; this is a
   visible product change, not a transparent session upgrade.
2. Approve refusing World delegations to remote authorities. Do not promise
   secret casts or portable group-to-group delegation in First World.
3. Confirm the maximum 60-second window for revocations not yet observed. Shorter
   bounds increase authority traffic. A future dedicated access-check query needs
   its own authenticated, versioned specification.
4. Choose archive key custody, backup purge deadlines, quotas and retention
   disclosure. Decide whether cached copies survive loss of the final session,
   for how long, and who may request deletion. Default to tombstones rather than
   retaining deleted prose.

   The full erasure design needs a separate bucket/account, witness service, keys,
   restore checks and purge operations. An alternative for closed alpha is to
   keep deletion tombstones and defer the ledger machinery until before open beta.
   That alternative cannot promise erasure from old backups or safe automatic
   rollback restores. It requires explicit retention disclosure, no automatic
   restoration to serving, and a reviewed manual purge/restore process for
   required erasures. Legal erasure duties do not wait for beta. If the manual
   process cannot meet them, real private data must wait for the full design.
   This alternative does not complete slice 6's compaction/erasure acceptance row.

5. Choose a measured reconciliation interval and outage recovery objective.
   Five minutes is a suggested starting interval, not a demonstrated service level.
6. Confirm the release prerequisites: tested encrypted owner export/restore and
   breaking-alpha migration support, alongside risk disclosure. Ingestion alone
   does not make real private play ready.

## Revisit when

Revisit when Spaces reaches its reviewed production release, upstream supports
portable delegation with authenticated member identity, repeated reader credential
issuance has a measured cost, or private snapshot sizes justify verified
incremental sync. Revisit storage when compaction, erasure rules or multi-node
deployment change its assumptions.

Implementation order and test obligations are in
[the private Spaces plan](../architecture/private-spaces-plan.md).

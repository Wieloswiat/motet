# 0024. Save dice results so retries cannot roll again

Status: Proposed

## Context and implementation when proposed

The tenor should issue a signed dice result that survives retries, crashes and
World moves. A retry must return the same result, not offer another chance at a
better roll. For a restricted action, changing the request ID must not provide
another chance either.

This proposal would store the result before returning it and give each restricted
action a single-use roll slot. The tenor still sees results first and remains
trusted. As the plan's [Dice contract](../plan.md) explains, authenticating the
issuer does not establish fair randomness.

When this proposal was drafted, the code supplied the following parts, not an
issuance flow:

- `crates/dice` parses integer expressions and evaluates them through `Source`.
  `GRAMMAR_VERSION` is `motet-dice/1`. `Limits` bounds generated values and die
  sides. Extra dice from explosions count toward the value limit; arithmetic
  overflow is an error.
- `tenor::dice::OsSource` uses operating-system randomness and rejection sampling,
  which avoids the bias of taking a random word's remainder. Entropy failure
  currently panics, and `Source` cannot return errors. Issuance needs an explicit
  failure policy instead.
- `rules::world` has admission retry handling and places scoped to an epoch, but
  **no roll commands, opportunities or slot state**. Generic operation retry tests
  in `world_properties.rs` do not test restricted-roll retries. The acceptance
  table then marked roll slots as unmodelled.
- Draft `roll.request` can reference an opportunity, but there is no opportunity
  lexicon. Draft `roll.result` stores faces, kept flags, a total or a failure. It
  lacks audience, slot, recovery and signature fields. Generic World writes to
  roll results are already forbidden by `tenor::authorization::protected`.
- `tenor::shard` logs effects before writing SlateDB or returning outcomes.
  `publication.rs` captures the audience, refuses stale Space generations and
  atomically commits publication receipts. Its intents cover only admissions and
  closings; dice needs new support.
- atproto-crates revision `09a868cdde938c789326f1db16ffc30bbf38c15e` supplies
  `atproto-attestation`. Its CID-first inline signature signs the record content,
  metadata and repository DID. Here a CID is a content identifier; CID-first means
  computing the content's CID before adding the signature. `verify_record` alone
  does not check that the claimed issuer owns the signing key. That needs a
  separate check.

Since drafting, `rules::dice` and its property tests have added the pure request
and slot transitions. Live issuance, signing, publication and export are still
unimplemented. That partial implementation does not accept this proposal or
settle its owner decisions.

This builds on ADRs [0014](0014-tenor-state-in-slatedb.md),
[0018](0018-world-service-auth.md), [0021](0021-single-use-member-tokens.md)
and proposed [0022](0022-private-space-ingestion.md). The proposal itself changes no runtime
behavior and claims no executed tests. The requirements below remain proposed,
including the open [owner decisions](#owner-decisions-before-acceptance).

## Proposed approach

### Identify the request and reserve one roll per restricted action

#### A request identifies one attempt

A signed-in member asks through the AppView. Independent clients use the same
tenor procedures. The AppView gets a fresh member service token addressed to the World for the
exact method. The tenor takes the actor from the verified issuer
and consumes the token's JTI (unique token identifier) in the shard transaction,
as ADR 0021 requires. A retry gets a new token but keeps the request identity and
operation ID.

A request is an immutable record in the author's public repo or the scene's
Space repo. Extend the draft lexicon with its creation epoch and explicit
audience identity: public or a particular Space. Put both in the record the tenor
reads back, not just in RPC arguments. The AppView writes the request create-only. If the PDS response is
uncertain, reconcile it before asking the tenor to issue the roll.

The tenor reads the exact CID back and checks author, World, scene and audience.
A stream or notification may also discover a request, but must use this same
verified read-back and issuance path, not a second roller. Private writes depend
on ADR 0022's conditional personal-PDS writes and verified private ingestion.

#### An opportunity permits one restricted roll

Add an immutable, World-authored **opportunity** record. For example, a GM could
open one roll slot for Alice's attempt to unlock a door. A second request for
that slot cannot produce a second roll. In First World, only a permitted GM
procedure opens opportunities. Versioned game rules may later open them through
the same transaction.

Each opportunity fixes:

- World, scene and audience;
- recovery epoch and a strong reference to the action, naming its exact version;
- actor, slot name, canonical expression and grammar;
- rules version and an exact character-sheet reference, or explicit absence.

One opportunity represents one slot. Its unique restricted-slot key is
`(World, action identity, actor, slot name)`. The epoch records where it came
from; it is not part of that key and cannot reopen the action after recovery.
A legitimate new attempt needs an explicit new action or adjudication record
linked to the old one.

Changing an opportunity's expression or context needs an explicit cancellation
or supersession decision. Editing or deleting it cannot reset the slot. A request
cannot open an opportunity and must match every field fixed by the opportunity
it references.

Free-form rolls have no opportunity. Label them free-form; they cannot later
satisfy a restricted action. In First World, any current, unsuspended cast member
may request a free-form roll in an open scene. Only the opportunity's actor may
consume a restricted slot. Check those facts in the same shard transaction that
reserves the request, not merely before PDS I/O.

#### Preserve request and slot identities

The stable request key is `(audience identity, author DID, collection, rkey)`.
The `rkey` is the request record's key within its collection. Freeze the first
accepted CID. Rewriting that record key or changing an operation's payload is a
conflict, not a new roll. Deleting and recreating a request does not reset its
receipt.

Keep request and slot receipts through compaction for as long as replay or action
references can reach them. Bound admission rate and queue size. Reject overload
before acceptance instead of evicting the state that prevents duplicate rolls.

The first eligible request reserves a restricted slot. Refuse competing requests
durably, pointing to the winning request only when the caller may read it. Every
accepted request has one terminal result. Every restricted slot has at most one
accepted roll outcome, whether rolled or failed. Unauthorized, malformed-envelope
and competing requests receive refusals, not invented dice results. Refusals must
not expose private slot details to unauthorized callers.

### Record progress before drawing or publishing

Keep deterministic transitions in a small `rules::dice` module. The tenor owns
randomness, signing and persistence. Persist dice state, indexes and publication
intents together as shard `Write` effects. Never put an operating-system random
draw inside a replayed `Entry::Apply` command: replay must not roll again.

Use these states:

1. **Accepted.** Atomically reserve the request, restricted slot, fixed result
   rkey, audience/generation and input versions. Store the operation receipt.
   Nothing has been drawn, so a restart can resume here.
2. **Drawing.** The single fenced worker durably claims the request before asking
   for entropy. No other worker or retry may draw it. Evaluate once within the
   limits and append the complete exact outcome or failure as effects.
3. **Outcome recorded.** Keep all result fields, timestamps and generated faces.
   Signing and publication may retry these stored bytes. Evaluation may not.
4. **Published.** Attest the fixed result with the World signing key and commit
   it to the captured public repo or private Space. Atomically store signed bytes,
   repo/Space effects and the publication receipt in the shard. Only after that
   log commit may an endpoint return a terminal result and its exact reference.

If a process dies in `Drawing`, the new worker cannot know whether it had already
sampled a result. First use fenced takeover to establish that the old worker
cannot commit. Then record an `interrupted` terminal failure without sampling
again. The slot remains consumed. This may lose an occasional roll, but a retry
cannot turn a crash into another chance.

A client timeout does not cancel work or create an `interrupted` failure. Return
**pending** and query the same request until the server resolves it. An expired
HTTP request is not evidence that the worker is lost.

Reconcile uncertain log appends with `motet-log` before takeover or further work.
If durability remains unknown, stop the shard. Never assume that a draw was not
recorded.

If signing, storage or publication fails after evaluation, keep the exact outcome
pending publication. Do not replace a successful but unpublished outcome with a
new failure. Audience deletion or incompatible recovery can block publication.
Show authorized callers only a status without content; never fall back to public
publication. Membership removal after acceptance does not erase the reserved
outcome, but reading it still requires current audience authorization.

### Sign and verify the whole result

#### Sign the result, including private results

Use one inline World attestation for both public and private results. Public repo
commits already authenticate published records. The extra signature lets someone
verify an exported result without a Merkle Search Tree (MST) inclusion proof from
the repo. Private Space commits use deniable MACs and cannot provide that portable
issuer proof.

This is a deliberate privacy trade-off: someone holding a private dice result
can show the World's signed statement to outsiders. It does not add author
signatures to private posts. Never put a private attestation or its contents in
the public repo.

Extend the result lexicon so the signed content fixes:

- World, scene and exact audience/Space identity;
- request strong reference and author, and opportunity/slot for restricted rolls;
- authority revision, recovery epoch and result rkey;
- grammar, canonical expression, algorithm version and limits policy version;
- complete outcome, with inline signatures in the resulting record.

Use the pinned CID-first format. Its metadata names the World issuer and explicit
public signing key; its repository DID is the World's DID. Sign with the
tenor-held World account key, `keys::Purpose::Signing`, sealed under the tenor
master key. This is **not** the owner's PLC rotation key. The owner keeps that
key and uses it to authorize account control and key rotation, not to sign each
roll. The AppView holds neither World key.

Do not sign only a total or an unversioned JSON serialization. Define a new Motet
attestation type with an allowed schema. Regenerate lexicons instead of putting
fields in extras.

The signed record's repo CID differs from the attestation's content CID. Preserve
both; the signature must not depend on its own final CID. Record which faces
caused explosions, alongside faces and kept flags, so a verifier can recompute
the trace and total. The draft result already has `kept` but lacks this `exploded`
information. Success and failure shapes must be exclusive, with bounded groups
and values.

#### Publish only the stored outcome to its original audience

Extend `publication.rs` with a dice intent. Reuse its captured destination,
receipt and fenced transaction; do not represent a roll as an admission. Use the
rkey allocated at acceptance and the stored outcome. A retry must not generate a
new timestamp or expression. An unknown or stale audience publishes nowhere.
Keep the request and outcome history even if the author later deletes the request.

Order dice with a durable World event sequence within the recovery epoch, not
client time. Define a scene event sequence/cursor when adding dice between posts
in the UI. Do not silently use admission place numbers that were defined only
for posts.

#### Verify content and the World's authority to sign it

Before archiving, AppView ingestion checks the exact record CID, inline signature,
World issuer/key binding and audience. A nonempty signature array is insufficient,
as is a valid signature by an arbitrary key. Independently recompute the grammar,
trace, kept decisions and total. Treat unknown algorithm or grammar versions as
unsupported; never interpret them using defaults. A failure has a typed code and
no invented total.

Also check repository provenance and opportunity matching. Copying a valid signed
record does not grant access, create a second slot result or move it to a
different scene.

At live ingestion, resolve the World and check its authorized signing key. Archive
the resolved identity/key evidence with the exact signed record. Looking up only
the current key would fail after migration rotates it. As in ADR 0007, offline
rebuild trusts previously verified archive evidence. It must not treat a result's
timestamp as proof of historical key ownership.

The tenor installs the World signing key as PLC's `#atproto` verification method
in `plc_genesis.rs`. Starting from a stated trusted checkpoint, accepted PLC audit
history establishes when the key was authorized. A historical verifier checks
the signed creation time against that interval and verifies the World/key
binding. This trusts directory time and accepted operation history. A timestamp
chosen by the signer does not independently establish signing time; it cannot
settle a compromised old key's backdated claims.

Independent clients may fetch records through public sync or authorized Space
reads and check the current issuer/key binding locally. Rotation or migration
changes neither old attestation bytes nor the mathematical validity of their
signatures. Preserve old public keys and the verified evidence observed at
ingestion. New results use the newly appointed tenor's World signing key. Never
replace old signatures silently.

“Signature valid, historical issuer binding unavailable” is a fallback when
directory/history evidence is missing, not the normal migration outcome. An
embedded `did:key` alone does not establish that the key belonged to the World.

First World must preserve and restore exact committed results and trusted archive
evidence. A self-contained export verification bundle, packaged PLC history and
checkpoints, external profile fixtures and an independent historical verifier
are post-First-World work in slice 5. A milestone backup is not already such a
third-party verification bundle.

A signature authenticates the World's statement. It does not establish that the
operator behaved fairly or never discarded unseen draws. Keep `os-csprng` and a
versioned evaluation/sampling policy explicit. Do not advertise verifiable
randomness. Do not export a random seed or use one to regenerate results.

### Limits and failures

Check HTTP/body and record-size bounds before acceptance. Once an authorized,
well-formed request is accepted, invalid notation, unsupported dice, exceeded
face/side/group/output limits and arithmetic overflow must produce one signed
terminal failure. Validate restricted expressions when creating opportunities too.
Explosion limits can still be exhausted at runtime; that failure consumes the
slot. A corrected attempt needs an explicit GM decision and a new action, not a
retry.

Persist the effective limits policy at acceptance. Bound total generated values,
result groups and bytes, parser depth and length, and integers on the wire.
Per-array lexicon limits alone are insufficient:

- The total generated value cap must not exceed either the enforced
  `Limits::max_values` or the result lexicon's faces/kept `maxLength`. First World
  caps all three at 1,000.
- Allow at most 64 groups. Within each group, faces, kept and exploded arrays
  must have equal lengths.
- The 1,000-value limit is a total, not an allowance for each of 64 groups.
  Test these limits through a consumer of the encoded result.

Bound the number of entropy words used in rejection sampling. Never substitute
biased remainder sampling when that budget runs out. Add a fallible source and
evaluator path that reports OS entropy failure and budget exhaustion without
panic or a partially successful result. This deliberately changes the current
infallible `Source` interface. Keep the unbiased-sampling tests.

Infrastructure failure before durable acceptance remains retryable. Once drawing
starts, failure must never allow another entropy attempt for that request. This
includes an entropy error before the first accepted face: it permanently consumes
a restricted slot. That particular error does not allow result fishing, but using
one terminal rule avoids relying on volatile evidence about whether a face was
drawn before a crash. A bounded retry with zero faces would need its own durable
attempt counter and precise crash rules. First World proposes the simpler,
conservative rule and must disclose that cost.

### Show the same result after retries and recovery

#### A retry resumes the original request

Add procedures to create requests and query status, an AppView outbox kind and
session-scoped result queries. The outbox keeps request rkey, CID, operation ID
and pending status across lost responses. A retry button resumes that request;
“new free-form roll” is a separate deliberate action. Save pending state in the
browser so navigation and reload cannot issue another request.

Show pending, rolled, failed, publication-blocked and recovery-unresolved states
distinctly. Display expression, faces/kept trace, total, grammar and World
verification status. Do not animate a new local outcome or mark an unverified
HTTP response as final.

Index only exact World-authored results, matching requests and opportunities by
identities that include their Space. Public results use public ingestion. Private
results need ADR 0022's verified archive and per-reader access checks, including
historical versions and later downloaded proof bundles. Bump `PROJECTION_VERSION`
for projection changes and add migrations for outbox/service changes. Rebuild must
not obtain entropy, issue rolls, sign records or send notifications.

#### Recovery must not reopen an uncertain slot

Export all opportunities, slot reservations, immutable request identities,
`Drawing` markers, exact outcomes/signatures, publication intents/receipts,
identity evidence and the recovery epoch. A complete planned move transfers this
state before the new writer starts and returns identical committed results. An
old client's query keeps its original identity after migration.

An older export cannot establish that a slot shown as open was never used.
Declare a new recovery epoch. Freeze every pre-checkpoint opportunity whose fate
after the checkpoint lacks evidence, including opportunities shown as open in
the export.

Reconcile the old log, published World records, permitted AppView archive copies
and submitted signed receipts. A recovered authenticated result consumes the
original slot. Never sample it again or re-sign it as a new roll. If the new
epoch needs a record, publish a recovery decision referencing the old result.
Conflicting signed outcomes remain disputed; do not choose the more favorable one.

Without surviving evidence, keep the old slot `recovery-unresolved`. A GM may
record an attributed adjudication of the game consequence, but cannot silently
clear the slot or convert uncertainty into a new random result. New actions after
recovery must be explicit and linked to that adjudication.

Old-epoch requests absent from the export stay unresolved; they do not become
fresh free-form rolls. The API must bind requests to their creation epoch.
Changing the recovery epoch does not reset duplicate-request protection. No
system can reconstruct an unexported result once every copy is gone.

## Implementation order

The following stages record the proposed work and test obligations. The pure
core in stage 1 has since been implemented; the remaining integration work is
still required. This list is not evidence that any test ran. Each test must name its property and plausible bug, following
[testing.md](../testing.md).

### 1. Pure request/slot transitions (one to two agent-days)

Add `crates/rules/src/dice.rs` and focused property tests. Model reservation,
starting a draw, recording an outcome, publication receipts and freezing recovery
state without randomness, HTTP or storage. Do not expand the general admission
command enum just to inject randomness.

Test that many distinct requests for one slot yield at most one accepted outcome,
retries preserve receipts, changed payloads conflict, and interrupted or recovered
slots cannot reopen. These catch duplicate protection based only on operation ID,
or a failure that clears the slot. No running stack is needed. This supplies the
core of “Many requests for one restricted roll slot”, not its end-to-end coverage.

### 2. Durable issuance and signed publication

Add tenor dice operations, store keys, shard effects, authorization, publication,
new opportunity/procedure/attestation lexicons and updated request/result lexicons.
Run `lexgen`. Add a fallible, bounded source interface in `crates/dice`.

Exercise real signed member tokens and recorded shard effects: concurrent
requests, removal before acceptance, forced entropy error, process loss in
`Drawing`, uncertain log append and publication retry. Assert no second source
invocation after a durable `Drawing` claim and identical bytes after replay.
These catch replay that rerolls or retries that regenerate result metadata.
Test signature tampering, false issuer keys and wrong audiences for public and
private results.

Fault cases use real S3; pure cryptographic and transition tests need no stack.
This covers the tenor portion of the restricted-slot acceptance row. Private
serving still depends on ADR 0022.

### 3. AppView and visible roll journey

Add AppView auth scopes, writes/outbox, API, ingestion/archive and projections;
web composer/status/result UI; and a production Playwright journey. Test a lost
HTTP response and page reload while a roll completes. The reader must see the
same signed result, not a second request. This catches a client that generates
a new rkey on retry. Race two clients for one opportunity and check that there
is one winner.

Private results must become inaccessible after reader revocation. Tampered
outcomes must never appear verified. Proof-bundle export checks belong to slice 5.
Use PostgreSQL/S3 and the full TLS/PDS/tenor/web stack. The restricted-slot row is
complete only once both public and private paths run.

### 4. Export, migration and older-checkpoint recovery

Extend the owner export/recovery format, not the browser key-backup file. Add
isolated acceptance tests for “Return a roll, lose the client, migrate” and
“Restore an export older than a completed roll”. Stop the old tenor after showing
a result, then restore current and old exports separately.

The current export must return the identical result. The older export must
recover the old signed result if evidence survives, or leave its original slot
unresolved. Rebuild with network and entropy disabled. These catch resetting a
slot on a new epoch, leaving pending publication out of an export, or replacing
an old signature after migration.

Use the running stack with isolated fresh storage, never destructive tests
against real Worlds. Owner export work is a dependency, not part of the
one-to-two-day stage. These milestone rows do not require an independent
third-party key-history bundle.

### 5. Portable historical verification bundles (after First World)

Package PLC operation history, trusted checkpoints, signing-key intervals, exact
records and profile fixtures in an optional export verification bundle. Add
independent historical verification instructions/tooling and an authorized
AppView download path.

With a separate consumer, test old signatures across key rotation, claimed
creation times outside the key's PLC interval, incomplete history and mismatched
checkpoints. These catch accepting any valid `did:key` as the World or requiring
the current key for an old result. Test private bundle denial after revocation
through the real API. Do not present signature validity as independent evidence
of signing time or fairness.

Cryptographic and fixture tests need no stack; download and migration tests do.
This extends portability beyond the milestone's promise to preserve identical
committed results.

## Rejected alternatives

- **Client rolls or optimistic totals:** do not provide durable World-issued
  outcomes and let retries select a favorable result.
- **One result per operation ID only:** changing the ID could bypass a restricted
  slot or reroll an edited request. Preserve request identity and slot uniqueness.
- **Random draws inside replayed rules commands:** recovery would change outcomes.
- **Commit a sealed seed at acceptance, then evaluate deterministically:** a viable
  alternative, not a reroll. It avoids `Drawing` and interrupted failures and can
  replay without fresh entropy. But it needs a pinned seeded pseudorandom-number
  generator (PRNG), fixed byte-consumption rules, a versioned evaluator and limits,
  and compatibility fixtures. A pending request's seed must survive key rotation
  and export without exposing future outcomes. The current direct-OS `Source`
  defines none of that. First World instead proposes storing exact outcomes and
  accepting rare interrupted failures, so recovery does not need an old evaluator
  to reproduce results. Revisit if interruptions become operationally significant.
  Committing a seed alone does not establish fair seed selection.
- **Return a result after drawing or shard reservation:** reservation is not
  publication. Return pending until signed repository publication commits.
- **Reroll after timeout or interrupted execution:** uncertainty becomes another
  chance. Interrupted execution must consume the slot with a failure instead.
- **Public commit proof only:** portable exports need retained inclusion proofs,
  and private Space results still lack a signature.
- **A separate public proof record for private rolls:** leaks private context.
  Keep the inline attestation with the same audience as the result.
- **Commit/reveal, an external randomness beacon or a verifiable random function
  (VRF) now:** require more trust and availability design than First World needs.
  A signature supplies none of these mechanisms.

## Owner decisions before acceptance

1. Approve GM-opened opportunities first, separate free-form cast rolls, and
   explicit versioned game-rule automation later. Choose the concrete action and
   character-sheet references the initial game UI should expose.
2. Approve failed or interrupted rolls permanently consuming their restricted
   slots, including OS entropy failure before any face is drawn. This trades
   zero-face retries or committed seeds for simpler crash recovery. A GM may
   adjudicate, but “try again” cannot erase a failure or unknown prior outcome.
3. Approve portable World signatures for private dice. A recipient can show the
   signed result to outsiders. Ordinary private posts keep Spaces' deniable
   authentication model.
4. Choose effective limits, per-member queue/rate quotas and a UI wait budget.
   Ten seconds before switching to pending polling is a suggestion, not a
   cancellation deadline. Measure before claiming a latency objective.
5. Confirm that older-export recovery stays unresolved without evidence, even
   when it delays play. Choose the GM adjudication workflow and export frequency.

## Revisit when

Revisit when a game needs automatic opportunity creation, measured shard latency
requires an external evaluation worker, or users need independently verifiable
randomness. Any worker must preserve the durable `Drawing` claim and single
outcome. Before changing the atproto-attestation pin, review the signature format
with fixed fixtures and historical issuer-key verification tests.

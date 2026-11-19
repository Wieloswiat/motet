# 0018. Use member service tokens for World actions

Status: Accepted. Supersedes the World-session and tenor OAuth-server parts of
[0013](0013-tenor-hosts-worlds.md), and the group OAuth-server assumption in
[0015](0015-opensocial-group.md).

## Context

The tenor hosts a World's identity, repositories and Spaces. It must still
know which person took each action as the World.

[0013](0013-tenor-hosts-worlds.md) chose tenor-issued OAuth sessions for the
World DID after the GM signed in at their own PDS. That needs another OAuth
server and login. A session could also retain authority after the GM loses
the role that allowed it.

Tangled's [knot architecture](https://tangled.org/tangled.org/core/blob/9934837f0a9153d87a9558c938b1e654109556cf/docs/src/content/docs/architecture/data-flow.mdx)
uses a simpler approach: when its AppView calls a knot, it sends a short-lived
service token signed by the person's PDS.

## Decision

Require a member's PDS service token for each World action, rather than issue
OAuth sessions for the World.

The AppView authenticates the person at their PDS and obtains a service-auth
JWT for the exact XRPC method it will call. Other clients can obtain and
submit the same tokens. The tenor verifies the issuer's `#atproto` signing
key, token lifetime, audience and `lxm` method claim. Only the verified `iss`
claim identifies the acting member; a DID in the request body never does.

For World-scoped calls:

- Require the audience to be the hosted World's DID, either bare or with
  `#motet_tenor`. Select the World from that audience. Refuse an unhosted
  World or a conflicting World, repository or Space target in the request.
- Support standard atproto-proxy calls, where the PDS uses the proxied DID
  as the audience. Always require `lxm` to equal the called method.
- Before the World exists, require the tenor's bare service DID for
  `reserveWorld` and `createWorld`. Locally this is
  `did:web:tenor.motet.test`. Percent-encode a port in a `did:web` authority
  as `%3A`.

In the shard transaction for each mutation, read the actor's current
membership, roles that still exist and opensocial grants. Write an audit
record identifying the member in the GM Space in the same transaction.
Removing a role must remove its authority on the next write, even if an older
token is still valid. For author-driven post admission, also check the current
scene cast and sanctions.

Open public scenes through `at.motet.pilot.play.openScene`. Always include
the opener in the cast and publish the cast in the public scene record.
Change scenes through typed methods, not generic record writes. Default
owner and GM grants include the scene procedures. To let players open
scenes, add `rpc:at.motet.pilot.play.openScene` to their role's grant in
`group.opensocial.permissions` in the World's members Space. Grant cast
changes and scene closure separately if desired.

The tenor does not authenticate members at their PDS, issue World OAuth
sessions or mint World tokens for downstream services.

## Owner signature

The owner supplies a public P-256 or K-256 `did:key` as the first PLC rotation
key. `reserveWorld` returns `signingBytes`: the unsigned PLC genesis operation
serialized as canonical DAG-CBOR, then encoded as unpadded base64url. It
includes `prev: null` and excludes `sig`.

For P-256:

1. Decode `signingBytes` and sign those bytes with ECDSA and SHA-256.
   WebCrypto hashes the message itself; do not hash it first.
2. Encode the signature as 64-byte compact IEEE P1363, not DER: a 32-byte
   big-endian `r` followed by a 32-byte big-endian `s`.
3. Normalize `s` to the lower half of the P-256 group order. The tenor rejects
   a high-S signature.
4. Encode the signature as unpadded base64url and add it as `sig` to the
   otherwise unchanged operation.

The tenor verifies the signature against the first rotation key.

## Consequences

Role changes take effect on the next write. Audit records identify the member
without relying on a shared World credential. For each action, the caller or
AppView must obtain a new token for that method from the member's PDS.

The tokens are bearer credentials: anyone holding one can use it during its
short lifetime. Clients and services must protect them. Third-party clients
cannot obtain OAuth sessions whose subject is the World; they must use the
member-token procedures instead. Ordinary AppView sign-in sessions remain
separate from World authorization and are not issued by the tenor.

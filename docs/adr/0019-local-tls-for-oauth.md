# 0019. Use HTTPS for local sign-in

Status: Accepted. Supersedes only the plain-HTTP local endpoint assumptions in
[0016](0016-local-atproto-network.md) and
[0017](0017-rust-pds-local-network.md). Their other decisions remain in force,
including 0017's PDS coverage and relay patch scope.

## Context

The AppView acts as a confidential OAuth client of the user's PDS: it, not
the browser, exchanges tokens with the PDS. Its client metadata, callback and
the PDS authorization server need stable HTTPS origins. The pinned atproto
validators reject `.localhost` service names but accept `.test`. HTTP
loopback URLs and a `did:web:localhost` with a port cannot serve this setup.

## Decision

Use a locally generated development certificate authority (CA) and HTTPS on
port 443 at these canonical origins:

- `https://motet.test`
- `https://pds.motet.test`
- `https://tenor.motet.test`
- `https://plc.motet.test`
- `https://relay.motet.test`
- `https://tap.motet.test`

Configure scoped wildcard DNS to send World and PDS account handles to the
local proxy. Route `*.motet.test` World handles to the tenor and
`*.pds.motet.test` account handles to the PDS. On `motet.test`, route
`/xrpc/*` and `/oauth/*` to the AppView and other web paths to SvelteKit.

This gives the UI and AppView one origin. The PDS authorization page stays
on the PDS origin, and only the AppView exchanges OAuth tokens with it.
Readiness checks for local services may still use HTTP loopback.

Allow private-network requests only by an explicit local-development opt-in.
Production requests must keep HTTPS, address filtering and certificate
verification. Make local clients and Chromium trust the development CA;
do not disable TLS verification. Configure the PDS as `pds.motet.test` with
DID `did:web:pds.motet.test` and trust only the reverse proxy's single hop.

## Consequences

Setup changes host DNS, port permissions and certificate trust. It creates
private keys in the dev environment's state directory. The local CA can issue
certificates trusted on that host; neither it nor the server keys belongs
in the repository or production. Port 443 must be free. The local-network
guide documents setup, trust, state reset and cleanup.

Choosing this setup does not establish that the OAuth journey or compose
deployment has passed an end-to-end test.

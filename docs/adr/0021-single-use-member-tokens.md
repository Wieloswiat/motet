# 0021. Accept each member service token only once

Status: Accepted. Narrows the bearer-token assumptions of
[0018](0018-world-service-auth.md).

## Context

A service token names a method but does not bind its request body. Someone
who obtains it could replay it with different arguments. The resulting audit
record would attribute another action to the member who obtained the token.
A short expiry limits the opportunity but does not prevent that replay.

## Decision

Accept each service token only once for a member-authorized World mutation,
including World reservation and creation.

Require a nonempty `jti`, the token's identifier. In the same shard transaction
as the mutation, record the verified issuer and `jti`. Refuse reuse of that
pair until the token expires. An aborted transaction does not consume the
token. Limit token lifetime to 60 seconds and allow expired replay entries
to be removed.

For a retry, clients must obtain a fresh service token even if the operation
ID and body are unchanged. The operation ID still returns the original
outcome for an unchanged retry; the token authorizes one attempt. Read-only
queries and Space protocol notifications do not consume this mutation token.

## Consequences

The record of used tokens survives shard replay with the mutations. A used
token cannot authorize a second mutation, even with a different body.

This does not bind a token to a body. Someone who steals an unused token can
still spend it first. Clients and services must protect tokens in transit
and avoid logging them.

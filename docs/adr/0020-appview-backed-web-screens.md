# 0020. Use real AppView data on working screens

Status: Accepted. Supersedes the demo-route and demo-data decisions in
[0012](0012-design-system-and-demo-screens.md); its design-system and Theme
Creator decisions still stand.

## Context

The AppView now supplies browser sessions, public World and scene projections,
and methods to create Worlds, manage scenes and post. The browser holds the
owner's World rotation key. Sample-data screens cannot show whether these
actions were authorized or a post was admitted.

## Decision

Use the AppView for sign-in, World creation, World overview, scene lists and
scenes. Never fall back to sample Worlds or posts on real routes. Keep the
remaining design demos and label them on `/design`.

Send browser mutations to the shared HTTPS origin with the session cookie.
For server-side loads, forward cookies only to the configured
`MOTET_APPVIEW_PUBLIC_ORIGIN`, never to an origin taken from the request's
Host header.

Keep the owner key and backup passphrase in the browser. Do not mark setup
complete until the user has downloaded the encrypted backup, re-imported it
and verified it.

Use Motet's default appearance on real screens. World theme records are not
yet projected, so a demo theme would misrepresent the World. When those
records become available, compile the real theme on the server for the
first paint.

## Consequences

Readers see AppView projections, not immediate results of writes. A newly
created World, scene or admitted post may take time to appear. Gallery tests
cover only screens with sample data; browser journeys cover the real routes.

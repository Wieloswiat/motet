# Architecture decision records

These records explain what we chose and why. Each has a status:

- **Proposed:** not approved yet.
- **Accepted:** the agreed decision.
- **Provisional:** a working choice we expect to revisit.
- **Superseded:** replaced by a later decision, linked from the record.

Use a new record to change an accepted decision, and link it from the old one.
Wording can be clarified in place, but must preserve the original decision, its
status and the reasons for it. Explain technical terms where they are needed;
prefer concrete actions and examples to new jargon.

| ADR                                            | Decision                                                             | Status                                                             |
| ---------------------------------------------- | -------------------------------------------------------------------- | ------------------------------------------------------------------ |
| [0001](0001-licence.md)                        | License Motet under MIT                                              | Accepted                                                           |
| [0002](0002-nix-and-devenv.md)                 | Use Nix and devenv for builds, tools and local services              | Accepted; shell entry point superseded in part by 0025             |
| [0003](0003-workspace-layout.md)               | Keep Rust and web packages in one repository                         | Accepted                                                           |
| [0004](0004-atproto-crates.md)                 | Use atproto-crates for protocol support                              | Accepted; crates.io half superseded by 0013                        |
| [0005](0005-testing.md)                        | Test system behavior with property, integration and acceptance tests | Accepted; retry result superseded by 0011                          |
| [0006](0006-storage.md)                        | Store durable logs in object storage; use SeaweedFS locally          | Accepted; SlateDB added by 0014                                    |
| [0007](0007-projections.md)                    | Rebuild PostgreSQL views from the archive                            | Accepted                                                           |
| [0008](0008-rendering.md)                      | Render Typst in a separate, restricted process                       | Accepted                                                           |
| [0009](0009-web-stack.md)                      | Use SvelteKit, Panda CSS and Ark UI                                  | Provisional; design package amended by 0012                        |
| [0010](0010-lexicon-codegen.md)                | Generate Rust and TypeScript types from lexicons                     | Accepted                                                           |
| [0011](0011-flaky-tests-fail.md)               | Fail CI even when a test passes on retry                             | Accepted                                                           |
| [0012](0012-design-system-and-demo-screens.md) | Keep the exported design system and demo screens                     | Provisional; demo routes and data superseded by 0020               |
| [0013](0013-tenor-hosts-worlds.md)             | Host World accounts on the tenor                                     | Accepted; World sessions and tenor OAuth server superseded by 0018 |
| [0014](0014-tenor-state-in-slatedb.md)         | Use SlateDB with its own write-ahead log disabled                    | Accepted                                                           |
| [0015](0015-opensocial-group.md)               | Use a pinned version of opensocial.group                             | Provisional; group OAuth assumption superseded by 0018             |
| [0016](0016-local-atproto-network.md)          | Run the atproto network locally                                      | Accepted; PDS/patch by 0017; HTTP endpoints by 0019                |
| [0017](0017-rust-pds-local-network.md)         | Use the Rust PDS locally; defer testing the reference PDS            | Accepted; HTTP endpoints superseded by 0019                        |
| [0018](0018-world-service-auth.md)             | Authorize World actions with the acting member’s service token       | Accepted; bearer-token assumptions narrowed by 0021                |
| [0019](0019-local-tls-for-oauth.md)            | Use HTTPS for local OAuth                                            | Accepted                                                           |
| [0020](0020-appview-backed-web-screens.md)     | Load World and scene pages from the AppView                          | Accepted; demo routes and data supersede 0012                      |
| [0021](0021-single-use-member-tokens.md)       | Allow each member service token to be used once                      | Accepted                                                           |
| [0022](0022-private-space-ingestion.md)        | Read private Spaces and check each reader’s access                   | Proposed; would supersede part of 0018                             |
| [0023](0023-kani-proofs.md)                    | Try Kani on the Spaces access check                                  | Proposed                                                           |
| [0024](0024-durable-dice.md)                   | Save signed dice results and prevent repeat rolls                    | Proposed                                                           |
| [0025](0025-native-devenv.md)                  | Use native devenv for the shell and the flake for builds             | Accepted; supersedes shell clause of 0002                          |

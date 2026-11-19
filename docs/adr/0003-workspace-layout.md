# 0003. Keep Rust and web code in one repository

Status: Accepted

## Context

The backend uses Rust, as chosen in the backend plan's Language section.
The web app uses TypeScript. Both use the same lexicons, and a change often
needs code in both languages.

## Decision

Keep the backend, web app and lexicons in one repository.

- Use a Cargo workspace for `crates/*` and a pnpm workspace for `apps/*` and
  `packages/*`. Generate types for both from `lexicons/`.
- Separate crates by responsibility. Keep `motet-rules` and `motet-dice`
  free of I/O so their logic can be tested without services, including
  exhaustive tests where practical. Let `motet-log` own durable logging,
  `motet-net` own outbound networking and `motet-xrpc` own XRPC serving.
  The `motet-appview` and `motet-tenor` binaries combine these libraries.
- Put shared dependency versions and lints in the root `Cargo.toml`.
  Crates inherit them with `workspace = true`. Enable Clippy's pedantic
  group and treat warnings as errors.
- Configure Prettier, ESLint flat config and `tsconfig.base.json` once at
  the root for the web packages.

## Consequences

New crates inherit the workspace's dependency versions and lints. Code that
needs a network or database does not belong in `rules` or `dice`.

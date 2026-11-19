# 0025. Use native devenv for the development shell

Status: Accepted

Supersedes ADR 0002's single-flake entry point and flake-parts shell clause.
The hermetic checks, deployment packages and images remain flake outputs.

## Context

The flake-parts shell provides a compatibility wrapper, not the native
devenv CLI. Without root `devenv.nix` and `devenv.yaml` files, `devenv shell`
and `devenv info` do not work normally. Developers and CI need to run the
same commands through the native CLI.

## Decision

Use native devenv 2.4.0 to enter the development shell and run its commands.
Keep hermetic checks and deployment builds in the flake.

- Declare nixpkgs, rust-overlay and crane inputs in root `devenv.yaml`.
  Use root `devenv.nix` to build the toolchain from `rust-toolchain.toml`,
  Kani and network packages. Import `nix/devenv.nix` as the sole definition
  of scripts, hooks, services, processes and the service-backed gate.
- Enter with `devenv shell`, run scripts with `devenv shell -- <script>`,
  start services with `devenv up` and run the full gate with `devenv test`.
  For direnv, use `devenv direnvrc` and `use devenv`, not nix-direnv's
  `use flake`.
- Remove the flake-parts devenv module and compatibility shell. Keep crane
  checks, network packages, Kani and images in the flake. Remove the
  `devenv-up` flake package and use native `devenv test` in CI.
- Treat `flake.lock` as the source of truth for nixpkgs, rust-overlay and
  crane. Put matching revision URLs in `devenv.yaml` and commit
  `devenv.lock`. Reject different revisions or source hashes during shell
  evaluation, and compare the locks in a hermetic flake check. Keeping both
  locks lets native devenv manage its inputs without fetching the working
  tree as a local flake input or copying build output into the Nix store.
- Install the native CLI in CI through the official Nix/Cachix setup. Pin
  it to the 2.4.0 commit recorded for its modules in `devenv.lock`. Use native
  devenv for the gate, proofs and scheduled jobs. Keep `nix flake check`
  and `nix build` for hermetic checks and images.

## Consequences

Contributors install Nix and devenv; direnv is optional. `nix develop` is no
longer the shell entry point.

To update shared dependencies, update `flake.lock`, copy matching revisions
into `devenv.yaml` and run `devenv update`. A missed step fails the checks
rather than silently using different tools. Devenv's own module inputs have
separate pins in `devenv.lock`; deployment builds do not need those inputs.

Native shell startup must preserve the local service environment and put
repository scripts first, including Motet's `fmt` ahead of GNU fmt.

Local TLS setup still changes host DNS, CA trust and port permissions. The
new shell entry point neither removes that setup nor establishes that the
service-backed gate passes on an unconfigured host.

## Rejected alternatives

- Keep the flake-parts shell. Its wrapper does not provide the normal native
  CLI commands requested by the user.
- Import the local flake's packages into devenv. This makes shell startup
  depend on flake evaluation and requires locking the local source. Shared
  Nix functions already build the same packages without those dependencies.
- Keep independent locks without checks. A partial update could silently
  give developers and CI different toolchains or package versions.

## Revisit when

Native devenv can use one shared lock directly, or keeping the two locks in
sync costs more than the separation of development and deployment builds
is worth.

References: [inputs](https://devenv.sh/inputs/),
[YAML schema](https://devenv.sh/reference/yaml-options/),
[composition](https://devenv.sh/composing-using-imports/),
[direnv](https://devenv.sh/integrations/direnv/), and
[GitHub Actions](https://devenv.sh/integrations/github-actions/).

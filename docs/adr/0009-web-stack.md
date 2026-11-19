# 0009. SvelteKit, Panda CSS and Ark UI

Status: Provisional

## Context

The web app needs server-rendered AppView data and a strict Content Security
Policy, particularly because it displays untrusted posts. Each World must
also be able to change the appearance of its part of a page at runtime.

## Decision

Use SvelteKit, Panda CSS and Ark UI as the initial web stack.

- Run SvelteKit with adapter-node and Svelte 5 runes. Set the Content Security
  Policy in SvelteKit's configuration. Run Vitest component tests in a real
  browser and Playwright journeys against a production build.
- Put Theme Creator tokens in `@motet/design` as CSS variables. Its Panda
  preset refers only to those variables, and contrast checks cover every
  theme. A World replaces variables on its own subtree, so every World can
  share one stylesheet. Use Panda's strict tokens to require explicit
  exceptions for styles outside the design system.
- Use Ark UI for accessible components without imposed styling.
- Do not vendor webfonts until each font's licence is confirmed.

## Revisit when

The first real screens are built. Panda and Ark are provisional; replacing
them should affect only `apps/web` and `@motet/design`.

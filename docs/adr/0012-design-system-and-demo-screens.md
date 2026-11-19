# 0012. Import the design system and sample screens

Status: Provisional. Amends [0009](0009-web-stack.md): the design package
section below replaces 0009's description of `@motet/design`; the rest of
0009 stands.

## Context

The design system, app screens and Theme Creator were built outside this
repository as claude.ai design artifacts. The repository has only their CSS
variables and a small Panda preset that refers to them.

The exports include a Panda v2 preset with component recipes, an engine that
turns World theme records into CSS custom properties and ornaments, demo
content and twenty sample-data screens. The user wants these in the app now,
as a starting point for real features. At this decision, there are no AppView
reads, sign-in or posting behind the screens.

## Decision

Import the design system and port the screens as explicitly nonfunctional
demos.

- Replace `@motet/design`'s small preset with the exported `panda/` directory.
  Record its source version and local changes in `packages/design/UPSTREAM.md`.
  Build app and Theme Creator CSS with Panda rather than copy the export's
  prebuilt `bundle.css`.
- Emit every recipe with `staticCss: { recipes: '*' }`. The ported markup
  contains classes such as `m-button` and `m-chip__root` that Panda cannot
  extract. Remove this setting when components call recipes from TypeScript.
- Load the exported `Motet.mountWorld` engine lazily in the browser to apply
  a World's theme. Server-rendered demos initially show Motet's appearance;
  hydration applies the World's. Before real World pages ship, compile the
  theme on the server so the first paint is correct. The engine's functions
  work directly on a theme record and can run there.
- Make the screens a one-time port owned by this repository. Store Svelte
  components in `apps/web/src/lib/screens/` and sample-data modules in
  `src/lib/demo/screens/`, returned by each route's `load`. Do not retain the
  bulk converter: future behavior will be edited by hand, and re-exporting
  would overwrite it. Port shared page chrome, the World header, pickers and
  report dialog by hand. Buttons remain inactive; only a few menus and
  toggles keep local state.
- Organize routes for the app rather than the export's screen order. `/`
  shows shelves, `/w/[world]` holds World pages and `/design` lists every
  screen with a theme switch. Resolve only the demo World, scene and
  character; return 404 for other identifiers.
- Keep the Theme Creator separate in `apps/theme-creator`. Serve the exported
  static editor through Vite and share `@motet/design`. Leave its script as
  exported and outside lint so a later export can replace it.
- Do not import webfonts, as decided in 0009, or the Mgławica Mocy artwork
  borrowed by the export.

## Consequences

The app matches the design artifacts and gives each feature a starting
screen, but those screens do not demonstrate working features.

Keep the vendored preset, engine, demo and Theme Creator script outside lint
and formatting. Only make the local changes listed in their `UPSTREAM.md`.
Until fonts are added, text uses system fonts and icons display their
ligature names.

Use Playwright to check that every screen loads under the production Content
Security Policy without errors and that a World page paints its own art.
These tests do not establish that the depicted features work.

## Revisit when

A screen gains real data or behavior: remove its demo module. When components
call recipes from TypeScript, remove `staticCss`. When World themes render
on the server, stop loading the engine just to apply the first-paint theme.

# 0008. Render Typst posts in a separate, restricted process

Status: Accepted

## Context

Posts contain untrusted Typst code. The typesetting system can loop, consume
memory and load WebAssembly plugins. Running it in a server process would
put that server at risk.

The backend plan calls for typst.ts vector artifacts rendered in the browser.
That library uses a patched Typst version that can lag the upstream compiler.

## Decision

Compile posts in a separate, restricted `motet-render-worker` process managed
by `motet-render`.

- Start the worker with a clean environment, address-space and CPU rlimits,
  a wall-clock timeout and an output-size cap. These limits contain hostile
  code; a package denylist is not a substitute.
- Give the compiler access only to the post source, bundled fonts and
  Motet's package mirror. Fix its date to the post's date. Do not allow file
  or network access outside those inputs.
- Cache output by `RenderKey`, which includes source, compiler version,
  security-policy version and layout. A change to any of these must cause
  a new render rather than return stale output.
- Produce static SVG with upstream `typst` for now. Later, switch to typst.ts
  vector artifacts and its required Typst version. Keep the sandbox and
  worker supervisor when the format changes.

## Revisit when

Groundwork's typst.ts experiment has tested two widths, both themes and a
screen reader.

# 0005. Test system behavior, not copies of the implementation

Status: Accepted. The retry result is superseded by
[0011](0011-flaky-tests-fail.md).

## Context

Motet must catch disagreements between parts of the system: consumers showing
different scenes, replay losing acknowledged work, or a token authorizing
the wrong method. A test that repeats a function's logic can pass while these
bugs remain.

## Decision

Test the properties the system must preserve, not just examples of what
individual functions return.

- Use `proptest` to generate delivery orders, retries, failures and hostile
  inputs that could break those properties.
- Test PostgreSQL, S3 storage, signatures and HTTP routing with real
  implementations. Mark service-backed tests `#[ignore = "integration: …"]`
  and run them against devenv services locally and in CI.
- Track the backend plan's acceptance table in `docs/testing.md`. Put tests
  that span several parts of the system in `crates/acceptance`.
- Run `cargo mutants` weekly in CI on the core crates. Its deliberate code
  changes check whether tests detect incorrect behavior.
- Run the tests with nextest. Its `ci` profile retries a failure once and
  reports the retry so intermittent failures are visible.

## Consequences

Changes to rules or logging need tests for the property they preserve. If
code can only be tested with a mock, consider separating its pure logic or
testing it against the real service instead.

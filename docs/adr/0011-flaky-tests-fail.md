# 0011. A test that passes only on its retry fails CI

Status: Accepted. Supersedes the retry result in
[0005](0005-testing.md); the rest of 0005 stands.

## Context

[ADR 0005](0005-testing.md) gave nextest's `ci` profile one retry. By default,
nextest calls a test flaky if it fails and then passes, but still passes the
run. The summary and `junit.xml` show the failure, yet a green build gives
no reason to investigate it. An intermittent failure may be a race in the
product, not just a faulty test.

## Decision

Keep the retry for diagnosis, but fail CI when a test needs it to pass.

- Set `flaky-result = "fail"`, which requires nextest 0.9.131 or later.
  JUnit then reports the test as a failure and marks it flaky.
- Fix the cause in the test or production code. More retries, sleeps, longer
  timeouts, skipped tests and quarantine do not count as fixes.

## Consequences

A flaky test blocks merging like any other failing test. The integration
profile has no retries, so its failures already fail the run.

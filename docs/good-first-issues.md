---
title: Good first issues
description: Small, bounded ways to contribute to Trace.
---

# Good first issues

These are intentionally small contribution areas, not promises that a specific
issue is currently open. Check the issue tracker before starting and ask for an
assignment when a task overlaps someone else's work.

## Documentation and examples

- Improve one integration guide with an exact command and a stated capability
  boundary.
- Add a sanitized troubleshooting example for a common installer or daemon
  failure.
- Check one public claim against source evidence and record the result in the
  relevant documentation.

## Static tooling and packaging

- Add a shell-level fixture test for a checksum mismatch or missing sidecar.
- Add a portable static check for release workflow permissions or artifact names.
- Improve the sanitized CI summary without including source contents or secret
  values.

## Safety and integrations

- Add a focused fixture for a documented command-guard edge case.
- Improve an integration's error message while preserving its existing data
  handling and hook boundary.
- Add a test for a documented observe-only versus enforced capability.

## Contribution rules

Keep the patch narrow, avoid unrelated dashboard/importer/pricing changes, run
the smallest relevant checks, and report exactly what was not tested. See
[`CONTRIBUTING.md`](../CONTRIBUTING.md) and [`SECURITY.md`](../SECURITY.md).

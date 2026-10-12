# Changelog

This file records user-visible changes that are not yet attached to a tagged
release. Tagged release history remains in [docs/release-notes.mdx](docs/release-notes.mdx).

## [Unreleased]

- Clarified the free, open-source, local-first product boundary on the landing
  site and removed the password-gated page.
- Made shell, PowerShell, and npm binary installers require a valid SHA-256
  sidecar by default. `TRACE_ALLOW_UNSIGNED=1` is an explicit legacy escape
  hatch.
- Made the GitHub CI integration count high-severity policy findings from
  structured review JSON rather than text matching.
- Added release build provenance attestations and contributor, security,
  privacy, conduct, and onboarding documentation.

---
title: Install and license claim audit
description: Evidence and limits for Trace's public packaging claims.
---

# Install and license claim audit

Audit date: 2026-10-12

This is a source audit of the current checkout. It is not proof that a tagged
release completed, that a hosted site deployed, or that a downloaded artifact
was executed on every supported platform.

## License evidence

- `LICENSE` contains the MIT license.
- `Cargo.toml` declares `license = "MIT"` for the Rust workspace, and the
  workspace crates inherit it.
- `packages/npm/package.json` declares `license: "MIT"` and points at this
  repository.
- The landing, docs, and web packages are workspace applications marked private
  for publication purposes; that flag is not a paid product claim or a
  different software license. Retired desktop/cloud/hosting sketches live
  under `experimental/` and are not release packages.

The public description “free and open source” is therefore grounded in the
checked-in license and repository source. It does not mean that an agent,
provider API, GitHub, or other third-party service used with Trace is free.

## Installer evidence

The following installers now fail closed when the checksum sidecar is missing,
unreachable, malformed, or does not match:

- `scripts/install.sh`
- `apps/landing/public/install.sh`
- `scripts/install.ps1`
- `apps/landing/public/install.ps1`
- `packages/npm/scripts/platform.js`

Each requires a 64-character SHA-256 digest before writing the binary. The
explicit `TRACE_ALLOW_UNSIGNED=1` environment variable bypasses verification for
legacy/manual use and is intentionally not the default. Release workflow staging
creates the sidecars used by the binary installers.

## Release provenance

`.github/workflows/release.yml` requests GitHub artifact-attestation permissions
and attests staged CLI artifacts on tagged runs. This checkout has not run the
release workflow, so the attestation, uploaded assets, signatures, and live
release page remain unverified here.

## Landing claim audit

`apps/landing` now describes Trace as free, open-source, and local-first. The
password-gated `/private` route and its client-side password have been removed.
The dashboard copy describes the local daemon only; it does not advertise a
hosted dashboard, paid plan, or cloud-sync path. Vercel/GitHub Pages remain
deployment hosts for static public files, not a claim that Trace stores project
data there.

## Follow-up verification

Before calling a release complete, run the exact tagged workflow, inspect the
release assets and attestations, and test the installer against a real release
with both a matching and a deliberately incorrect checksum. Those checks are
outside this static audit.

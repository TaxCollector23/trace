# Contributing to Trace

Trace is a free, MIT-licensed, local-first tool for recording and guarding AI
coding sessions. Contributions should preserve that boundary: local operation
is the default, data handling should be explicit, and public copy must match
what the checkout and release workflow actually prove.

## Before you start

1. Read the relevant source and documentation, especially `docs/STATE.md` when
   working on usage, integrations, installers, or release claims.
2. Search existing issues and pull requests before opening a new one.
3. Keep changes focused. Avoid mixing importer, pricing, dashboard, packaging,
   or documentation changes unless the task requires the combination.
4. Never commit credentials, local databases, transcripts, private paths, or
   generated build output.

## Development

```bash
git clone https://github.com/TaxCollector23/trace
cd trace
npm --prefix apps/landing ci
npm --prefix apps/docs ci
npm --prefix apps/web ci
```

The CLI and daemon are Rust crates. Use the pinned workspace toolchain from
`Cargo.toml`; use the smallest relevant package or test command when working
on Rust code.

## Checks

Run the checks that match the files you changed. Typical checks include:

```bash
cargo fmt --all --check
npm --prefix apps/landing run build
npm --prefix apps/docs run build
npm --prefix apps/web run build
bash -n scripts/install.sh apps/landing/public/install.sh
git diff --check
```

Do not describe a static check as proof of a hosted deployment, a live release,
or an integration working on every supported machine. Document any check you
could not run and why.

## Pull requests

- Explain the user-visible result and the files in scope.
- Include the exact checks you ran and identify untested surfaces.
- Keep product claims tied to checked-in behavior and evidence.
- Update the relevant docs, changelog, or privacy/security notes when behavior
  or public claims change.
- Keep unrelated working-tree changes out of the commit.

By contributing, you agree that your work is licensed under the repository's
MIT license and that you will follow the Code of Conduct.

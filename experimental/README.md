# Experimental surfaces

The default Trace product is the local Rust CLI, daemon, SQLite store, and
browser dashboard. The directories here are retained as reference code only;
they are not workspace members, release artifacts, or supported deployment
targets.

- `desktop-shell/` is the former Tauri wrapper around the local dashboard.
- `cloud-api/` is the former hosted sync API. The daemon no longer contains a
  cloud-sync client.
- `firebase/` and `render.yaml` are old hosting scaffolding and are not part of
  the local-first release.
- `github-app/` is a webhook handler sketch. Use the maintained GitHub Action
  and `trc review-diff`/`trc ratify` paths instead.

Experimental code is intentionally excluded from the Cargo workspace, CI,
release workflow, landing site, and product documentation.

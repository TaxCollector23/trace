---
title: Privacy
description: What Trace stores locally and when it makes network requests.
---

# Privacy

Trace is free, open-source, and local-first. The core CLI and daemon do not
require an account or a hosted service. By default, project data is stored on
your machine in `~/.trace` and the project's `.trace/` directory.

## Local data

Depending on the commands and integrations you use, local storage can include
run metadata, commands, redacted output, file-change summaries, diffs, policy
findings, usage metadata, and checkpoints. Trace is not a substitute for a
secret manager: avoid placing credentials in commands or prompts, and review
the documented redaction and limitation boundaries.

The local dashboard talks to the daemon on `127.0.0.1`. The static landing and
documentation sites do not need the daemon and do not read the local database.

## Network operations

Trace makes network requests only for explicit or separately configured actions,
including GitHub reads and ratification, release downloads and updates, and
agent/provider integrations you choose to run. Those operations can expose the
data required by the remote service; review their permissions before use.

The core guard and policy checks do not call an AI provider. There is no cloud
sync or account requirement for local runs.

## Retention and deletion

Trace has no central Trace account or default hosted event store. You control
local retention by removing the relevant Trace home or project `.trace/` data
after stopping the daemon. Git checkpoints, agent transcripts, GitHub data, and
provider-side records are controlled by those other tools and are not removed
by deleting Trace's local database.

## Questions and reports

For a privacy or security concern, use the private reporting path described in
[`SECURITY.md`](../SECURITY.md). Do not include real credentials, private paths,
or project contents in a public issue.

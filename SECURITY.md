# Security Policy

Trace is local-first. The daemon binds to loopback by default, the guard and
secret scanner make decisions locally, and no account is required for the core
CLI. GitHub and release-download commands are explicit network operations.

## Reporting a vulnerability

Please use GitHub's private vulnerability reporting or a private security
advisory for this repository. Do not put credentials, exploit details, private
paths, transcripts, or proof-of-concept payloads in a public issue.

Include, when safe:

- the affected version, commit, or file;
- a short description of the impact;
- reliable reproduction steps with synthetic data; and
- any suggested mitigation.

If private reporting is unavailable, open a public issue with only a minimal,
non-sensitive description and ask the maintainers for a private channel.

## In scope

Reports about command execution or guard bypasses, secret exposure or
redaction failures, unintended network or cloud data transfer, installer or
release-integrity failures, authentication/authorization in optional surfaces,
and vulnerabilities introduced by Trace's own code are in scope.

## Out of scope

Provider vulnerabilities, compromised third-party dependencies, unsafe commands
chosen by a user, and credentials deliberately supplied to `trc github` or
another explicit network command should be reported to the relevant owner.
Please still report a Trace defect that causes unexpected disclosure or unsafe
execution.

There is no guarantee that a local guard can identify every obfuscated command;
the documented pattern-based limitations remain part of the threat model.

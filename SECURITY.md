# Security Policy

## Supported versions

This project is pre-1.0; security fixes are applied to the latest release on the `main` branch.

| Version | Supported |
| ------- | --------- |
| 0.1.x   | ✅        |

## Reporting a vulnerability

Please report security issues **privately** — do not open a public issue for an unfixed
vulnerability.

- Email: **security@noru.tech** with a subject line beginning `[SECURITY] fideslang-tools`.
- Or use GitHub **Private vulnerability reporting** (Security → *Report a vulnerability*).

Please include: a description of the issue, the affected version or commit, reproduction steps or a
proof of concept, and the impact you foresee.

We aim to acknowledge reports within **5 business days** and to provide a remediation timeline after
triage. We will credit reporters who wish to be named once a fix is released.

## Scope and threat model

`fl` is a local command-line tool:

- It performs **no network calls** at runtime. The taxonomy snapshot is compiled into the binary.
- It reads the manifest files you point it at and writes only where you tell it to (`-o`,
  `--out-dir`).
- Release binaries are built by GitHub Actions from tagged commits, published with SHA-256 checksums
  and GitHub artifact attestations. Verify with
  `gh attestation verify <archive> --repo noru-tech/fideslang-tools`.

Out of scope: the semantic accuracy of the upstream Fideslang taxonomy, and vulnerabilities in
upstream Fideslang or Fides.

### A note on manifests

A Fides data map describes where personal data lives. Treat manifests and anything `fl` derives from
them (graphs, stats, merged files) as **sensitive**.

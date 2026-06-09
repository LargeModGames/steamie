# Security Policy

vapour speaks to Steam's connection-manager protocol on your behalf and handles authentication
sessions, so we take security seriously.

## Supported Versions

We maintain security fixes only for the current stable release stream.

| Version | Supported          |
| ------- | ------------------ |
| 0.4.x   | :white_check_mark: |
| < 0.4   | :x:                |

## Reporting a Vulnerability

- Please report vulnerabilities **privately** via GitHub Security Advisories
  (Security → Report a vulnerability) instead of opening a public issue.
- Include details that help us reproduce and assess impact: vapour version, OS, terminal,
  installation method, minimal repro steps, and any relevant logs **with secrets and session
  tokens stripped**.
- You can expect an acknowledgment within 3 business days and status updates at least weekly until
  resolution.
- If the issue is confirmed, we will coordinate a fix and release notes before public disclosure.

## A note on credentials

vapour never stores your Steam password. QR sign-in is the default. If you report an issue, never
include account credentials, QR payloads, refresh tokens, or session secrets in the report.

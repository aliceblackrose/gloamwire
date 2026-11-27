# Security policy

Report suspected security vulnerabilities privately through GitHub's private vulnerability reporting interface if enabled, or contact the repository maintainer privately. Do not post usable credentials or exploit details in public issues.

## Repository protection (GW-002)

A pull request cannot configure GitHub branch protection. Before merging this hardening change, a repository administrator must enable a ruleset targeting `main` under Settings → Rules → Rulesets. Require pull requests, successful `check` and `dependency-audit` status checks, block force pushes and deletion, and restrict bypass. Require review where a second maintainer is available. Confirm the branch is protected through the GitHub API after saving. Enable private vulnerability reporting and secret scanning where supported.

## Deployment boundaries

The managed voice session owns outbound nonce state; external cryptography and cross-process key uniqueness remain the caller/service's responsibility. OAuth2 pending authorizations must be stored per initiating browser session and consumed on callback. Distributed stores must atomically enforce persistent fencing generations. A cache that has lost stream continuity must be rebuilt from authoritative snapshots before it is trusted.

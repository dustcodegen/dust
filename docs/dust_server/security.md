# Security and production ownership

Dust supplies safe defaults and explicit controls. It cannot decide who may
read a customer's record, where TLS terminates, or which proxy headers are
trusted. Those remain application and deployment policy.

## Minimum internet-facing policy

```dart
final app = productionRouter(
  bodyLimit: 64 * 1024,
  maxConcurrentRequests: 64,
  maxQueuedRequests: 64,
  requestTimeout: const Duration(seconds: 10),
  onRecord: logger.info,
  onError: logger.error,
)
  ..layer(RateLimit(
    maxRequests: 60,
    window: const Duration(minutes: 1),
    keyBy: (request) => request.headers['x-api-key'] ?? 'anonymous',
  ))
  ..merge(apiRoutes());
```

At the trusted reverse proxy, also bound connections, header size, and request
rate before the request reaches Dart. Terminate TLS there or pass a
`SecurityContext` to `serve`. Never trust `x-forwarded-for` unless that proxy
strips the client's value and writes its own.

For browser WebSockets, pass every trusted origin explicitly:

```dart
ws(chat, allowedOrigins: const {'https://app.example'});
```

An absent `Origin` remains valid for non-browser clients. A browser-only
endpoint should require the header in an upgrade guard as well. The HTTP
concurrency layer covers only the upgrade; cap active WebSocket connections per
identity in the application or at the trusted reverse proxy.

## OWASP Top 10:2025 ownership

| Risk | Dust supplies | The application or deployment still owns |
| :--- | :--- | :--- |
| A01 Broken Access Control | scoped route layers, typed auth extraction, WebSocket origin allowlists | object- and tenant-level authorization on every operation |
| A02 Security Misconfiguration | explicit CORS/security headers, body limits, opaque 500s | production CSP/HSTS values, TLS, debug/config review |
| A03 Software Supply Chain Failures | pinned release actions/toolchains, locked dependencies, local and release audits | secret protection and reviewing dependency changes |
| A04 Cryptographic Failures | constant-time webhook/session examples, TLS support | key storage, rotation, approved algorithms and data classification |
| A05 Injection | typed extraction, escaped templates, SQLx-style bound SQL | never concatenate SQL identifiers or shell commands from input |
| A06 Insecure Design | concurrency, queue, rate, body and timeout limits | abuse-case design, proxy limits and endpoint-specific budgets |
| A07 Authentication Failures | bearer/basic/API-key/cookie primitives and correct challenges | token validation, expiry, revocation, issuer/audience and MFA |
| A08 Software or Data Integrity Failures | deterministic generation and release checksums | trusted deployment, migration review and artifact provenance policy |
| A09 Security Logging and Alerting Failures | request ids, access records and traces without query strings | durable sinks, redaction, alerts, retention and incident response |
| A10 Mishandling Exceptional Conditions | typed rejections, guarded 500s, observer isolation and drain deadlines | failure-mode tests for external services and business transactions |

This table follows the [OWASP Top 10:2025](https://top10.owasp.org/2025/0x00_2025-Introduction/).
Use [OWASP ASVS](https://asvs.owasp.org/) for a release verification checklist;
the Top 10 is a risk index, not a complete test plan.

## Memory and dependency safety

The Rust workspace forbids `unsafe` in first-party crates. Dart code is
memory-managed. The local pre-commit hook and release-only CI run
`scripts/security/audit.sh`, which rejects Rust vulnerabilities, unsoundness,
unmaintained crates, unexpected yanks, and affected/retracted/discontinued Dart
packages.

One lockfile warning is explicitly bounded: `spin 0.9.8` is yanked and remains
transitive through SQLx 0.8.6's SQLite channel. The audit permits only that
exact name and version; any additional yanked crate fails. Upgrade it when SQLx
offers a compatible path.

No framework check proves a deployed service secure. Before production, test
the real proxy/TLS configuration, application authorization, secret handling,
database privileges, backups, restore, monitoring, and sustained load.

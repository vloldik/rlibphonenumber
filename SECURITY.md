# Security Policy

We take the security and stability of `rlibphonenumber` seriously. This document outlines our supported versions, vulnerability reporting procedures, and the built-in security mechanisms we use to protect user data.

## Supported Versions

Security updates and critical bug fixes are provided for the current major version. Following the complete core refactor in v2 (migration to `prost` and AOT validation), v1 is no longer supported.

| Version | Supported          |
| ------- | ------------------ |
| 2.x.x   | ✅ Yes              |
| 1.x.x   | ❌ No               |
| < 1.0   | ❌ No               |

## Reporting a Vulnerability

**Please do not open public Issues for security-related vulnerabilities.** Publicly disclosing a flaw before a patch is available puts all users at risk.

To report a vulnerability, please use the GitHub Security Advisory feature:

1. Navigate to the **[Security](../../security)** tab of this repository.
2. Select **Advisories** and click **Report a vulnerability**.
3. Provide a detailed description of the issue.

**What to include in your report:**
* The type of vulnerability (e.g., ReDoS, memory leak, runtime panic, or PII leakage).
* A Proof of Concept (PoC) or example input that triggers the issue.
* The version of `rlibphonenumber` used and any enabled feature flags (e.g., `regex`, `lite`, `digest`).

## Privacy & PII (GDPR) Compliance

`rlibphonenumber` is designed for high-performance, local data processing. 
* **Zero Telemetry:** The library does not send phone numbers, logs, or metrics to any external servers.
* **DLP Features:** We provide the `PhoneMaskUtil` specifically for **GDPR** and **Data Loss Prevention (DLP)** compliance. This allows you to mask (e.g., `***-***-1234`) or cryptographically hash (HMAC/SHA256) phone numbers directly into stack-allocated buffers, ensuring sensitive data is never accidentally logged or stored in plain text.

## Security Engineering Practices

We employ several layers of defense to ensure the library remains "enterprise-grade":

*   **Differential Fuzzing:** The parser is continuously fuzzed (over 500,000 iterations) against Google's original C++ ICU implementation. This ensures zero mismatches and protects against edge-case crashes or hangs when processing malicious input strings.
*   **Memory Safety:** Built in Rust, the library leverages the borrow checker to eliminate common vulnerabilities like buffer overflows, use-after-free, and data races.
*   **AOT Metadata Validation:** To prevent Regular Expression Denial of Service (ReDoS) or runtime panics, all telecom metadata is strictly validated at compile-time. We check byte lengths and compile all regexes before the binary is even shipped.

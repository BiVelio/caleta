# Security policy

## Supported versions

Only the latest released minor version is expected to receive security fixes during the 0.x series. Version 0.1.0 has not been published by this source archive alone; check the repository's actual release page after publication.

## Report privately

Use GitHub **Private vulnerability reporting** on the Caleta repository. Repository owners should enable it before the public launch. If that mechanism is unavailable, contact a BiVelio maintainer through an already verified organizational channel; do not post vulnerability details in a public issue or invent a security email address.

Include affected versions, platform, backend version, impact, reproduction steps, and whether credentials or prompts are exposed. Remove secrets and sensitive prompt content. A minimal proof of concept is preferable to a large artifact.

## Response process

Maintainers should acknowledge a report within five business days, assess scope and severity, coordinate a fix and regression test, and publish an advisory with the release. These are operating targets, not a guarantee of resolution time. Reporters will be credited unless they request otherwise.

## Security boundaries

Please review `docs/threat-model.md`. Reports are especially relevant for authentication bypass, unexpected non-loopback exposure, upstream CORS bypass, credential leakage, unsafe argument/environment injection, artifact verification bypass, process-lifetime failures, or prompt persistence. The absence of OS-level egress sandboxing, signed manifests, and TLS is documented and should not be reported as though those features were claimed.

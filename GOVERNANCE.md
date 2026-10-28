# Governance

Caleta is initially maintained by [BiVelio](https://bivelio.com) and developed publicly as part of [BiVelio Research](https://research.bivelio.com). No individual maintainers or external institutional affiliations are assumed in this document.

## Decision making

Routine changes are reviewed through pull requests and require passing CI. Changes to network exposure, authentication, model verification, process execution, manifest semantics, or benchmark definitions need an additional design explanation and threat-model update. A maintainer may revert a change that introduces an undocumented security or privacy claim.

## Community input

Issues and pull requests are the default decision record. Significant architectural proposals should describe the problem, alternatives, compatibility impact, threat-model impact, and validation plan. Community adoption of a schema does not prevent versioning, but breaking changes require a new schema version and migration notes.

## Releases and security

Maintainers review release checklists, tags, binaries, and advisories. Vulnerability reports follow `SECURITY.md` and are not handled in public issues. Security fixes take precedence over roadmap features.

## Trademarks and models

The Apache-2.0 license covers project code, not BiVelio/Caleta trademarks or third-party model artifacts. Contributors must not upload models unless redistribution rights permit it. References to a model's license are metadata supplied by the manifest author and must be checked by users.

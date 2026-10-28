# Threat model

Caleta uses “private” and “verified” as bounded technical claims. This document defines the boundary so that a release cannot honestly turn those words into an absolute guarantee.

## Assets and adversaries

The assets are prompt and response contents, model and backend artifacts, public API credentials, host resources, and the accuracy of benchmark or execution evidence. Relevant adversaries include a remote client reaching an exposed listener, a malicious web page attempting browser cross-origin requests, an artifact substituted in transit or storage, an untrusted local process racing file replacement or connecting to loopback, and accidental misconfiguration by the operator.

A fully malicious host OS, privileged local attacker, compromised Caleta binary, or malicious backend with native-code execution is outside the v0.1 containment boundary. Model-file parsing occurs inside llama.cpp, so Caleta's digest check cannot make a malicious GGUF safe to parse.

## Controls and residual risk

| Threat | v0.1 control | Residual risk / non-claim |
|---|---|---|
| Artifact changed after download | Exact size and streamed SHA-256 before serve/bench | Expected digest must come from a trusted source; TOCTOU replacement remains possible |
| Wrong backend binary | Optional explicit-path SHA-256 | Unpinned `PATH` resolution is allowed and reported as unpinned |
| Backend reaches network | Loopback listener, `--offline`, no inherited `LLAMA_ARG_*` | Backend flag is not OS-enforced egress isolation |
| Unauthorized remote API use | Required bearer key for non-loopback; constant-time comparison | HTTP has no TLS; key quality, transport, and client storage remain operator duties |
| Browser cross-origin abuse | No CORS by default; explicit origins only; upstream CORS stripped | Non-browser clients ignore CORS; authentication remains necessary |
| Backend credential visible in process list | Random internal key passed through `LLAMA_API_KEY` | Same-privilege local processes may inspect environments on some systems |
| Dangerous backend features | Agent, tools, Web UI, listener, model, and credential flags rejected | New backend options require ongoing review; denylist is not universal |
| Prompt retention | No Caleta request/response logging; benchmark stores prompt hash by default | Backend, clients, crash reports, swap, and OS facilities are separate systems |
| Oversized request | 16 MiB request-body limit | Large requests still consume that bounded memory; response size is model-dependent |
| Misleading benchmark | Raw per-run metrics, failures, environment, cold/warm distinction, limitations | No universal ranking; ITL is SSE-event based; energy and quality are absent |
| False execution evidence | Prompt-free JSON records exact declared configuration | Record is unsigned and created by the same process; it is not an attestation |

## Loopback is not a complete security boundary

Any local process may attempt to connect to a loopback port. Caleta's internal random key reduces accidental or opportunistic access, but it is not protection against a privileged inspector. Reserving an ephemeral port before launching the backend also has a small race: another local process could bind it between reservation and child startup. Readiness would then fail or communicate with the wrong service; deployments with hostile local users need OS-specific socket passing, stronger isolation, and additional tests.

The public `/health` endpoint is unauthenticated and reveals only service name, status, and Caleta version. Model listing and chat require the public key when configured. Caleta does not forward arbitrary backend routes such as metrics, props, slots, or tools.

## Recommended deployment

Keep model and backend directories writable only by trusted operators. Use a current llama.cpp build and monitor its security advisories. Leave the public listener on loopback when possible. When remote access is necessary, use a TLS reverse proxy or SSH tunnel, restrict network reachability with a firewall, generate a high-entropy key, and keep it out of manifests, shell history, and issue reports. Treat execution records as operational evidence rather than cryptographic proof.

For stronger privacy guarantees, add a tested OS-specific sandbox that denies outbound networking and limits filesystem access, use signed manifests from a trusted publisher, and independently verify released Caleta binaries. Those capabilities are roadmap items, not hidden properties of v0.1.

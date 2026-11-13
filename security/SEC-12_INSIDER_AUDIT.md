# SEC-12: Insider Threat Audit — Design Specification

**Status:** Design complete. Implementation requires separate signing key.

## Problem

An authorised operator (with the MAVLink signing key) can send valid
commands that cause harm. The current `rnfc-hase` filters unsafe commands,
but:

1. HASE is advisory — an operator with root access can bypass it
2. HASE logs to the same CertChain that the operator controls
3. There is no independent audit trail

## Design: Dual-Key Audit Log

```
Operator key (K_op)     → signs MAVLink commands (existing)
Audit key (K_audit)     → signs the audit log (NEW, held by a third party)
```

### Architecture

```
+------------------+     +------------------+     +------------------+
| Drone            |     | CertChain        |     | Audit Service    |
|                  |     | (operator-owned) |     | (third-party)    |
| Every command    |---->| Hash chain       |     |                  |
| is logged with:  |     |                  |     | Receives signed  |
|  - timestamp     |     |                  |     | digests every N  |
|  - operator ID   |     |                  |     | minutes          |
|  - command       |     |                  |     |                  |
|  - K_op signature|     |                  |     | Verifies with    |
|  - K_audit sig   |     |                  |     | K_audit public   |
+------------------+     +------------------+     +------------------+
```

### Key Separation

| Key | Holder | Purpose |
|---|---|---|
| K_op | Operator | Signs MAVLink commands |
| K_audit | Third party (regulator, insurer) | Signs audit digests |
| K_chain | Drone | Signs CertChain blocks (existing) |

The operator cannot forge the audit log because they do not have K_audit.

### Implementation Steps

1. **Companion**: after every N commands (e.g., 100), compute
   `digest = SHA-256(command_log_since_last_digest)`
2. **Companion**: sign `digest` with K_audit (private key stored in HSM)
3. **Companion**: send `(digest, signature, timestamp)` to audit service
4. **Audit service**: store, verify with K_audit public key
5. **Auditor**: at investigation time, compare audit log with CertChain

### What This Closes

| Threat | Before | After |
|---|---|---|
| T-11 (insider) | ⚠️ Partial (HASE advisory) | ✅ Independent audit trail |

### What Remains

- Collusion between operator and third party (out of scope)
- Compromise of the audit service (trust in third party)

## Status

| Component | Status |
|---|---|
| Design (this document) | ✅ Complete |
| `rnfc-audit` crate | ❌ Not implemented |
| K_audit key management | ❌ Requires HSM |
| Audit service | ❌ Requires third party |

**This is a process/organisational change, not a code change.**
The code (a new crate `rnfc-audit`) is straightforward (~200 lines),
but it is useless without the third-party key holder.

*Last updated: v6.4.0*

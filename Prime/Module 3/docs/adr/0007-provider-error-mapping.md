# ADR 0007 — Provider HTTP Error Mapping

## Status

Accepted

## Context

M05–M08 need consistent retry behavior. M01 defines `TalosError` classes; M03 maps provider transport failures onto them.

## Decision

| Condition | Map to | Retry? |
|-----------|--------|--------|
| Timeout / connection reset | `Transient` | Yes (bounded) |
| HTTP 429, 502, 503, 504 | `Transient` | Yes |
| HTTP 400 (bad image/schema) | `Validation` | No |
| HTTP 401, 403 | `Config` or `Permanent` | No |
| HTTP 404 (model) | `Permanent` | No |
| Provider enabled but no API key | `NotImplemented` or `Config` | No |
| Unimplemented adapter | `NotImplemented` | No |

Failover to the next configured provider occurs only after retries are exhausted on `Transient` errors.

## Consequences

- Workers can rely on `TalosError::is_retryable()` without parsing HTTP codes.
- Bad credentials do not burn fallback quotas in a retry loop.

# PDR M12 — Auth, Admin & Config

| Field | Value |
|-------|-------|
| **Module ID** | M12 |
| **Module name** | Auth, Admin & Config |
| **Product** | Talos-RS — Automated Vehicle Identification & HSRP Audit Engine |
| **Document status** | Normative (v1.0) — APPROVED / DESIGN LOCKED |
| **Owns** | AuthN/AuthZ, sessions, MFA policy, admin settings, ConfigRevision |
| **Depends on** | M01 health/config spine; M10 for durable auth/config audit |
| **Layout** | [`Prime/Module 12/`](../../) |
| **Related** | [Architecture](../architecture/overview.md) · [M09](../../../Module%209/docs/pdr/M09-review-management.md) · [M11](../../../Module%2011/docs/pdr/M11-export-and-reporting.md) · [M10](../../../Module%2010/docs/pdr/M10-storage-and-data.md) |

---

## 1. Purpose

M12 answers **who may perform an operation**. Owning modules (M09, M11, …) define **what the operation means**. M10 keeps the **durable record**.

```text
M12 AuthN/AuthZ (permissions)
        │
   ┌────┴────┐
   ▼         ▼
  M09       M11
 Review    Export
   │         │
   └────┬────┘
        ▼
       M10
```

---

## 2. Product context (locked)

| Constraint | Rule |
|------------|------|
| **AuthZ model** | Permission checks — not role-name string equality |
| **system_admin** | No automatic review authority |
| **Secrets** | Refs in TOML; values in env/secret manager; redacted in logs/UI |
| **Config** | Transactional ConfigRevision; invalid → keep previous |
| **Hot reload** | Explicit operational knobs only; secrets require restart |
| **Sessions** | TTL + server-side revocation |
| **MFA recovery** | Privileged audited operation |

---

## 3. Roles and permissions

Starter roles: `review_officer`, `review_lead`, `export_viewer`, `export_admin`, `system_admin`.

Permissions (normative set):

```text
review.read | review.act | review.override
export.read | export.audit
user.manage | config.read | config.write
```

Default maps: [`configs/auth.toml`](../../configs/auth.toml). `system_admin` defaults exclude `review.act` / `review.override`.

---

## 4. ConfigRevision pipeline

```text
New Config → Parse → Schema Validation → Cross-field Validation
  → Create ConfigRevision → Atomic Activation → Audit Event
```

Invalid: reject; keep previous active. Prefer M08 decisions store `config_revision_id` / fingerprint for historical interpretability.

---

## 5. Secrets

```text
Committed TOML     → secret references/names only
Env / Secret Mgr   → actual secret
Logs / API / UI    → redacted
```

---

## 6. Scope

### 6.1 In scope

- User/account model (logical)
- Session + MFA policy
- Permission evaluation contract
- Admin-gateway route ownership (later)
- ConfigRevision identity and activation rules

### 6.2 Out of scope

| Owner | Capability |
|-------|------------|
| M09/M11 | Meaning of review/export actions |
| M10 | Storage drivers |
| Build | OIDC/IdP vendor wiring, UI |

### 6.3 Forbidden

- Secrets in committed defaults
- Role-name hard-coding in module APIs
- Auto-granting review to `system_admin`
- Half-applied config activation
- Treating MFA recovery as ordinary verify

---

## 7. Admin surface

Extends `admin-gateway` (today health-only). Document routes later for user manage, config activate, session revoke.

---

## 8. Acceptance criteria

- [ ] Permission model documented; starter role maps in config.
- [ ] system_admin ≠ automatic review.
- [ ] ConfigRevision transactional semantics locked.
- [ ] Secrets / redaction / non-reloadable secrets locked.
- [ ] Session revoke + MFA recovery policy locked.
- [ ] Docs only under `Prime/Module 12/`.

---

## 9. Build mapping (later)

`talos-auth` + `admin-gateway` routes. No Rust / IdP in this docs step.

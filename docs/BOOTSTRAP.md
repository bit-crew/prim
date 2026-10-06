# Prim: Bootstrap Runbook

From absolute zero to a running **central core** (data + services + identity). Ordered by strict dependency: each resource requires the previous one. The corporate email is the bootstrap root (it creates every account). Per-project VPS is **optional** and comes last — the core does not require it.

> **Placeholders:** `<project_name>` is the instance name  and `<GOP>` is the Git/code host. Substitute both once decided.
>
> **Identity model (revised 2026-09-26):** one **IdP** holds people + groups. Two access planes share that identity — **browser** (SSO, low privilege) and **machine** (short-lived SSH cert from a central CA, high privilege). Permissions propagate from IdP groups to every resource (R2 prefixes, DWH roles, service scopes). See [GOVERNANCE.md](./GOVERNANCE.md#2-access--identity-idp--two-planes) and [stack-selection.md](./stack-selection.md#access-model).
>
> **DECISIÓN PENDIENTE:** IdP = **Cloudflare Access** (lean) or self-hosted. Machine CA = **CF Access for Infrastructure** (lean) or Smallstep/Teleport. Front = **Mattermost** (lean). Permissions = **coarse-grained** to start.

## Dependency chain

```
Root email → Cloudflare (domain + email routing) → corporate email
   → R2 (data core) → IdP (identity + groups) → SSO services (front, code host, AI)
   → group→permission mapping (R2 prefixes / DWH roles / service scopes)
   → [optional per project] VPS + machine-plane SSH-CA
```

```
Purelymail ($10)
   └─> Cloudflare + domain ($10)
          ├─> Email Routing → infra@<project_name>.co   (corporate email)
          ├─> CF_TOKEN                                     (1st API key)
          └─> R2 (<project_name>-ids: auth/RAG/olap/blob)  (DATA CORE)
   infra@<project_name>.co
      ├─> IdP tenant (Cloudflare Access lean) → groups: admins, developers, data-readers, project-*
      ├─> <GOP> (code host) → GOP_TOKEN                    (2nd API key), SSO via IdP
      ├─> Front (Mattermost) → SSO via IdP                 (browser plane entry)
      └─> Machine CA (CF Access for Infrastructure) → short-lived SSH certs (machine plane)
   [optional per project]
      └─> Hetzner → HCLOUD_TOKEN → VPS (compute), reached via machine-plane cert
```

---

## Phase 0 — Root identity (100% manual, ~1 hour, ~$20/yr)

Not automatable: the chicken-and-egg problem. You need ONE email to create everything else.

### 0.1 Bootstrap email
- Create a **Purelymail** account ($10/yr) → `infra@purelymail.com` (temporary). Sole purpose: receive Cloudflare verification. Recovery-only afterward.

### 0.2 Domain
- Buy `<project_name>.co` (~$10/yr). Buying inside Cloudflare skips DNS delegation.

### 0.3 Cloudflare (the central piece)
- Create a Cloudflare account using `infra@purelymail.com`.
- Add `<project_name>.co` → configure DNS.
- Enable **Email Routing** → `infra@<project_name>.co` is born (the only corporate email). Purelymail becomes recovery-only.

### 0.4 First API token (Cloudflare)
- Create a token with: `Zone:Edit`, `Email Routing:Edit`, `Workers:Edit`, `R2:Edit`, and **`Access:Edit`** (needed now — Cloudflare Access is the IdP lean).
- Save as `CF_TOKEN`; record `ACCOUNT_ID` and `ZONE_ID`. **First secret.**

**Phase 0 result:** a corporate email, `CF_TOKEN`, and a scriptable Cloudflare account. Everything after this is CLI-driven.

---

## Phase 1 — Data core (R2 + secrets)

### 1.1 Create R2 (the IDS / data core)
- With `CF_TOKEN`: create bucket `<project_name>-ids`.
- Create the IDS structure (see [FABRIC.md](./FABRIC.md)): `auth/`, `RAG/`, `olap/`, `blob/`, plus a per-user prefix convention `users/<user>/`.
- Pull S3-compatible R2 credentials → `R2_ACCESS_KEY`, `R2_SECRET`.

### 1.2 Store secrets in the core
- Tokens + R2 credentials go encrypted to `<project_name>-ids/auth/secrets.enc.yaml` (SOPS/age, per FABRIC.md).
- `identity.kdbx` (KeePass) stays as the human-managed backup.

### 1.3 Email→R2 Worker
- Deploy the Cloudflare Worker that turns inbound emails into JSON in R2 (`<project_name>-ids/blob/emails/`). No IMAP mailbox; the agent reads R2.

---

## Phase 2 — Identity (IdP + groups)

This is the new center. **DECISIÓN PENDIENTE:** Cloudflare Access lean.

### 2.1 Create the IdP tenant
- Cloudflare Access (or the chosen IdP) as the single identity provider for the domain.

### 2.2 Define groups (the source of truth for permissions)
- Create coarse-grained groups: `admins`, `developers`, `data-readers`, `data-writers`, `project-<x>`.
- Onboarding later = add a person to groups; offboarding = remove from groups.

### 2.3 Map groups → permissions (propagation)
- **R2:** policies/tokens scoped by prefix per group (e.g. `data-writers` → write `users/<user>/`, `data-readers` → read `olap/`).
- **DWH/DB:** SQL roles (`GRANT SELECT` for readers, `SELECT/INSERT` on own schema for writers).
- **Services:** SSO/SCIM role from the same groups.

---

## Phase 3 — Services (SSO, browser plane)

All services authenticate against the IdP. No separate logins.

### 3.1 Code host (`<GOP>`)
- Create `<GOP>` account with `infra@<project_name>.co`; enable SSO via the IdP.
- Create org/group; save `GOP_TOKEN`. **Second secret.** Code push uses the machine plane (Phase 4).

### 3.2 Front (browser plane)
- Bring up the front (**Mattermost** lean) with SSO via the IdP.
- This is the browser-plane entry: chat, calendar, video, tickets, project tracking.

### 3.3 Data viewing/editing (browser plane)
- Static app on Cloudflare Pages + DuckDB-WASM (view/query raw data) + a spreadsheet component (edit final reports) + a Worker as the R2 data-gate (reads/writes scoped by group). Stack open — see [stack-selection.md](./stack-selection.md#access-model).

### 3.4 AI (OAS)
- Deploy the operational assistant over the shared RAG (indexes `prim/` + instance docs + channel history + DWH). See [stack-selection.md](./stack-selection.md#oas-candidates).

---

## Phase 4 — Machine plane (SSH-CA)

For technical staff who need high privilege (develop, push code, write own data to R2).

### 4.1 Set up the central CA
- **DECISIÓN PENDIENTE:** Cloudflare Access for Infrastructure (lean, managed SSH-CA) or Smallstep/Teleport (self-hosted).
- The CA signs **short-lived SSH certs** (8–24h) after SSO, scoped to the person's groups.

### 4.2 Developer usage
- Developer does SSO → obtains a cert → uses it with their editor (Helix / VSCode / Cursor) over SSH.
- No `authorized_keys` copied anywhere. Revocation = remove from group (cert stops issuing, expires in hours).
- Optional: require a YubiKey to sign the cert as a hardware 2nd factor.

---

## Phase 5 — Per-project compute (OPTIONAL)

Only if a project needs dedicated compute. The core works without this.

### 5.1 Provision a VPS
- `hcloud server create ...` with `HCLOUD_TOKEN`.
- Reached via the machine plane (short-lived cert), not per-VPS keys.
- Install the dev environment via `dev-setup` (`home-manager switch --flake .#dev`).

---

## Phase 6 — Automate the bootstrap (`uvx prim init`)

Once done manually once, encapsulate Phases 1–4 in the CLI:

```bash
export CF_TOKEN=... GOP_TOKEN=...
uvx prim init --config config.yaml
```

Runs in order: create R2 → store secrets → deploy Worker → create IdP tenant + groups → map group permissions → bring up SSO services → set up machine CA. Per-project VPS (Phase 5) is a separate opt-in command.

Person onboarding/offboarding is specified in [GOVERNANCE.md](./GOVERNANCE.md#2-access--identity-idp--two-planes) (`prim onboard <user>` / `prim offboard <user>` = add/remove IdP groups).

---

## Known risks

- **IdP is now central (single point).** If the IdP is unavailable, both planes are affected. Mitigation: pick a managed, highly-available IdP (Cloudflare Access lean); keep `admins` break-glass credentials offline (KeePass).
- **Purelymail is a sovereignty weak point.** External service the bootstrap depends on. Fine as bootstrap + recovery only. Mitigation: enable Cloudflare Email Routing ASAP; move recovery to a factor you control.
- **CA / cert-lifetime tuning.** Too-long certs weaken revocation; too-short annoy developers. Start 8–24h and adjust. DECISIÓN PENDIENTE.
- **Coarse-grained permissions may be too broad early on.** Acceptable to start (YAGNI); revisit fine-grained (table/column/row) only when a real need appears.

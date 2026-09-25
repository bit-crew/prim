# Kestrel: Bootstrap Runbook

From absolute zero to a running instance. Ordered by strict dependency: each resource requires the previous one. The corporate email is the bootstrap root (it is what creates every account). You cannot create Cloudflare without an email, nor the code host without an email, nor pull API keys without an account.

> **Placeholders:** `<project_name>` is the instance name (also the domain, e.g. `<project_name>.co`) and `<GOP>` is the Git/code host. Substitute both once decided.
>
> **Identity model (decided):** person authentication is NOT done via the code host, email, or an IdP. It is done on each VPS via **YubiKey + SSH** (FIDO2). Cloudflare is transport only. The `<GOP>` code host governs **code access for technical staff only**, never employee login. See [stack-selection.md](./docs/ops/stack-selection.md#access-model).
>
> **Code host (`<GOP>`):** for the **public framework** (kestrel/dev-setup/agent) the lean is GitHub canonical + Codeberg mirror. For the **private instance** repos, the host is not yet fixed — self-hosting is rejected. See [stack-selection.md](./docs/ops/stack-selection.md#gop-git--code-host-pending). The sequence below is identical regardless of choice.

## Dependency chain

```
Root email → Cloudflare (domain + email routing) → real corporate email
   → <GOP> (code host, technical staff) → Hetzner (compute) → API keys → automation
   Person auth → YubiKey + SSH on each VPS (not email, not GOP, not an IdP)
```

```
Purelymail ($10)
   └─> Cloudflare + domain ($10)
          ├─> Email Routing → infra@<project_name>.co   (real root email)
          ├─> CF_TOKEN                                     (1st API key)
          └─> R2 (<project_name>-ids: auth/RAG/olap/blob)
                 └─> secrets.enc.yaml (SOPS)                (all other tokens live here)
   infra@<project_name>.co
      ├─> <GOP> → GOP_TOKEN                                 (2nd API key) → clone <project_name>/kestrel/dev-setup/agent (code only)
      └─> Hetzner → HCLOUD_TOKEN                            (3rd API key) → VPS
                       ├─> Cloudflare Tunnel               (transport only, no person login)
                       ├─> sshd + YubiKey (FIDO2)          (person auth happens HERE)
                       └─> Mattermost + Tunnel              (channel = employee interface)
                              └─> OAS Agent + RAG            (brain, no frontend)
```

---

## Phase 0 — Root identity (100% manual, ~1 hour, ~$20/yr)

Not automatable: the chicken-and-egg problem. You need ONE email to create everything else.

### 0.1 Bootstrap email (the only one that exists at first)
- Create a **Purelymail** account ($10/yr) → `infra@purelymail.com` (temporary).
- Sole purpose: receive Cloudflare verification emails. Nothing else.

### 0.2 Domain
- Buy `<project_name>.co` (~$10/yr). Buying it inside Cloudflare skips DNS delegation.

### 0.3 Cloudflare (the central piece)
- Create a Cloudflare account using `infra@purelymail.com`.
- Add `<project_name>.co` → configure DNS.
- Enable **Email Routing** → this is where the real corporate email is born: `infra@<project_name>.co`.
- From this moment, `infra@<project_name>.co` is **the only email that exists** for the company. Purelymail becomes recovery-only.

### 0.4 First API token (Cloudflare)
- Cloudflare → My Profile → API Tokens → Create Token.
- Permissions: `Zone:Edit`, `Email Routing:Edit`, `Workers:Edit`, `R2:Edit`, `Cloudflare Tunnel:Edit`.
  - Note: no `Access:Edit` needed for person login — people authenticate on the VPS via YubiKey/SSH, not Cloudflare Access.
- Save as `CF_TOKEN`; record `ACCOUNT_ID` and `ZONE_ID`.
- **First secret.** Cloudflare is now scriptable.

### 0.5 Code host account (`<GOP>` — code only, not identity)
> Host not fixed for the instance; self-hosting rejected. Public framework leans GitHub + Codeberg mirror. See [stack-selection.md](./docs/ops/stack-selection.md#access-model).
- Create `<GOP>` account with `infra@<project_name>.co` (the email arrives via Email Routing; read it in Purelymail, or via the Worker→R2 flow once step 1.3 is done).
- Create org/group (e.g. `<project_name>-sas`) — for **code access of technical staff only**, not employee login.
- Create a Personal Access Token with API scope.
- Save as `GOP_TOKEN`. **Second secret.**

### 0.6 Hetzner (compute)
- Create Hetzner Cloud account with `infra@<project_name>.co`.
- Create project → Security → API Tokens → Read & Write.
- Save as `HCLOUD_TOKEN`. **Third secret.**

### 0.7 YubiKey(s) — person identity
- Provision one **YubiKey (FIDO2)** per person (starting with yourself).
- Generate a hardware-backed SSH key: `ssh-keygen -t ed25519-sk` (requires the YubiKey present).
- This is the **identity for VPS access** — portable across machines, no email, no IdP. See [GOVERNANCE.md](./GOVERNANCE.md#vps-authentication-yubikey-over-ssh).

**Phase 0 result:** three API keys (`CF_TOKEN`, `GOP_TOKEN`, `HCLOUD_TOKEN`), a corporate email, and at least one YubiKey with its FIDO2 SSH key. Everything after this is done by the CLI.

---

## Phase 1 — Storage and secrets (first CLI use)

### 1.1 Create R2 (the IDS)
- With `CF_TOKEN`: create bucket `<project_name>-ids`.
- Create the IDS structure (see [FABRIC.md](./FABRIC.md)): `auth/`, `RAG/`, `olap/`, `blob/`.
- Pull S3-compatible R2 credentials (Access Key + Secret) → `R2_ACCESS_KEY`, `R2_SECRET`.

### 1.2 Store secrets in the IDS
- The 3 tokens + R2 credentials go encrypted to `<project_name>-ids/auth/secrets.enc.yaml` (SOPS/age, per FABRIC.md).
- `identity.kdbx` (KeePass) stays as the human-managed backup.
- Secrets now live in the SoT, not on your machine.

### 1.3 Email→R2 Worker
- Deploy the Cloudflare Worker that turns inbound emails into JSON in R2 (`<project_name>-ids/blob/emails/`).
- Now `*@<project_name>.co` is captured with no IMAP mailbox. The agent reads R2.

---

## Phase 2 — Repos (code and rules)

### 2.1 Clone/create base repos
> All URLs depend on the chosen code host (code only — not identity).

| Repo | Role |
|------|------|
| `<project_name>/` | Statutes, investment policy, OLTP (instance-specific). |
| `kestrel/` | Python package + operating model + `config.yaml`. |
| `dev-setup/` | Technical environment (Nix). |
| `agent/` | LLM steering. |

### 2.2 Write the instance `config.yaml`
```yaml
instancia:
  nombre: <project_name>
  dominio: <project_name>.co
  repo_codigo: <GOP>/<org>/<project_name>.git      # code host (technical staff)
  storage: s3://<project_name>-ids                  # R2
  tds: <tds-dsn>                                    # TDS/OLTP, engine in stack-selection.md
canal:
  tipo: mattermost                                  # self-hosted (data sovereignty)
agente:
  tipo: oas                                         # see stack-selection.md
  steering: <GOP>/<org>/agent.git                   # code host
acceso:
  identidad: yubikey-ssh                            # person auth on VPS, not IdP/email
```

---

## Phase 3 — The channel (employee interface)

### 3.1 Bring up Mattermost
- Provision a Hetzner VPS (`hcloud server create`) with `HCLOUD_TOKEN`.
- Mattermost self-hosted (Docker or binary) — satisfies "self-hosted, own data".
- Expose via Cloudflare Tunnel (no public IP, `cloudflared`) — **transport only**.
- Harden `sshd`: FIDO2 SSH key (YubiKey) required, `MaxSessions 1`. Person auth is on the VPS, not Cloudflare. See [GOVERNANCE.md](./GOVERNANCE.md#vps-authentication-yubikey-over-ssh).

### 3.2 Create the bot
- Bot account in Mattermost → pull `MATTERMOST_TOKEN`.
- Webhook → points to the agent (or to a Worker proxy if the VPS has no fixed IP).

---

## Phase 4 — The OAS agent (the brain, no frontend)

### 4.1 Deploy the agent
- `agent.py` runs on the VPS, triggered by the channel webhook.
- On startup: reads secrets from R2, loads steering from `agent/`.

### 4.2 Initial RAG
- Index `kestrel/` + `<project_name>/` (docs) into FAISS → `<project_name>-ids/RAG/brain_index/`.
- Index channel history.
- Connect the TDS/OLTP DB as a data source.

**Result:** employees ask in Mattermost, the agent answers with real context. No portal.

---

## Phase 5 — Automate the bootstrap itself (`uvx kestrel init`)

Once done manually once and the flow is understood, encapsulate Phases 1–4 in the CLI. From any clean PC with only the 3 tokens:

```bash
export CF_TOKEN=... GOP_TOKEN=... HCLOUD_TOKEN=...
uvx kestrel init --config config.yaml
```

Runs in order: create R2 → store secrets → deploy Worker → provision VPS → bring up Mattermost + tunnel → deploy agent → index RAG.

Person onboarding/offboarding is already specified in [GOVERNANCE.md](./GOVERNANCE.md) (`kestrel onboard <user>` / `kestrel offboard <user>`).

---

## Known risks

- **Code host undecided (instance repos).** Instance repos may be split across platforms today. Pick ONE code host before running this runbook. Note this is **code hosting only** — it is no longer the identity/OIDC provider (person auth moved to YubiKey/SSH). Public framework leans GitHub + Codeberg mirror. Tracked in [stack-selection.md](./docs/ops/stack-selection.md#access-model).
- **YubiKey recovery gap.** Person identity = FIDO2 SSH key on a physical YubiKey. If a person loses their YubiKey, with email/IdP deliberately rejected there is no external recovery channel. Mitigation: issue a **backup YubiKey per person** enrolled with the same access, stored offline. Define this in onboarding. Open item.
- **Purelymail is a sovereignty weak point.** It is an external service the entire bootstrap depends on. Fine as bootstrap + recovery only, but if you lose it before `infra@<project_name>.co` works, you lose the root. Mitigation: enable Cloudflare Email Routing ASAP and move recovery to a second factor you control.

# Kestrel: Governance & Operations

Minimalist operation centered on the channel as the terminal and Git as the source of truth.

## 1. Channel-Centric Demand Flow

The **Unified Demand Flow** follows the "The Thread is the Issue" principle:

1.  **Capture (Channel):** Convert messages into Issues via native Git-platform shortcuts.
2.  **Execution (CLI):** Development occurs in the terminal. Branch name: `feature/issue-ID`.
3.  **Sync:** Automated notifications post commit updates back to the original channel thread.
4.  **Review & Merge:** Approval and consolidation happen directly via channel buttons.

### Standardized Classification Labels
Use these labels for Agent-led classification:
- **Task Type:** `Type::Tech`, `Type::Operational`, `Type::Legal`, `Type::Financial`.
- **Approval:** `Status::Pending-LLM-Approval`, `Status::Pending-Unanimity`.
- **Asset:** `Asset::House-X`, `Expense::Capex`, `Expense::Opex`.

## 2. Zero-Trust Access Layer

Security is an environmental condition, not a business logic function.

Two independent layers, never conflated:

- **Layer A — Transport (Cloudflare):** **Cloudflare Tunnel** exposes each VPS with no public IP and no inbound ports (outbound-only connection to the edge). Cloudflare is **only the cable** — it does NOT authenticate people. No Cloudflare Access, no IdP, no OTP for person login.
- **Layer B — Person authentication (on the VPS):** The VPS `sshd` authenticates the person directly with **SSH key + hardware key (YubiKey FIDO2)**. No email, no IdP, no third party. Portable: the person carries their YubiKey and connects from any machine.

- **Internal resources (TDS/OLTP DB, private services, private R2):** reachable **only from the VPS**, listening on localhost/private network. They do NOT authenticate people — trust is "you are inside the VPS". No extra 2FA at this layer.
- **Identity model:** identity = SSH key presence + physical YubiKey touch, verified by the VPS. Not tied to a device; tied to the person's hardware key.
- **Single session:** `MaxSessions 1` + `MaxStartups 1` in `sshd_config` — one live connection per VPS at a time.
- **RBAC:** access assigned by **Role and Project** (which VPS a person's key is authorized on), not by individual identity providers.
- **Audit:** SSH session identity injected into every event trace in the IDS for forensic analysis.
- **Management:** VPS provisioning and Tunnel setup 100% via API/Terraform — zero dashboard dependency, agent-automatable.

> **Why not Cloudflare Access / IdP / OTP for person login:** each was evaluated and rejected. Self-hosting (Forgejo) → operational overhead. Social identity (Google/Microsoft) → rejected. Okta → extra external dependency. Email OTP → requires giving each person an email. With email and all IdPs removed, Cloudflare Access has no way to identify a *person* (only a service token, which identifies a credential, not a person). Therefore person authentication is moved to the VPS itself via YubiKey, and Cloudflare is used purely as transport. See [docs/ops/stack-selection.md](./docs/ops/stack-selection.md#access-model).

> `<GOP>` = Git platform, pending. It governs **code access for technical staff only** (never employee VPS login). See [docs/ops/stack-selection.md](./docs/ops/stack-selection.md#gop-git--code-host-pending).

### VPS Authentication (YubiKey over SSH)

```
Person (any machine, carries YubiKey)
   │  ssh via cloudflared ProxyCommand (transport only)
   ▼
Cloudflare Tunnel  ── no public IP, no inbound ports, NO person login here
   │
   ▼
VPS sshd  ── authenticates the person HERE:
   ├─ Factor: hardware-backed SSH key (ed25519-sk / ecdsa-sk) → requires YubiKey touch
   │          (or pam_u2f as an explicit second factor)
   ├─ MaxSessions 1 / MaxStartups 1 → one connection at a time
   └─ Once inside → reaches internal resources (no further login)
```

- **Preferred:** FIDO2-backed SSH keys (`ssh-keygen -t ed25519-sk`). The private key cannot be used without the physical YubiKey present and touched. Single factor to configure, hardware-enforced. Works from any machine that has the key file + the YubiKey.
- **Editors:** VSCode / Cursor / JetBrains / Helix connect via **Remote-SSH** using `cloudflared access ssh` as `ProxyCommand`. Code and data stay on the VPS; the local machine is a thin client and is disposable — any machine works because nothing is stored locally.
- **Onboarding a new machine:** the person installs `cloudflared` + their SSH key file and plugs in their YubiKey. No re-enrollment of the machine needed; the identity is the key, not the device.

### Identity & Bootstrap Sequence

#### Prerequisites (one-time, manual)

| Step | Action | Cost |
|------|--------|------|
| 1 | Create Purelymail account ($10/yr) — `infra@purelymail.com` or similar | $10/yr |
| 2 | Buy domain (e.g. `<project_name>.co`) | ~$10/yr |
| 3 | Create Cloudflare account (using Purelymail email) | $0 |
| 4 | Add domain to Cloudflare (DNS + Email Routing) | $0 |
| 5 | Create `<GOP>` account with `infra@<project_name>.co` (routed via CF) | $0 |
| 6 | Create `<GOP>` Group `<project_name>-sas` | $0 |
| 7 | Provision a YubiKey per person; generate FIDO2 SSH key (`ssh-keygen -t ed25519-sk`) | ~$25-50/key |
| 8 | Create Hetzner account with `infra@<project_name>.co` | $0 |

After this, **everything is automatable by the agent.**

#### Email Architecture (no mailboxes)

```
Cloudflare Email Routing (free, unlimited aliases)
│
├── infra@<project_name>.co     → Worker → R2 (service accounts, 2FA, invoices)
├── <user>@<project_name>.co    → Worker → R2 (employee identity, confirmations)
├── <user2>@<project_name>.co   → Worker → R2 (employee identity, confirmations)
└── *@<project_name>.co         → catch-all → Worker → R2 (anything else)

No IMAP. No mailbox. No email client.
Worker stores as JSON in R2. Agent reads R2 when needed.
```

Purelymail ($10/yr) exists ONLY as:
- Bootstrap email to create Cloudflare account
- Recovery/fallback if Cloudflare access is lost

#### Onboarding Flow (Agent-Executable)

```bash
# === FULL ONBOARDING: "Add <user> as employee" ===

# 1. Create corporate email alias (Cloudflare API)
curl -X POST "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/email/routing/rules" \
  -H "Authorization: Bearer $CF_TOKEN" \
  -d '{"name": "<user>", "enabled": true,
       "matchers": [{"type": "literal", "field": "to", "value": "<user>@<project_name>.co"}],
       "actions": [{"type": "worker", "value": ["email-to-r2"]}]}'

# 2. Create <GOP> account with <user>@<project_name>.co
# Agent navigates <GOP> signup, confirmation arrives in R2,
# agent extracts confirmation link and activates account

# 3. Invite to <GOP> group
curl -X POST "https://<gop-host>/api/v4/groups/<project_name>-sas/invitations" \
  -H "PRIVATE-TOKEN: $GOP_TOKEN" \
  -d "email=<user>@<project_name>.co&access_level=30"

# 4. Provision VPS (Hetzner API)
hcloud server create --name worker-<user> --type cx22 --image ubuntu-24.04

# 5. Install dev environment
ssh root@<ip> "curl -L https://nixos.org/nix/install | sh && \
  home-manager switch --flake <GOP>:<org>/dev-setup#server"

# 6. Install cloudflared + create tunnel (transport ONLY, no person login)
ssh root@<ip> "curl -L https://pkg.cloudflare.com/cloudflared-linux-amd64 \
  -o /usr/local/bin/cloudflared && chmod +x /usr/local/bin/cloudflared"
# Create tunnel via CF API + configure + start

# 7. Enroll the person's YubiKey public key on the VPS (person auth happens HERE)
#    The person generated it once with: ssh-keygen -t ed25519-sk  (requires YubiKey)
ssh root@<ip> "mkdir -p /home/<user>/.ssh && \
  echo '<user-ed25519-sk-public-key>' >> /home/<user>/.ssh/authorized_keys"

# 8. Harden sshd: FIDO2 key required, single session
ssh root@<ip> "sed -i 's/^#*MaxSessions.*/MaxSessions 1/;s/^#*MaxStartups.*/MaxStartups 1/;\
  s/^#*PasswordAuthentication.*/PasswordAuthentication no/;\
  s/^#*PubkeyAuthentication.*/PubkeyAuthentication yes/' /etc/ssh/sshd_config && \
  systemctl restart sshd"

# === RESULT ===
# <user> connects with: ssh via cloudflared ProxyCommand → VPS sshd
# authenticates with their FIDO2 SSH key (YubiKey touch). No email, no IdP, no browser login.
# Lands in Nix environment (Helix + Zellij), portable from any machine that holds the key + YubiKey.
```

#### Offboarding Flow (Agent-Executable)

```bash
# === FULL OFFBOARDING: "Remove <user>" ===

# 1. Remove from <GOP> group (revokes code access — technical staff only)
curl -X DELETE "https://<gop-host>/api/v4/groups/<project_name>-sas/members/$USER_ID" \
  -H "PRIVATE-TOKEN: $GOP_TOKEN"

# 2. Destroy VPS (this alone kills person access: the YubiKey key only lived here)
hcloud server delete worker-<user>

# 3. Delete tunnel (CF API) — the transport for that VPS
# 4. Disable email alias (CF API)
curl -X PUT "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/email/routing/rules/$RULE_ID" \
  -H "Authorization: Bearer $CF_TOKEN" \
  -d '{"enabled": false}'

# === RESULT ===
# <user> loses: VPS access (their authorized_keys died with the VPS), <GOP> code access, email.
# The YubiKey itself is now useless for this company — no VPS trusts it anymore.
# Data on VPS is destroyed. Data in R2 retained for audit.
```

#### Cost Summary

| Component | Cost | What it provides |
|-----------|------|-----------------|
| Purelymail | $10/yr | Bootstrap email (create CF account) |
| Domain | ~$10/yr | Corporate identity namespace |
| Cloudflare (Email Routing + Tunnel + R2) | $0 | Transport + email-to-R2 + storage |
| `<GOP>` | $0 | Code hosting (technical staff only) |
| Hetzner per worker | ~$5/mo | VPS (dev environment) |
| YubiKey per person | ~$25-50 one-time | Hardware auth key (portable identity) |
| **Base (you alone)** | **$20/yr + 1 YubiKey** | Full infra |
| **Per employee** | **+$5/mo + 1 YubiKey** | Their VPS + hardware key |

### Architecture

```
Worker (any machine, carries YubiKey)
      │
      ▼ ssh via cloudflared ProxyCommand  (transport only, no person login)
┌─────────────────────────────────────┐
│  Cloudflare Tunnel (outbound-only)  │
│  No public IP required on VPS       │
│  No firewall ports to open          │
│  1 binary: cloudflared              │
│  Transport ONLY — no IAP, no IdP    │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│  VPS (Hetzner/Oracle)               │
│  sshd authenticates the person:     │
│    - FIDO2 SSH key (ed25519-sk)     │
│      → YubiKey touch required       │
│    - MaxSessions 1 (one at a time)  │
│  TDS/OLTP DB + Dev Environment      │
│  Internal resources: localhost only │
└─────────────────────────────────────┘
```

### Why NOT a VPN mesh or an IdP-brokered IAP

The decided model is **Cloudflare Tunnel (transport) + VPS-side FIDO2 SSH (person auth)**. Alternatives were evaluated and rejected:

| Concern | Nebula + Pomerium | Cloudflare Access + IdP | **Tunnel + YubiKey/SSH (decided)** |
|---------|-------------------|-------------------------|-------------------------------------|
| Services to maintain | 2 daemons + CA + OAuth app | Access policies + an IdP | 1 binary (`cloudflared`) + native OpenSSH |
| Person identity | Certificate per host | Requires email/IdP per person | Physical YubiKey (no email, no IdP) |
| Portability across machines | Manual cert re-issue | Device/session-bound | Any machine + the key file + YubiKey |
| Certificate/TLS management | Manual (`nebula-cert`) | Automatic (Cloudflare) | Automatic (Cloudflare) for transport |
| External identity dependency | Self-run OAuth | Google/Okta/etc. | None — auth is local to the VPS |
| Scriptable | Partially | 100% API/Terraform | 100% API/Terraform (VPS + tunnel) |
| Cost | $0 + your time | $0 up to 50 users | $0 + one YubiKey per person |

The IdP-brokered options were rejected because email and all IdPs were deliberately removed (see the elimination chain above): with no email and no IdP, Cloudflare Access cannot identify a *person*. Person authentication therefore lives on the VPS via the YubiKey; Cloudflare is used purely as the transport cable.

### Automation (Agent-Executable)

All operations are API-driven. An agent (OAS or CAG) can execute:

```bash
# === ONBOARDING (new worker) ===

# 1. Provision VPS
hcloud server create --name worker-<user> --type cx22 --image ubuntu-24.04

# 2. Install dev environment on VPS (via SSH)
ssh root@<ip> "curl -L https://nixos.org/nix/install | sh && \
  nix run home-manager/master -- init --flake <GOP>:<org>/dev-setup#server && \
  home-manager switch --flake <GOP>:<org>/dev-setup#server"

# 3. Install cloudflared on VPS (transport only)
ssh root@<ip> "curl -L https://pkg.cloudflare.com/cloudflared-linux-amd64 -o /usr/local/bin/cloudflared && \
  chmod +x /usr/local/bin/cloudflared"

# 4. Create tunnel (Cloudflare API) — transport, NOT person login
curl -X POST "https://api.cloudflare.com/client/v4/accounts/$ACCOUNT_ID/cfd_tunnel" \
  -H "Authorization: Bearer $CF_TOKEN" \
  -d '{"name": "worker-<user>", "tunnel_secret": "'$(openssl rand -base64 32)'"}'

# 5. Enroll the person's FIDO2 SSH public key + harden sshd (person auth on the VPS)
ssh root@<ip> "echo '<user-ed25519-sk-public-key>' >> /home/<user>/.ssh/authorized_keys && \
  sed -i 's/^#*MaxSessions.*/MaxSessions 1/;s/^#*PasswordAuthentication.*/PasswordAuthentication no/' \
  /etc/ssh/sshd_config && systemctl restart sshd"

# 6. Run the tunnel on the VPS
ssh root@<ip> "cloudflared tunnel run worker-<user>"

# === OFFBOARDING (revoke access) ===

# 1. Destroy VPS: hcloud server delete worker-<user>  (kills the authorized_keys trust)
# 2. Delete tunnel (API)
# 3. Remove from <GOP> group (revokes code access)
```

### Terraform (Declarative, Git-Versioned)

```hcl
# infrastructure/workers.tf

variable "workers" {
  # ssh_pubkey = the person's FIDO2 (ed25519-sk) SSH public key. Identity = their YubiKey.
  type = map(object({ ssh_pubkey = string }))
  default = {
    <user>  = { ssh_pubkey = "sk-ssh-ed25519@openssh.com AAAA... <user>@yubikey" }
    <user2> = { ssh_pubkey = "sk-ssh-ed25519@openssh.com AAAA... <user2>@yubikey" }
  }
}

resource "hcloud_server" "worker" {
  for_each    = var.workers
  name        = "worker-${each.key}"
  server_type = "cx22"
  image       = "ubuntu-24.04"
  # Person auth is on the VPS: only this FIDO2 key can log in (YubiKey touch required).
  user_data = templatefile("${path.module}/cloud-init.yaml.tftpl", {
    username   = each.key
    ssh_pubkey = each.value.ssh_pubkey  # -> authorized_keys; sshd: MaxSessions 1, PasswordAuthentication no
  })
}

# Transport only — no public IP, no inbound ports. Cloudflare does NOT authenticate the person.
resource "cloudflare_zero_trust_tunnel_cloudflared" "worker" {
  for_each   = var.workers
  account_id = var.cf_account_id
  name       = "worker-${each.key}"
  secret     = random_id.tunnel_secret[each.key].b64_std
}

# NOTE: no cloudflare_zero_trust_access_application / _policy.
# Person identity is the FIDO2 SSH key on the VPS, not a Cloudflare Access + IdP policy.
```

### Worker Access Flow

1. Worker runs `ssh <vps>` where the SSH config uses `cloudflared access ssh --hostname <user>.<project_name>.co` as `ProxyCommand` (Cloudflare = transport only, no login prompt).
2. Connection routes through the Tunnel to the VPS `sshd`.
3. `sshd` authenticates the person: the FIDO2 SSH key (`ed25519-sk`) requires the physical **YubiKey touch**. No email, no IdP, no browser login.
4. `MaxSessions 1` rejects a second concurrent connection.
5. Worker lands in their Nix environment (Helix, Zellij, VSCode/Cursor via Remote-SSH — full dev stack).
6. Data never leaves the VPS — the local machine is a thin client and disposable; any machine + the YubiKey works.

## 3. Complex Event Processing (CEP) Alerts

Eliminate polling and BI dashboards through push-based alerts in the EPE (functional iterators):

| Alert Type | Logic (CEP) | Example |
| :--- | :--- | :--- |
| **Accumulation** | Metric exceeds threshold in time window. | Counter of high-value transactions per account > Limit. |
| **Correlation** | Event A occurred but expected Event B did not follow. | "Due Date Expired" without "Payment Created" within 1h. |
| **Deviation** | Significant difference vs historical average. | Ratio of issues closed without valid ID rises significantly. |
| **Threshold** | Event changes state past a defined limit. | Account balance falls below threshold after transaction. |

## 4. Documentation Strategy (DaC)

- **Non-Technical:** Statutes and agreements in a central repo, published as GOP Pages.
- **Technical:** API and environment docs live within each repo (`Docs-in-Repo`).
- **Visual:** Use **Markmap** for planning and **Mermaid** for architecture diagrams.

# Document Governance — Voting and Legal Documents Pattern

> **STATUS: REVIEW PENDING** — This pattern must be reconsidered before implementation.

Reusable pattern to manage legal documents with traceability, formal voting, and digital signature in any company.

---

## Concept

Every company needs a system to:
1. Create and version legal documents (minutes, protocols, contracts)
2. Formally vote on decisions affecting the company
3. Digitally sign with legal validity
4. Keep an auditable record of the entire process

---

## Schema: Legal Document Metadata

Control table that defines which flow each document type follows.

| Field | Type | Purpose |
|---|---|---|
| document_id | VARCHAR(50) PK | Unique document ID |
| short_name | VARCHAR(100) | Descriptive name |
| repo_file | VARCHAR(255) | File path in the repository |
| legal_type | VARCHAR(50) | Classification: Assembly_Minutes, Internal_Protocol, etc. |
| req_vote | BOOLEAN | TRUE if formal voting is required |
| req_pki_signature | BOOLEAN | TRUE if a certified signature (PKI) is required |
| req_chamber_registration | BOOLEAN | TRUE if it must be registered with the Chamber of Commerce |
| active_version | VARCHAR(64) | Commit hash of the legally active version |

---

## Schema: Voting Record

Records the explicit vote and the outcome of the signing process.

| Field | Type | Purpose |
|---|---|---|
| vote_id | SERIAL PK | Voting session ID |
| document_id | VARCHAR(50) FK | Document it applies to |
| mr_id | INTEGER UNIQUE | ID of the Merge Request with the draft |
| final_result | VARCHAR(20) | APPROVED or REJECTED |
| votes_for | INTEGER | Count of signatures/votes in favor |
| votes_against | INTEGER | Count of votes against |
| signed_pdf_hash | VARCHAR(64) UNIQUE | SHA-256 of the final signed document |
| decision_date | TIMESTAMP | Date of the decision |

---

## General Flow

```
1. Draft → created in the repository (Markdown)
2. Merge Request → triggers the voting process
3. Voting → each shareholder votes (for / against)
4. If approved → PKI signature → immutable PDF
5. If registration required → a task is created for the Chamber of Commerce
6. The PDF hash is recorded as the active version
```

---

## Notes for review

- Evaluate whether the MR as a voting mechanism is adequate or whether it should be simpler
- Evaluate whether PKI signature is necessary for all documents or only critical ones
- Consider an alternative: signing with an age key (already used in dev-setup) instead of formal PKI
- Define whether this lives as code (SQL migrations) or only as reference documentation

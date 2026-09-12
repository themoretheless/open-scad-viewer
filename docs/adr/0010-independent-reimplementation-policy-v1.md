# ADR 0010: Independent reimplementation and contributor provenance

- Status: accepted
- Date: 2026-09-12
- Accepted-by: repository-owner (human authorization in session)
- Reviewer: repository-owner (solo dual-role attestation; not organizationally independent)
- Contract: `independent-reimplementation-policy-v1`
- Artifacts:
  - `docs/qualification/allowlisted-sources-v1.json`
  - `docs/qualification/rights-inventory-v1.json`
  - `docs/qualification/provenance/PROVENANCE.example.yml`

## Context

Design §23 requires a documented independent-reimplementation policy, an
allowlisted source log, rights inventory, and per-algorithm `PROVENANCE.yml`
before high-risk geometry capabilities ship. This is not automatically a formal
clean-room label.

## Decision

### Policy

Implement from published mathematics or dated internal design records. Do not
translate foreign kernel code. Clean-room labeling requires a separate protocol
(specification vs implementation roles, exposure declarations, approvals).

### Allowlisted idea sources (v1 seed)

The machine log in `allowlisted-sources-v1.json` seeds the published
references from design §23 (Cox, Boehm, Piegl–Tiller, Shewchuk paper mode,
Sederberg–Nishita, Grandine–Klein, Bentley–Ottmann, Piegl–Richard, Requicha).
New sources require an inventory amendment before use.

### Shewchuk modes

Exactly one mode per module: `original_from_paper` | `derived_port` |
`vendored`. ADR 0006 selected `original_from_paper` for G2a. Ports/vendors need
a new ADR, upstream hash, notice, and approved `LicenseRef`.

### Forbidden inputs

Proprietary source, decompilation, debug traces, closed SDK/NDA docs, private
commercial-kernel fixtures. Publisher PDFs are not redistributed. Open CASCADE
source is out of bounds as an implementation reference under this voluntary
policy.

### PROVENANCE.yml

Every qualified/production algorithm keeps a versioned file matching the
example schema. Missing required fields for the risk tier keeps the capability
experimental.

### Rights inventory

`rights-inventory-v1.json` tracks outbound license intent (`MIT OR Apache-2.0`
proposed), inbound=outbound expectation, and whether DCO/CLA is decided.
Outbound license text is not activated by this ADR alone.

### Incident response

Disallowed source discovery quarantines affected paths; releases block until
the incident record closes under an exposure-cleared rewrite protocol.

## Consequences

- G0.13 has machine-readable seeds for review
- Legal FTO / patent review remains outside the public repo (approval IDs only)

## Acceptance gates

1. this ADR is human-accepted;
2. allowlisted sources + rights inventory + example PROVENANCE validate;
3. at least one foundational algorithm row cites the example schema;
4. no production module claims clean-room without the separate protocol.

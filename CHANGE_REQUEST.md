# SARA Fork — Change Request

This document describes the capabilities needed to adapt SARA for investigative knowledge management. It focuses on **what** is required, not how to implement it. Each section references the project with the best existing vision for that capability.

The consumer project is [Wombatt](https://github.com/jamps/wombatt), an open-source investigation platform. The first investigation is the DOJ Epstein Files — ~70 markdown claim files, 2,691+ EFTA document citations, 8 claim types, and an inference DAG that compounds through forward chaining.

---

## Prior art

Projects evaluated before choosing SARA as the base. Each contributed to the design thinking below.

| Project | What it does well | What it lacks | Relevance to this change request |
|---|---|---|---|
| [SARA](https://github.com/cledouarec/sara) | Typed relations in YAML frontmatter, bidirectional inference, knowledge graph traversal (`petgraph`), multi-repo, `sara diff` | Fixed document/relation types, no per-link change detection, no review tracking | **Base project.** The typed relation model is the right architecture. Sections 1–2 generalize what SARA already does. |
| [Doorstop](https://github.com/doorstop-dev/doorstop) | SHA-stamped suspect link detection, per-item review tracking, configurable fingerprint fields (`attributes.reviewed`), 12 years of edge-case handling | Flat untyped `links` array, no relation semantics, document hierarchy forces tree structure | **Reference for sections 4–7.** Fingerprinting, suspect links, review tracking, `header`, `derived`/`normative` — all proven by Doorstop. |
| [StrictDoc](https://github.com/strictdoc-project/strictdoc) | Custom grammars per document type, typed relations with roles (Refines, Implements, Verifies), Machine Identifiers (MIDs) for change tracking, ReqIF export | Custom DSL (not markdown), requires running a server, targets aerospace/regulatory compliance | **Reference for section 1.** StrictDoc's custom grammar system proves that document types and their fields can be fully user-defined rather than hardcoded. |
| [Sphinx-Needs](https://www.sphinx-needs.com/) | Typed need objects with custom fields, rich reporting (tables, flowcharts, pie charts), mature ecosystem | Locked to Sphinx documentation system | **Reference for reporting.** Demonstrates that traceability matrices and coverage reports are standard expectations for this class of tool. |
| [Reqflow](https://goeb.github.io/reqflow/) | Cross-document traceability analysis across Word, PDF, HTML | Read-only analysis, doesn't manage items | Validated that cross-document traceability is a common need. |
| [git-reqs](https://github.com/niradynamics/git-reqs) | Simple YAML + Git requirements management | Minimal features, no validation depth, no change detection | Confirmed that YAML + Git is the right storage model but insufficient alone. |

---

## 1. Investigation document types

> **Status:** Implemented (`001dcb3`). Types are hardcoded enum variants, not config-driven.

**Reference:** Wombatt [claim-model.md](https://github.com/jamps/wombatt/blob/main/docs/claim-model.md) — 8 claim types with per-type field constraints and outcome lifecycles.

SARA's original 10 systems-engineering document types remain unchanged. 8 investigation types are added alongside them as new `ItemType` enum variants.

**Decision:** A prior attempt at config-driven types broke 64 compilation sites across 30 files. The enum-based approach preserves compile-time exhaustiveness checking and requires zero architectural changes. Config-driven types can be revisited if a second investigation needs a different taxonomy.

**Implemented types:**

| Type | Prefix | Purpose | Root? |
|------|--------|---------|-------|
| `entity` | ITM | Entity definition — graph node | Yes |
| `evidence` | EVD | Cited evidentiary claim about entities | No |
| `thesis` | THS | Investigation container + conclusion | Yes |
| `hypothesis` | HYP | Competing explanation with independent outcome | No |
| `analysis` | ANL | Evaluates evidence against hypotheses | No |
| `premise` | PRM | Verified predicate for forward chaining | No |
| `question` | QST | Information gap | No |
| `block` | BLK | Access/capability gap | Yes |

**Not yet implemented:** Per-type field constraints, allowed outcome values, fingerprint field configuration (see section 4).

---

## 2. Investigation relation types

> **Status:** Implemented (`001dcb3`). Relations are hardcoded enum variants, not config-driven.

**Reference:** SARA's existing typed relations (`derives_from`, `satisfies`, `depends_on`) — the right model. Wombatt [claim-model.md § Linking Conventions](https://github.com/jamps/wombatt/blob/main/docs/claim-model.md#linking-conventions) — the required relation vocabulary.

SARA's typed, bidirectional relations are extended with 10 new pairs (20 `RelationshipType` variants). Same decision as section 1 — hardcoded enums, not config-driven.

**Implemented relations:**

| Relation | Reverse | Direction | Source types | Target types |
|----------|---------|-----------|-------------|-------------|
| `parent` | `children` | Upstream | evidence, hypothesis, analysis, question | entity, thesis |
| `cites` | `cited_by` | Upstream | analysis | evidence |
| `evaluates` | `evaluated_by` | Upstream | analysis | hypothesis |
| `premises` | `premise_of` | Downstream | analysis | premise |
| `gaps` | `gap_of` | Downstream | analysis | question |
| `established_by` | `establishes` | Upstream | premise | thesis |
| `raised_by` | `raises` | Upstream | question | analysis, evidence |
| `affects` | `affected_by` | Peer | block | evidence, analysis, question |
| `hypotheses` | `hypothesis_of` | Downstream | thesis | hypothesis |
| `analyses` | `analysis_of` | Downstream | thesis | analysis |

Authors define the relation in one direction. SARA infers the reverse. The graph builder wires bidirectional edges automatically.

**Not yet implemented:** Cardinality constraints, config-driven validation of source/target type pairs.

---

## 3. N-ary participants with typed roles

> **Status:** Implemented (`6dab843`). Participants stored on `Item`, flattened to `Participant`/`ParticipantOf` graph edges.

**Reference:** Wombatt [claim-model.md § Participants format](https://github.com/jamps/wombatt/blob/main/docs/claim-model.md#participants-format) — N-ary participant model with named roles per relation type.

SARA's existing relations are binary (source item → target item). Investigative evidence often involves multiple entities in a single event, each with a distinct role.

**Required:** A `participants` field on items that declares multiple entity references with typed roles. The relation type determines which roles are valid.

**Example:**

```yaml
relation: hosted
participants:
  - entity: ITM-jeffrey-epstein
    role: host
  - entity: ITM-linda-stone
    role: guest
  - entity: ITM-301-east-66th
    role: venue
```

The knowledge graph flattens N-ary participants into binary edges for traversal and visualization. The participant model is the authoring representation; the graph is the derived representation.

Role definitions per relation type should be configurable:

| Relation | Valid roles |
|----------|------------|
| `hosted` | host, guest, venue |
| `met-at` | attendee, venue |
| `introduced` | introducer, introduced |
| `brokered` | broker, party |
| `command` | commander, subordinate |

---

## 4. Per-link fingerprint stamps and suspect link detection

> **Status:** Implemented (`9c559ce`, `a51faa8`, `75468c2`, `2bcf7a6`). SHA-256 fingerprints, per-link stamps, suspect link validation, `sara review` and `sara clear` commands.

**Reference:** Doorstop ([doorstop-dev/doorstop](https://github.com/doorstop-dev/doorstop)) — SHA-stamped suspect link detection. Wombatt [claim-model.md § Suspect Link Detection](https://github.com/jamps/wombatt/blob/main/docs/claim-model.md#suspect-link-detection) — truth maintenance via inference DAG.

This is the critical capability SARA currently lacks. SARA can detect broken references and cycles, but cannot detect **stale** links — references to items whose content has changed since the link was last reviewed.

**Required:**

- **Fingerprint:** Each item has a content fingerprint (SHA-256 hash of UID + body + configurable reviewed attributes). The fingerprint changes when semantically significant content changes.
- **Stamps:** Each outgoing relation stores the target item's fingerprint at the time the link was last reviewed. Stamps persist across edits to the source item.
- **Suspect detection:** On validation, compare each stored stamp against the target's current fingerprint. A mismatch means the target changed since the link was reviewed — flag as suspect.
- **Review command:** A CLI command that re-stamps all outgoing relations for a given item, recording that the author has reviewed the current state of all targets.
- **Clear command:** A CLI command that re-stamps a specific relation, for partial review.
- **Configurable fingerprint fields:** Per document type, declare which frontmatter fields contribute to the fingerprint beyond UID and body. When these fields change, downstream links become suspect.

**Example:** An evidence claim has `sourcing: C` in its fingerprint. If sourcing is changed to `X`, every analysis that cites this evidence gets a suspect link warning — the analysis was written assuming confirmed sourcing and needs review.

**Why `sara diff` is insufficient:** `sara diff` compares two graph snapshots globally. Suspect link detection is per-link and cumulative — it tracks which specific links have been reviewed and which haven't, across any number of commits. An investigator needs to know "these 3 analyses need review because their cited evidence changed" without diffing the entire graph.

---

## 5. Review tracking

> **Status:** Implemented (`75468c2`, `6dab843`). Per-item `reviewed` field, `UnreviewedItemsRule` validation, `sara review` command.

**Reference:** Doorstop — per-item `reviewed` field with fingerprint comparison. Doorstop [Item.review()](https://github.com/doorstop-dev/doorstop/blob/develop/doorstop/core/item.py) — marks item as reviewed by storing its own current fingerprint.

**Required:**

- Each item tracks whether it has been reviewed since its last modification.
- An item is "unreviewed" when its current fingerprint differs from its stored reviewed fingerprint.
- A CLI command marks an item as reviewed.
- Validation reports unreviewed items as warnings.
- When an item is reviewed, its outgoing relation stamps are also updated (the reviewer has seen the current state of all targets).

This integrates with suspect link detection: reviewing an item clears its suspect links and marks it as reviewed in one operation.

---

## 6. Display titles

> **Status:** Already covered. SARA's existing `name` field serves this purpose.

**Reference:** Doorstop — built-in `header` attribute for human-readable display names, excluded from the fingerprint.

SARA already has an optional `name` field on every item, used for display in CLI output, reports, and traceability matrices. This maps directly to Doorstop's `header` concept. When fingerprinting is implemented (section 4), `name` should be excluded from the fingerprint — changing a display title should not trigger suspect links.

---

## 7. Derived and non-normative items

> **Status:** Implemented (`6dab843`). Per-item `derived` and `normative` flags. Derived items suppress orphan warnings; non-normative items excluded from coverage.

**Reference:** Doorstop — `derived` and `normative` flags for traceability classification.

**Required:**

- **Derived:** Items not expected to have upstream links (e.g., root entities, top-level theses). Suppresses "orphaned item" warnings. Currently handled at the type level — `Entity`, `Thesis`, and `Block` return `is_root() = true`. A per-item `derived: true` frontmatter flag would allow any item to opt out of orphan warnings regardless of type.
- **Non-normative:** Items excluded from traceability checks and coverage reports (e.g., section headings, structural separators). Not yet implemented.

---

## 8. Structured evidence envelopes and entity-entity edges

> **Status:** Implemented (`aa62c3a`). MDX support, 4 envelope types, 5 entity-entity relation pairs, envelope validation, deprecated field lint, envelope fingerprinting.

**Reference:** Wombatt [claim-model.md](https://github.com/jamps/wombatt/blob/main/docs/claim-model.md) — structured envelope schemas (messages, deposition, flights, transactions).

Evidence items can now carry structured envelope data in frontmatter alongside participants. Each envelope type captures a specific kind of interaction between entities:

| Envelope | Fields | Entity-entity edge |
|----------|--------|--------------------|
| `messages` | from, to, cc, bcc, date, subject | `communicated_with` / `received_communication_from` |
| `deposition` | witness, exchanges (speaker, page, objection) | *(no edges — witness/speaker tracked via participants)* |
| `flights` | origin, destination, passengers, aircraft | `traveled_with` (symmetric co-occurrence) |
| `transactions` | from, to, amount, currency, method | `paid_to` / `received_payment_from` |

**Validation rules:**
- **EnvelopeRule**: Every entity UID in envelope data must appear in `participants`. At most one envelope type per evidence item. Envelope IDs must be unique within their array.
- **DeprecatedFieldsRule**: Configurable per-type deprecated field warnings via `sara.toml` for the MDX migration period.

**File discovery:** `.mdx` files are now accepted alongside `.md` and `.markdown`.

---

## Non-goals

These are explicitly out of scope for the fork:

- **GUI or web server.** The investigation uses Astro for its web UI and Forgejo for editing. SARA remains a CLI tool.
- **Publishing.** Astro handles HTML generation. SARA does not need to produce HTML, PDF, or LaTeX.
- **Import/export.** Not needed. Claims are authored as markdown files.
- **Prescribing a document hierarchy.** Unlike Doorstop's parent-child document tree, SARA's flat graph with typed relations is the right model. Do not add hierarchical document containers.

---

## Integration context

SARA will be called from a TypeScript/Bun build pipeline as a CLI tool:

```
sara check                     # validation (including suspect links)
sara review <ITEM-ID>          # mark item reviewed, re-stamp links
sara clear <ITEM-ID> <TARGET>  # clear specific suspect link
sara query <ITEM-ID>           # upstream/downstream traversal
sara diff <REF1> <REF2>        # graph diff between Git refs
sara report matrix             # typed traceability matrix
sara report coverage           # coverage report
```

Configuration lives in `sara.toml` at the investigation repo root. Document types, relation types, role definitions, and fingerprint field configuration are all declared there.

The investigation repo (Epstein Files) and the platform repo (Wombatt) are separate Git repositories. SARA's existing multi-repo support covers this.

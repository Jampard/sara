# Fingerprinting, Suspect Links & Review Tracking — Design (Sections 4+5)

Adds per-link fingerprint stamps, suspect link detection, and review
tracking to SARA. Implements Change Request sections 4 and 5.

## Rationale

SARA can detect broken references and cycles, but cannot detect **stale**
links — references to items whose content has changed since the link was
last reviewed. This is the critical capability needed for investigation
workflows where evidence and analyses evolve over time.

Doorstop's SHA-stamped suspect link detection is the proven model.
This design adapts it to SARA's architecture.

## New fields

### Top-level on `Item`

| Field | Type | Purpose |
|-------|------|---------|
| `outcome` | `Option<String>` | Lifecycle state. All types. SARA reads but does not validate values (Zod does). |
| `reviewed` | `Option<String>` | Item's own fingerprint at last review. |
| `stamps` | `HashMap<ItemId, String>` | Target ID → target fingerprint at last review. |
| `body_hash` | `Option<String>` | SHA-256 of body content, computed during parsing. Used by validation without re-reading files. |

### `ItemAttributes` changes

Unit variants gain type-specific fields:

- `Evidence { sourcing: Option<String>, relation: Option<String> }`
- `Analysis { assessment: Option<String> }`
- `Hypothesis { assessment: Option<String> }`

All other investigation `ItemAttributes` variants remain unit structs.

### Field semantics

`outcome` and `assessment` are orthogonal axes:

- **outcome** — lifecycle state (mutable via PR). Tracks where the claim
  is in its investigation lifecycle. Values vary by type: `open` →
  `verified`/`disproven`/`inconclusive` (hypothesis, thesis), `open` →
  `blocked` (item), `active`/`suspended`/`retired` (premise).

- **assessment** — ICD-203 estimative language probability. Lives on
  hypothesis (optional) and analysis (required). 7-point scale: `remote`
  → `very-unlikely` → `unlikely` → `roughly-even` → `likely` →
  `very-likely` → `almost-certainly`.

They move independently. A hypothesis starts `outcome: open`,
`assessment: roughly-even`, accumulates evidence that shifts assessment
to `very-likely`, and eventually resolves to `outcome: verified`.

## Fingerprint computation

```
fingerprint(item) = SHA-256(id + body + outcome + type_specific_fields)
```

All values concatenated in deterministic order. Missing optional fields
contribute empty string. Type-specific fields sorted by field name.

### Fingerprinted fields per type

| Type | Beyond UID + body |
|------|-------------------|
| Evidence | `outcome`, `sourcing`, `relation` |
| Analysis | `outcome`, `assessment` |
| All others | `outcome` |

### Excluded from fingerprint

`name`, `description`, `stamps`, `reviewed`, all relation fields. Changing
display text or review metadata must not trigger suspect cascades.

### Computation strategy

Fingerprints are computed on the fly from parsed document content. Never
stored in frontmatter — always derived. `body_hash` is computed once
during parsing and stored on `Item` so validation doesn't re-read files.

## Stamps

Stored in each item's YAML frontmatter:

```yaml
stamps:
  EVD-001: "a3f2b1c9"
  HYP-003: "9c8d7e4a"
```

Truncated SHA-256 (first 8 hex chars) for frontmatter readability.
Compared at full length internally.

Keyed by target ID. Written by `sara review` and `sara clear`.

## Suspect link detection

New validation rule: `SuspectLinksRule` implementing `ValidationRule`.

During `validate(graph)`:

1. For each item, collect all outgoing relation target IDs
2. For each target, compute current fingerprint from `body_hash` +
   `outcome` + type-specific fields
3. Compare against `stamps[target_id]`:
   - No stamp → **suspect** (never reviewed)
   - Mismatch → **suspect** (target changed since review)
   - Match → clean

Severity: **Warning**. Output includes source item, suspect target,
and reason (unreviewed / stale).

## Review tracking

`reviewed` field in frontmatter — the item's own fingerprint at last review:

```yaml
reviewed: "b4e8f2a1"
```

Unreviewed when `reviewed` is absent or mismatches current fingerprint.
Reported by `SuspectLinksRule` as a separate warning category.

`sara review <ITEM-ID>`:

1. Parse the item's document
2. Compute fingerprint → write to `reviewed`
3. For each outgoing relation target, compute target fingerprint → write to `stamps`
4. Write updated frontmatter (preserving body)

`sara clear <ITEM-ID> <TARGET-ID>`:

1. Parse source item's document
2. Compute target's current fingerprint
3. Update `stamps[TARGET-ID]` only
4. Write updated frontmatter (preserving body)

Both use the existing `EditService` pattern.

## New module: `fingerprint/`

`sara-core/src/fingerprint/`:

- `mod.rs` — public API
- `compute.rs` — `compute_fingerprint(id, body, outcome, type_fields) -> String`
- `config.rs` — `fingerprinted_fields(item_type) -> &[&str]` (hardcoded match)

## Parser changes

`RawFrontmatter` gains:

- `outcome: Option<String>`
- `sourcing: Option<String>`
- `relation: Option<String>`
- `assessment: Option<String>`
- `reviewed: Option<String>`
- `stamps: HashMap<String, String>`

`ItemBuilder` extended to accept all new fields. Body hash computed
during `parse_markdown_file()`.

## Changes per module

| Module | Change |
|--------|--------|
| `model/item.rs` | `outcome`, `reviewed`, `stamps`, `body_hash` on Item. Fields on Evidence/Analysis/Hypothesis attributes. |
| `parser/markdown.rs` | Parse new fields. Compute `body_hash` during parse. |
| `fingerprint/` | New module — hash computation + field config |
| `validation/rules/suspect_links.rs` | New `SuspectLinksRule` |
| `edit/service.rs` | Extend to write `reviewed`, `stamps`, `outcome`, `assessment`, `sourcing`, `relation` |
| `sara-cli/commands/review.rs` | New `sara review` command |
| `sara-cli/commands/clear.rs` | New `sara clear` command |
| `template/` | Add new fields to investigation templates |
| `init/` | Add `outcome` to init options |

## What stays unchanged

- Graph construction and traversal
- Existing 7 validation rules
- Diff module
- All existing SE types and relations
- All 230 existing tests

# Investigation Types — Design (Sections 1+2)

Adds 8 investigation document types and 10 relation pairs to SARA's existing
enum-based type system. No architectural changes, no breaking changes.

Implements Change Request sections 1 (Configurable document types) and
2 (Configurable relation types) via static enum extension rather than
runtime configuration.

## Rationale

The change request calls for config-driven types. A prior attempt replaced
the `ItemType` and `RelationshipType` enums with dynamic string-based types,
breaking 64 compilation sites across 30 files.

This design adds the investigation types as new enum variants instead.
The two domains (SE and investigation) are fully independent — different
prefixes, different relations, different traceability rules. They coexist
in the binary but never cross paths at runtime.

Config-driven types can be added later if a second investigation needs a
different taxonomy. For now, YAGNI.

## New `ItemType` variants

| Variant | `as_str()` | Prefix | `display_name()` | Root? |
|---------|-----------|--------|-------------------|-------|
| `Entity` | `"entity"` | `ITM` | `"Entity"` | Yes |
| `Evidence` | `"evidence"` | `EVD` | `"Evidence"` | No |
| `Thesis` | `"thesis"` | `THS` | `"Thesis"` | Yes |
| `Hypothesis` | `"hypothesis"` | `HYP` | `"Hypothesis"` | No |
| `Analysis` | `"analysis"` | `ANL` | `"Analysis"` | No |
| `Premise` | `"premise"` | `PRM` | `"Premise"` | No |
| `Question` | `"question"` | `QST` | `"Question"` | No |
| `Block` | `"block"` | `BLK` | `"Block"` | Yes |

`Entity` avoids collision with the `Item` struct. Frontmatter uses
`type: entity`, prefix is `ITM`.

`Block` is root because it represents external constraints with no
upstream traceability.

## New `RelationshipType` variants

10 pairs (20 variants). Each follows the existing bidirectional pattern.

| Primary | Inverse | Direction | Source types | Target types |
|---------|---------|-----------|-------------|-------------|
| `Parent` | `Children` | Upstream | evidence, hypothesis, analysis, question | entity, thesis |
| `Cites` | `CitedBy` | Upstream | analysis | evidence |
| `Evaluates` | `EvaluatedBy` | Upstream | analysis | hypothesis |
| `Premises` | `PremiseOf` | Downstream | analysis | premise |
| `Gaps` | `GapOf` | Downstream | analysis | question |
| `EstablishedBy` | `Establishes` | Upstream | premise | thesis |
| `RaisedBy` | `Raises` | Upstream | question | analysis, evidence |
| `Affects` | `AffectedBy` | Peer | block | evidence, analysis, question |
| `Hypotheses` | `HypothesisOf` | Downstream | thesis | hypothesis |
| `Analyses` | `AnalysisOf` | Downstream | thesis | analysis |

### Direction classification

- **Upstream** (source points to parent/ancestor): Parent, Cites, Evaluates,
  EstablishedBy, RaisedBy
- **Downstream** (source points to child/descendant): Premises, Gaps,
  Hypotheses, Analyses
- **Peer** (lateral constraint): Affects

## New `FieldName` variants

One per relation (20 total) plus investigation-specific fields:

**Upstream relation fields:** Parent, Cites, Evaluates, EstablishedBy, RaisedBy

**Downstream relation fields:** Children, CitedBy, EvaluatedBy, Establishes,
Raises, Premises, PremiseOf, Gaps, GapOf, Hypotheses, HypothesisOf,
Analyses, AnalysisOf

**Peer fields:** Affects, AffectedBy

## `UpstreamRefs` additions (+9 fields)

Declared by author (upstream): `parent`, `cites`, `evaluates`,
`established_by`, `raised_by`

Auto-inferred inverses of downstream relations: `premise_of`, `gap_of`,
`hypothesis_of`, `analysis_of`

## `DownstreamRefs` additions (+9 fields)

Declared by author (downstream): `premises`, `gaps`, `hypotheses`, `analyses`

Auto-inferred inverses of upstream relations: `children`, `cited_by`,
`evaluated_by`, `establishes`, `raises`

## Peer refs

`affects` / `affected_by` — same pattern as existing `depends_on` /
`is_required_by`.

## `ItemAttributes` additions

8 new variants. Investigation types have simpler attribute needs than
SE types — no `specification` or `platform` fields. Specific fields
TBD during implementation based on the Wombatt claim model.

## `RawFrontmatter` additions

14 new optional `Vec<String>` fields for investigation relations
(author-declared directions only, both upstream and downstream).

## `RelationshipRules` additions

`valid_upstream_for()`, `valid_downstream_for()`, `valid_peer_for()`
extended with match arms per the source/target type constraints above.

## What stays unchanged

- Graph infrastructure (`KnowledgeGraph`, `KnowledgeGraphBuilder`)
- Validation engine and rules
- CLI commands
- Output formatting (uses `display_name()` which we extend)
- Config (`sara.toml`) — no schema section needed
- All existing tests continue passing

## Estimated LOC

~840 lines of mechanical additions. No existing lines modified beyond
extending match arms.

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is SARA?

SARA (Solution Architecture Requirement for Alignment) is a CLI tool that manages architecture documents and requirements as an interconnected knowledge graph. Documents are plain Markdown files with YAML frontmatter, stored in Git repositories.

## Commands

```bash
# Build
cargo build

# Run all tests (unit + integration)
cargo test --workspace --all-targets

# Run a single test
cargo test --workspace test_name

# Run tests for a specific crate
cargo test -p sara-core
cargo test -p sara-cli

# Lint (CI uses -D warnings)
cargo clippy --workspace --all-targets -- -D warnings

# Format check
cargo fmt --all -- --check

# Format fix
cargo fmt --all
```

## Architecture

Cargo workspace with two crates:

- **`sara-core`** — Library crate with all business logic. No CLI dependencies.
- **`sara-cli`** — Binary crate (`sara`) that provides the CLI interface using clap. Depends on `sara-core`.

### sara-core modules

| Module | Purpose |
|--------|---------|
| `model/` | Domain types: `Item`, `ItemId`, `ItemType` (18 types: 10 SE + 8 investigation), `RelationshipType`, `UpstreamRefs`/`DownstreamRefs`, `ItemAttributes` (type-specific enum), `ItemBuilder`, envelope structs (`EnvelopeMessage`, `EnvelopeDeposition`, `EnvelopeFlight`, `EnvelopeTransaction`) |
| `graph/` | `KnowledgeGraph` (wraps `petgraph::DiGraph<Item, RelationshipType>`) with `HashMap<ItemId, NodeIndex>` index. `KnowledgeGraphBuilder` does two-pass construction: add nodes, then resolve relationships |
| `parser/` | Extracts YAML frontmatter from Markdown/MDX files, maps to `Item` via `ItemBuilder`. Raw envelope serde types convert string UIDs to `ItemId` |
| `repository/` | File discovery via glob patterns across multiple repo paths. `GitReader` reads files at specific git refs |
| `validation/` | Rule-based validator with individual rules in `validation/rules/` (broken_refs, cycles, duplicates, metadata, orphans, redundant, relationships, suspect_links, unreviewed, envelope, deprecated) |
| `query/` | Traceability traversal — upstream/downstream chain walking |
| `report/` | Coverage reports and traceability matrices (text, JSON, CSV output) |
| `diff/` | Compares two `KnowledgeGraph` instances (e.g., between git refs) |
| `config/` | TOML config file loading (`sara.toml`) |
| `template/` | Tera-based document template generation |
| `init/` | New document initialization with frontmatter |
| `edit/` | In-place editing of existing document metadata |
| `fingerprint/` | SHA-256 content fingerprinting and review stamp management |
| `error.rs` | Error hierarchy: `SaraError` > `ParseError`, `ValidationError`, `ConfigError`, `QueryError`, `GitError`, `EditError` |

### sara-cli modules

| Module | Purpose |
|--------|---------|
| `commands/` | One file per subcommand: `check`, `clear`, `diff`, `edit`, `init`, `query`, `report`, `review`. Shared `CommandContext` holds output config + repo paths |
| `output/` | Colored/emoji terminal formatting, respects `--no-color`/`--no-emoji`/`NO_COLOR` env |

### Key design patterns

- **Two-pass graph building**: `KnowledgeGraphBuilder.build()` first adds all items as nodes, then resolves relationships as edges. This handles forward references.
- **Bidirectional relationships**: Defining a relationship in one direction (e.g., `refines`) auto-creates the inverse edge (`is_refined_by`). Both upstream and downstream refs on `Item` are checked.
- **Type-specific attributes via enum**: `ItemAttributes` is a tagged enum — each `ItemType` maps to a variant with its own fields (e.g., `SystemRequirement { specification, depends_on }`).
- **Validation rules as separate structs**: Each rule in `validation/rules/` implements a trait, composed by the `Validator`.
- **Entity-entity edges from envelopes**: Evidence envelope data (messages, flights, transactions) is used to build peer edges between entities during graph construction, with HashSet deduplication per entity pair per evidence item.

## Rust edition and toolchain

- Edition: 2024 (set in `Cargo.toml`)
- MSRV: 1.93.0
- Formatting: `rustfmt.toml` — max_width=100, tab_spaces=4
- Clippy: `clippy.toml` — msrv="1.93"

## Testing

- Unit tests live alongside source code (`#[cfg(test)] mod tests`)
- Integration test fixtures in `tests/fixtures/` (valid_graph, broken_refs, cycles, duplicates, orphans, edit_tests, investigation, envelope)
- CLI integration tests in `sara-cli/tests/cli_tests.rs` using `assert_cmd`
- `sara-core/src/test_utils.rs` provides helpers like `create_test_item`, `create_test_item_with_upstream`, `create_test_adr`

## Commit convention

Conventional Commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `build:`, `ci:`

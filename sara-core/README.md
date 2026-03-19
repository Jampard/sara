# sara-core

Core library for [SARA](https://github.com/cledouarec/sara) - **S**olution **A**rchitecture **R**equirement for **A**lignment.

This crate provides the business logic for managing architecture documents and requirements as an interconnected knowledge graph.

## Features

- **Knowledge Graph** - Build and traverse a graph of requirements and architecture documents
- **Markdown/MDX Parsing** - Parse Markdown and MDX files with YAML frontmatter
- **Multi-Repository Support** - Aggregate documents from multiple Git repositories
- **Validation** - Detect broken references, orphaned items, circular dependencies, duplicate identifiers, suspect links, unreviewed items, and envelope integrity issues
- **Traceability Queries** - Traverse upstream or downstream relationships
- **Reports** - Generate coverage reports and traceability matrices
- **Version Comparison** - Compare knowledge graphs between Git commits or branches
- **Fingerprinting** - SHA-256 content fingerprints with per-link suspect detection and review tracking
- **Templates** - Generate YAML frontmatter templates for new documents

## Usage

```bash
cargo add sara-core
```

### Example

```rust,no_run
use std::path::Path;
use sara_core::{
    config::load_config,
    repository::parse_directory,
    graph::KnowledgeGraphBuilder,
    validation::Validator,
    traverse_upstream,
    ItemId, TraversalOptions,
};

fn main() -> sara_core::Result<()> {
    // Load configuration
    let config = load_config(Path::new("sara.toml"))?;

    // Discover and parse documents from a directory
    let items = parse_directory(Path::new("./docs"))?;

    // Build the knowledge graph
    let graph = KnowledgeGraphBuilder::new()
        .add_items(items)
        .build()?;

    // Validate integrity
    let validator = Validator::new(config.validation);
    let report = validator.validate(&graph);

    println!("Errors: {}, Warnings: {}", report.error_count(), report.warning_count());

    // Query traceability
    let options = TraversalOptions::default();
    let item_id = ItemId::new("SYSREQ-001")?;
    if let Some(result) = traverse_upstream(&graph, &item_id, &options) {
        println!("Found {} upstream items", result.items.len());
    }

    Ok(())
}
```

## Document Types

The library supports 18 document types: 10 systems-engineering types forming a requirements hierarchy, and 8 investigation types for investigative knowledge management.

### Systems Engineering Types

| Type | Description |
|------|-------------|
| Solution | Customer-facing solution |
| Use Case | Customer/market need |
| Scenario | Abstract system behavior |
| System Requirement | Quantifiable system-level need |
| System Architecture | Platform implementation |
| Hardware Requirement | Hardware-specific need |
| Software Requirement | Software-specific need |
| HW Detailed Design | Hardware implementation |
| SW Detailed Design | Software implementation |
| ADR | Architecture Decision Record |

### Investigation Types

| Type | Prefix | Description |
|------|--------|-------------|
| Entity | ITM | Entity definition — graph node |
| Evidence | EVD | Cited evidentiary claim with optional structured envelopes (messages, deposition, flights, transactions) |
| Thesis | THS | Investigation container + conclusion |
| Hypothesis | HYP | Competing explanation with independent outcome |
| Analysis | ANL | Evaluates evidence against hypotheses |
| Premise | PRM | Verified predicate for forward chaining |
| Question | QST | Information gap |
| Block | BLK | Access/capability gap |

## Traceability Hierarchy

```
Solution
  -> Use Case
      -> Scenario
          -> System Requirement
              -> System Architecture
                  -> Hardware Requirement
                  |     -> HW Detailed Design
                  -> Software Requirement
                        -> SW Detailed Design
```

## License

Licensed under the Apache-2.0 License. See [LICENSE](https://github.com/cledouarec/sara/blob/main/LICENSE) for details.

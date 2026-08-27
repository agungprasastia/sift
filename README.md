# Sift

**Sift** is a context/token optimizer for coding AI agents. Its long-term goal:

> give a coding AI the minimum relevant context, cutting token usage without
> losing the information that matters.

Sift is implemented in **100% Rust**. Repository scanning, parsing, indexing, querying, and the context pipeline are implemented in Rust. Performance-sensitive paths use optimized Rust libraries and are selected through profiling and benchmarking.

Earlier milestones experimented with a native C subsystem. Benchmarks showed that optimized Rust implementations were equal or faster for current production workloads, so the production architecture was consolidated to pure Rust in M0.6.

```
               Coding Agent
                    │
                    ▼
               Sift Engine
        ┌───────────┼───────────┐
        │           │           │
      Parser      Index       Symbols
        │           │           │
        └───────────┬───────────┘
                    │
                    ▼
                Repository
```

## Architecture & Production Primitives

- **Repository Scanning**: `ignore::WalkBuilder` for standard traversal respecting `.gitignore` and ignored paths.
- **Binary / NUL Detection**: Pure Rust `memchr::memchr(0, bytes)`.
- **Content Hashing**: Pure safe Rust `sift_core::fnv1a_hash` for fast, deterministic 64-bit hashing.
- **Substring / Symbol Search**: Pure Rust substring matching with `memchr::memmem` and standard Rust string operations.
- **Symbol Extraction**: Modular tree-sitter grammars (Rust, C, C++, Go, JS, TS, Python).

## Testing & validation

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

## Roadmap

```text
M0     Foundation                         DONE
M0.5   Native performance experiments     DONE
M0.6   Full Rust consolidation            DONE

M1     Context Ranking
M2     Token Budgeting + Progressive Context
M3     Dependency Graph + Git Awareness
M4     MCP / Coding Agent Integration
M5     Profiling & Performance Optimization
```

# Sift

**Sift** is a context/token optimizer for coding AI agents. Its long-term goal:

> give a coding AI the minimum relevant context, cutting token usage without
> losing the information that matters.

M0 delivers the foundation: a repository scanner, symbol index, and CLI built
on a hybrid **Rust + C11** architecture.

```
               Coding Agent
                    │
                    │ future
                    ▼
                Sift API
                    │
                    ▼
                Rust Core
        ┌───────────┼───────────┐
        │           │           │
      Parser      Index       Symbols
        │           │           │
        └───────────┬───────────┘
                    │
              safe wrapper
                    │
                    ▼
              C11 Accelerator
             scan/hash/search
                    │
                    ▼
                Repository
```

## Architecture rules

> **Rust owns application semantics and long-lived application state.**
> **C owns explicitly-scoped native resources and bounded byte operations.**

- **Rust (core)**: CLI, repository traversal, language detection, data model
  (`SourceFile` / `Symbol` / `SymbolKind`), symbol extraction, index, search,
  stats, safe FFI wrappers, error handling.
- **C11 (`native/`)**: explicitly-scoped native resources and bounded primitives:
  - Arena scratch memory (`SiftArena`)
  - Native dynamic buffer (`SiftBuffer`)
  - Scanner scratch state (`SiftScanner`)
  - Bounded byte-oriented primitives (`sift_find_bytes`, `sift_find_many`, `sift_index_newlines`, `sift_hash_bytes`, `sift_count_byte`)

Cross-language ownership remains explicit and all unsafe Rust remains strictly isolated in `crates/sift-sys`.

### Native Lifecycle Model

- **Arena**: `init` → `alloc` → `reset/reuse` → `destroy`
- **Buffer**: `init` → `reserve/append` → `clear/reuse` → `destroy`
- **Scanner**: `init` → `use` → `reset/reuse` → `destroy`

### Intended Future Native Use

- **`SiftArena`**: Scratch allocation for high-throughput native batch and token operations.
- **`SiftBuffer`**: Reusable native output/scratch buffer for token generation and filtering.
- **`SiftScanner`**: Reusable native scan state when multi-file parallel native scans are profiled.

> The native C layer is hardened and heavily verified, but safety still depends on maintained ownership, lifetime, capacity, and bounds contracts.

### Memory-safety & FFI Contracts

- Rust always owns application memory and high-level semantics.
- Stateless byte operations (`sift_find_bytes`, `sift_hash_bytes`, `sift_count_byte`,
  `sift_find_many`, `sift_index_newlines`) receive borrowed `pointer + length` pairs.
  They store no pointers, retain no references, and allocate nothing.
- Managed native components have explicit ownership lifecycles and return typed `SiftStatus` error codes mapped to Rust `NativeError`.
- Every `unsafe` block is strictly isolated inside `crates/sift-sys` and carries
  a `// SAFETY:` invariant annotation.
- FFI boundary correctness is verified by **10,000+ randomized fuzz iterations** and
  differential testing against pure Rust reference implementations (`tests/differential.rs`,
  `tests/fuzz_primitives.rs`).

## Production Path Decisions Based on Empirical Benchmarks

1. **Binary / NUL Detection**:
   - Production path uses pure Rust `memchr::memchr(0, ...)` over leading bytes.
2. **Content Hashing**:
   - Production path uses pure Rust `sift_core::fnv1a_hash` to eliminate FFI transition overhead while matching C throughput.
3. **Newline Indexing**:
   - Production path uses pure Rust / `memchr` which runs ~2.5x faster than scalar C loops.
4. **Symbol & Substring Search**:
   - Production symbol indexing search (`RepositoryIndex::find`) currently uses Rust `str::contains()`.
   - `memmem::Finder` is benchmarked as the preferred candidate for repeated multi-file search.
   - C `sift_find_bytes` is maintained as a bounded micro-lookup native primitive (~5.5 ns latency on immediate match).

## Testing & validation

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
cargo bench --bench ffi_bench
```

### Compiler Warnings, Static Analysis & Sanitizers Status

- **Compiler Warnings**:
  - MSVC `/W4`: Executed with 0 warnings.
  - GCC/Clang flags (`-Wall -Wextra -Wpedantic -Wconversion -Wsign-conversion -Wshadow -Wformat=2 -Wundef`): Configured in `build.rs`.
- **Static Analysis Tools**:
  - `clang-tidy`, `clang --analyze`, `gcc -fanalyzer`: Not executed in current environment.
- **Sanitizers**:
  - ASan/UBSan support configured in `build.rs` via `SIFT_SANITIZERS=1` (`-fsanitize=address,undefined -fno-omit-frame-pointer`).
  - ASan/UBSan execution: Configured for Linux/macOS nightly toolchains; not executed on the current Windows MSVC host.

## Status

Sift is experimental and currently in early development.
M0/M0.5 are not yet production-ready.

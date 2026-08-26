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

> **Rust owns memory and semantics. C only accelerates bounded byte operations.**

- **Rust (core)**: CLI, repository traversal, language detection, data model
  (`SourceFile` / `Symbol` / `SymbolKind`), symbol extraction, index, search,
  stats, safe FFI wrappers, error handling. ~85% of the code.
- **C11 (`native/`)**: stateless byte primitives only — `sift_find_bytes`,
  `sift_hash_bytes`, `sift_count_byte`. No allocation, no globals, no retained
  pointers, no null-terminated assumptions. Compiled automatically by Cargo
  through `cc` + `build.rs`; no Makefile needed.

Dependency direction is one-way:

```
sift-cli → sift-core / sift-index / sift-parser → sift-sys → C11 native engine
```

Raw C FFI exists **only** inside `crates/sift-sys`. Every other crate consumes
safe functions (`find_bytes`, `hash_bytes`, `count_byte`) that validate every
value returned by C before it reaches Rust code.

### Memory-safety & Lifecycle contract (M0.5 Hardened)

- Rust always owns application memory and high-level semantics.
- Stateless byte operations (`sift_find_bytes`, `sift_hash_bytes`, `sift_count_byte`,
  `sift_find_many`, `sift_index_newlines`) receive borrowed `pointer + length` pairs.
  They store no pointers, retain no references, and allocate nothing.
- Managed native components have explicit ownership lifecycles:
  - **`SiftArena`** (`native/src/arena.c`): Fixed-capacity bump allocator. Owns
    its backing buffer via `sift_arena_init` and `sift_arena_destroy`.
    All returned pointers are borrowed and strictly bounded to the arena's lifetime.
  - **`SiftBuffer`** (`native/src/buffer.c`): Dynamically growing byte buffer with
    strict arithmetic overflow guards and safe temporary pointers on `realloc`.
  - **`SiftScanner`** (`native/src/scanner_state.c`): Native scanner state composed
    over a scratch arena. Never retains pointers borrowed from Rust.
- Every `unsafe` block is strictly isolated inside `crates/sift-sys` and carries
  a `// SAFETY:` invariant annotation.
- FFI boundary correctness is verified by **10,000+ randomized fuzz iterations** and
  differential testing against pure Rust reference implementations (`tests/differential.rs`,
  `tests/fuzz_primitives.rs`).

## Production Path Decisions Based on Empirical Benchmarks

1. **`find_bytes` / substring search**:
   - For small/early-hit needles, C primitive has minimal setup overhead.
   - For general absent needle scanning on large buffers, `memchr::memmem` SIMD routines
     achieve high throughput (>20 GiB/s).
2. **`find_many` (Batch Search)**:
   - Evaluated for multi-needle passes in a single FFI call.
3. **`count_byte`**:
   - LLVM auto-vectorization and `memchr` outperform naive scalar iteration. Rust iterator
     or `memchr` is designated for production hot paths.
4. **`hash_bytes` (FNV-1a 64-bit)**:
   - Parity between C and Rust (~550 µs on 260 KB). Rust implementation preferred in core
     where FFI overhead is unnecessary.

> Native C layer is hardened and heavily verified, but C remains an unsafe language and
> correctness still depends on maintained ownership and bounds contracts.

## Testing & validation

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

Optional sanitizer audit on Linux/macOS debug builds (requires nightly rustc linker support):

```bash
SIFT_SANITIZERS=1 RUSTFLAGS="-Zsanitizer=address" cargo test
```

## Status

Sift is experimental and currently in early development.
M0/M0.5 are not yet production-ready.

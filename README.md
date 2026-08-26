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

### Memory-safety contract

- Rust always owns memory; C receives borrowed `pointer + length` pairs.
- C never allocates (`malloc`/`free` are absent), stores no pointers past the
  call, keeps no global mutable state, and treats source as raw bytes.
- "Not found" is `SIZE_MAX`; Rust re-validates offsets against input bounds.
- Every `unsafe` block in `sift-sys` carries a `SAFETY:` comment naming the
  invariant that makes it sound; there is no `unsafe` anywhere else.
- Correctness of the FFI boundary is proven by **differential testing**: each
  primitive has a plain-Rust reference implementation, and fixed edge cases
  plus deterministic pseudo-random buffers must produce identical results
  from both sides (`crates/sift-sys/tests/differential.rs`). Real compiled C
  cannot run under Miri, so differential tests + the strict C contract are
  the designated proof strategy for M0.

## Usage

```bash
cargo run -p sift-cli -- map .
cargo run -p sift-cli -- find cleanup_session .
cargo run -p sift-cli -- stats .
# or via the alias:
cargo sift map .
```

`sift map` prints a symbol tree:

```
src/
├── auth.rs
│   ├── struct User
│   ├── fn login
│   └── fn authenticate
└── session.rs
    ├── struct Session
    └── fn cleanup_session
```

`sift stats` reports scan totals, per-language counts, and the native backend:

```
Files scanned: 184
Source files: 92
Symbols: 1,438

Languages:
Rust        61
TypeScript  18
Go          8

Native engine: enabled
Backend: C11
```

## Languages

Rust, C, C++, Go, JavaScript, TypeScript, Python — extracted with
[tree-sitter](https://tree-sitter.github.io/) grammars, one module per language
under `crates/sift-parser/src/lang/`. Adding a language = one new module, one
match arm, one enum variant. Known M0 limitations: TypeScript `type` aliases
are skipped (no alias kind yet) and `.tsx` routes through the plain TS grammar.

## Scanner

- Honors `.gitignore` (even outside git repositories) via the `ignore` crate.
- Always prunes: `.git`, `target`, `node_modules`, `dist`, `build`, `.next`,
  `coverage`, `vendor`, `.cache`.
- Skips binaries (NUL sniff over the leading 8 KB through the C accelerator),
  files > 1 MiB, and non-UTF-8 content. Processes one file at a time — the
  whole repository is never loaded into RAM.
- Uses the native engine on real execution paths: `count_byte` for binary
  detection and `hash_bytes` for per-file content hashes.

## Testing & validation

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

Coverage includes language detection, scanning, `.gitignore` handling,
ignored directories, extraction, search, stats, FFI correctness, empty/binary
inputs, hash consistency, and byte-search edge cases — all against a
deterministic fixture repository.

Optional sanitizer audit on Linux/macOS debug builds (requires nightly rustc
linker support):

```bash
SIFT_SANITIZERS=1 RUSTFLAGS="-Zsanitizer=address" cargo test
```

## Benchmarks

```bash
cargo bench            # runs ffi_bench, scanner_bench, parser_bench
```

The FFI benches compare every C primitive against its Rust baseline over a
realistic payload. Do not assume C wins: measure. If a primitive shows no
real advantage, the architecture does not depend on it — swap the wrapper's
body for the Rust implementation behind the same API.

## Attention
This is not a serious development project.

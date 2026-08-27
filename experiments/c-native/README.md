# Historical Experiment: C Native Subsystem (M0.5)

This directory contains the historical C native accelerator and FFI integration (`sift-sys`) developed and evaluated during Milestone M0.5.

## Architecture Decision Note

- **Status**: Historical Experiment / Archived
- **Rationale**: Benchmark evaluations in M0.5 showed that modern safe Rust implementations (such as `memchr`, `memmem`, and pure Rust FNV-1a hashing) match or exceed the performance of the native C primitives on production workloads without FFI transition overhead, cross-language build complexities, or unsafe surface area.
- **Production Status**: Sift production is **100% pure Rust**. The contents of this directory are not workspace members, are not compiled or tested during normal `cargo build` / `cargo test`, and are not used by any Sift production binaries or libraries.

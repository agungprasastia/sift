//! Compiles the C11 accelerator under `native/` through the `cc` crate so a
//! plain `cargo build` works on Windows (MSVC), Linux (GCC/Clang) and macOS
//! (Clang). No Makefile required.

use std::env;

const GCC_CLANG_WARNINGS: &[&str] = &[
    "-Wall",
    "-Wextra",
    "-Wpedantic",
    "-Wconversion",
    "-Wsign-conversion",
    "-Wshadow",
    "-Wformat=2",
    "-Wundef",
];

fn main() {
    println!("cargo:rerun-if-changed=../../native");

    let mut build = cc::Build::new();
    build
        .std("c11")
        .include("../../native/include")
        .file("../../native/src/search.c")
        .file("../../native/src/hash.c")
        .file("../../native/src/scan.c")
        .file("../../native/src/arena.c")
        .file("../../native/src/buffer.c")
        .file("../../native/src/scanner_state.c");

    if build.get_compiler().is_like_msvc() {
        // MSVC-native warnings only; GCC-specific flags would break this build.
        build.flag_if_supported("/W4");
    } else {
        for flag in GCC_CLANG_WARNINGS {
            build.flag_if_supported(flag);
        }
        maybe_enable_sanitizers(&mut build);
    }

    build.compile("sift_native");
}

/// Opt-in sanitizer instrumentation for local audits on Linux/macOS debug
/// builds: `SIFT_SANITIZERS=1 cargo test`.
///
/// Note: instrumentation also needs matching link-time support (e.g. nightly
/// `RUSTFLAGS="-Zsanitizer=address"`); normal release builds never enable it.
fn maybe_enable_sanitizers(build: &mut cc::Build) {
    let requested = env::var_os("SIFT_SANITIZERS").is_some();
    let debug = env::var("PROFILE").as_deref() == Ok("debug");
    if requested && debug {
        build.flag("-fsanitize=address,undefined");
        build.flag("-fno-omit-frame-pointer");
    }
}

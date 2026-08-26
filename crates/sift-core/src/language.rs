//! Source-language identification by file extension.

use std::fmt;
use std::path::Path;

/// Languages supported by Sift M0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Language {
    Rust,
    C,
    Cpp,
    Go,
    JavaScript,
    TypeScript,
    Python,
}

impl Language {
    /// Language for a file-extension string (case-insensitive).
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "rs" => Some(Self::Rust),
            "c" | "h" => Some(Self::C),
            "cc" | "cpp" | "cxx" | "c++" | "hpp" | "hh" | "hxx" => Some(Self::Cpp),
            "go" => Some(Self::Go),
            "js" | "mjs" | "cjs" | "jsx" => Some(Self::JavaScript),
            "ts" | "tsx" | "mts" | "cts" => Some(Self::TypeScript),
            "py" | "pyi" => Some(Self::Python),
            _ => None,
        }
    }

    /// Language detected from a path's extension.
    #[must_use]
    pub fn detect(path: &Path) -> Option<Self> {
        Self::from_extension(path.extension()?.to_str()?)
    }

    /// Human-readable name used in CLI output.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::C => "C",
            Self::Cpp => "C++",
            Self::Go => "Go",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Python => "Python",
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn path_with(ext: &str) -> PathBuf {
        PathBuf::from(format!("some/file.{ext}"))
    }

    #[test]
    fn detects_supported_extensions() {
        let cases = [
            ("rs", Language::Rust),
            ("c", Language::C),
            ("h", Language::C),
            ("cpp", Language::Cpp),
            ("hpp", Language::Cpp),
            ("cc", Language::Cpp),
            ("go", Language::Go),
            ("js", Language::JavaScript),
            ("jsx", Language::JavaScript),
            ("mjs", Language::JavaScript),
            ("ts", Language::TypeScript),
            ("tsx", Language::TypeScript),
            ("mts", Language::TypeScript),
            ("py", Language::Python),
            ("pyi", Language::Python),
        ];
        for (ext, expected) in cases {
            assert_eq!(Language::from_extension(ext), Some(expected), "ext {ext}");
            assert_eq!(Language::detect(&path_with(ext)), Some(expected));
        }
    }

    #[test]
    fn detection_is_case_insensitive() {
        assert_eq!(Language::from_extension("RS"), Some(Language::Rust));
        assert_eq!(Language::detect(&path_with("GO")), Some(Language::Go));
    }

    #[test]
    fn unknown_or_missing_extension_yields_none() {
        assert_eq!(Language::from_extension("md"), None);
        assert_eq!(Language::from_extension("bin"), None);
        assert_eq!(Language::detect(Path::new("noext")), None);
        assert_eq!(Language::detect(Path::new("dir.d/")), None);
    }

    #[test]
    fn names_render_for_display() {
        assert_eq!(Language::Rust.name(), "Rust");
        assert_eq!(Language::Cpp.name(), "C++");
        assert_eq!(Language::TypeScript.to_string(), "TypeScript");
    }
}

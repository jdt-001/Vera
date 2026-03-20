//! Shared types used across Vera's core modules.

use serde::{Deserialize, Serialize};

/// Filters that can be applied to search results.
///
/// All filters are optional. When set, they restrict results to only those
/// matching all specified criteria (AND semantics).
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    /// Filter by programming language (case-insensitive match).
    pub language: Option<String>,
    /// Filter by file path glob pattern (e.g., `src/**/*.rs`).
    pub path_glob: Option<String>,
    /// Filter by symbol type (case-insensitive match).
    pub symbol_type: Option<String>,
}

impl SearchFilters {
    /// Returns true if no filters are set.
    pub fn is_empty(&self) -> bool {
        self.language.is_none() && self.path_glob.is_none() && self.symbol_type.is_none()
    }

    /// Check whether a search result matches all active filters.
    pub fn matches(&self, result: &SearchResult) -> bool {
        // Language filter (case-insensitive).
        if let Some(ref lang) = self.language {
            if !result.language.to_string().eq_ignore_ascii_case(lang) {
                return false;
            }
        }

        // Path glob filter.
        if let Some(ref pattern) = self.path_glob {
            if !glob_matches(pattern, &result.file_path) {
                return false;
            }
        }

        // Symbol type filter (case-insensitive).
        if let Some(ref stype) = self.symbol_type {
            match &result.symbol_type {
                Some(st) => {
                    if !st.to_string().eq_ignore_ascii_case(stype) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        true
    }
}

/// Simple glob matching supporting `*` (any segment) and `**` (any path).
///
/// Supports common patterns: `*.rs`, `src/**/*.ts`, `**/test_*`.
/// Does not support character classes or brace expansion.
fn glob_matches(pattern: &str, path: &str) -> bool {
    // Normalize separators.
    let pattern = pattern.replace('\\', "/");
    let path = path.replace('\\', "/");

    glob_match_recursive(&pattern, &path)
}

/// Recursive glob matching helper.
fn glob_match_recursive(pattern: &str, text: &str) -> bool {
    // Handle `**` patterns (match any path segments).
    if let Some(rest) = pattern.strip_prefix("**/") {
        // `**/X` matches X at any depth.
        if glob_match_recursive(rest, text) {
            return true;
        }
        // Try skipping path segments.
        for (i, _) in text.char_indices() {
            if text.as_bytes().get(i) == Some(&b'/') && glob_match_recursive(rest, &text[i + 1..]) {
                return true;
            }
        }
        return false;
    }

    if pattern.is_empty() && text.is_empty() {
        return true;
    }
    if pattern.is_empty() {
        return false;
    }

    // Handle `*` within a segment (matches anything except `/`).
    if let Some(rest) = pattern.strip_prefix('*') {
        // Try matching * against 0..n characters (not crossing `/`).
        if glob_match_recursive(rest, text) {
            return true;
        }
        for (i, ch) in text.char_indices() {
            if ch == '/' {
                break;
            }
            if glob_match_recursive(rest, &text[i + 1..]) {
                return true;
            }
        }
        return false;
    }

    // Match literal characters.
    let mut p_chars = pattern.chars();
    let mut t_chars = text.chars();
    if let (Some(pc), Some(tc)) = (p_chars.next(), t_chars.next()) {
        if pc == tc {
            return glob_match_recursive(p_chars.as_str(), t_chars.as_str());
        }
    }

    false
}

/// A chunk of source code extracted from a parsed file.
///
/// This is the fundamental unit that gets indexed, embedded, and retrieved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Unique identifier for this chunk.
    pub id: String,
    /// Repository-relative file path.
    pub file_path: String,
    /// 1-based start line in the source file.
    pub line_start: u32,
    /// 1-based end line in the source file (inclusive).
    pub line_end: u32,
    /// The actual source code content of this chunk.
    pub content: String,
    /// Detected programming language.
    pub language: Language,
    /// Type of symbol this chunk represents (if any).
    pub symbol_type: Option<SymbolType>,
    /// Name of the symbol (if applicable).
    pub symbol_name: Option<String>,
}

/// Programming language of a source file or chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    C,
    Cpp,
    Ruby,
    Swift,
    Kotlin,
    Scala,
    Zig,
    Lua,
    Bash,
    /// Structural / config formats (Tier 1B).
    Toml,
    Yaml,
    Json,
    Markdown,
    /// Fallback for unrecognized file types (Tier 0).
    Unknown,
}

impl Language {
    /// Detect language from a file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "ts" | "tsx" => Self::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Self::JavaScript,
            "py" | "pyi" => Self::Python,
            "go" => Self::Go,
            "java" => Self::Java,
            "c" | "h" => Self::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => Self::Cpp,
            "rb" => Self::Ruby,
            "swift" => Self::Swift,
            "kt" | "kts" => Self::Kotlin,
            "scala" | "sc" => Self::Scala,
            "zig" => Self::Zig,
            "lua" => Self::Lua,
            "sh" | "bash" | "zsh" => Self::Bash,
            "toml" => Self::Toml,
            "yaml" | "yml" => Self::Yaml,
            "json" => Self::Json,
            "md" | "markdown" => Self::Markdown,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Python => "python",
            Self::Go => "go",
            Self::Java => "java",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Ruby => "ruby",
            Self::Swift => "swift",
            Self::Kotlin => "kotlin",
            Self::Scala => "scala",
            Self::Zig => "zig",
            Self::Lua => "lua",
            Self::Bash => "bash",
            Self::Toml => "toml",
            Self::Yaml => "yaml",
            Self::Json => "json",
            Self::Markdown => "markdown",
            Self::Unknown => "unknown",
        };
        write!(f, "{name}")
    }
}

/// Type of symbol extracted from source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolType {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Trait,
    Interface,
    TypeAlias,
    Constant,
    Variable,
    Module,
    /// A fallback chunk not aligned to a specific symbol.
    Block,
}

impl std::fmt::Display for SymbolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Interface => "interface",
            Self::TypeAlias => "type_alias",
            Self::Constant => "constant",
            Self::Variable => "variable",
            Self::Module => "module",
            Self::Block => "block",
        };
        write!(f, "{name}")
    }
}

/// A search result returned by the retrieval pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Repository-relative file path.
    pub file_path: String,
    /// 1-based start line.
    pub line_start: u32,
    /// 1-based end line (inclusive).
    pub line_end: u32,
    /// The code content of this result.
    pub content: String,
    /// Programming language.
    pub language: Language,
    /// Relevance score (higher is better).
    pub score: f64,
    /// Symbol name, if the result corresponds to a named symbol.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_name: Option<String>,
    /// Symbol type, if the result corresponds to a typed symbol.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_type: Option<SymbolType>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_from_extension_rust() {
        assert_eq!(Language::from_extension("rs"), Language::Rust);
    }

    #[test]
    fn language_from_extension_typescript() {
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
        assert_eq!(Language::from_extension("tsx"), Language::TypeScript);
    }

    #[test]
    fn language_from_extension_python() {
        assert_eq!(Language::from_extension("py"), Language::Python);
        assert_eq!(Language::from_extension("pyi"), Language::Python);
    }

    #[test]
    fn language_from_extension_unknown() {
        assert_eq!(Language::from_extension("xyz"), Language::Unknown);
    }

    #[test]
    fn language_from_extension_case_insensitive() {
        assert_eq!(Language::from_extension("RS"), Language::Rust);
        assert_eq!(Language::from_extension("Py"), Language::Python);
    }

    #[test]
    fn language_display() {
        assert_eq!(Language::Rust.to_string(), "rust");
        assert_eq!(Language::TypeScript.to_string(), "typescript");
        assert_eq!(Language::Unknown.to_string(), "unknown");
    }

    #[test]
    fn symbol_type_display() {
        assert_eq!(SymbolType::Function.to_string(), "function");
        assert_eq!(SymbolType::Class.to_string(), "class");
        assert_eq!(SymbolType::Block.to_string(), "block");
    }

    #[test]
    fn chunk_serialization_round_trip() {
        let chunk = Chunk {
            id: "test-1".to_string(),
            file_path: "src/main.rs".to_string(),
            line_start: 1,
            line_end: 10,
            content: "fn main() {}".to_string(),
            language: Language::Rust,
            symbol_type: Some(SymbolType::Function),
            symbol_name: Some("main".to_string()),
        };
        let json = serde_json::to_string(&chunk).unwrap();
        let deserialized: Chunk = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test-1");
        assert_eq!(deserialized.file_path, "src/main.rs");
        assert_eq!(deserialized.language, Language::Rust);
        assert_eq!(deserialized.symbol_name, Some("main".to_string()));
    }

    #[test]
    fn search_result_serialization_omits_none() {
        let result = SearchResult {
            file_path: "lib.rs".to_string(),
            line_start: 5,
            line_end: 20,
            content: "pub fn example() {}".to_string(),
            language: Language::Rust,
            score: 0.95,
            symbol_name: None,
            symbol_type: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.contains("symbol_name"));
        assert!(!json.contains("symbol_type"));
    }

    // ── SearchFilters tests ─────────────────────────────────────

    fn make_test_result(
        file: &str,
        lang: Language,
        sym_name: Option<&str>,
        sym_type: Option<SymbolType>,
    ) -> SearchResult {
        SearchResult {
            file_path: file.to_string(),
            line_start: 1,
            line_end: 10,
            content: "test content".to_string(),
            language: lang,
            score: 1.0,
            symbol_name: sym_name.map(|s| s.to_string()),
            symbol_type: sym_type,
        }
    }

    #[test]
    fn filters_empty_matches_everything() {
        let filters = SearchFilters::default();
        assert!(filters.is_empty());
        let result = make_test_result("src/main.rs", Language::Rust, None, None);
        assert!(filters.matches(&result));
    }

    #[test]
    fn filter_by_language() {
        let filters = SearchFilters {
            language: Some("rust".to_string()),
            ..Default::default()
        };
        let rust_result = make_test_result("a.rs", Language::Rust, None, None);
        let py_result = make_test_result("a.py", Language::Python, None, None);
        assert!(filters.matches(&rust_result));
        assert!(!filters.matches(&py_result));
    }

    #[test]
    fn filter_by_language_case_insensitive() {
        let filters = SearchFilters {
            language: Some("Rust".to_string()),
            ..Default::default()
        };
        let result = make_test_result("a.rs", Language::Rust, None, None);
        assert!(filters.matches(&result));
    }

    #[test]
    fn filter_by_symbol_type() {
        let filters = SearchFilters {
            symbol_type: Some("function".to_string()),
            ..Default::default()
        };
        let func = make_test_result(
            "a.rs",
            Language::Rust,
            Some("foo"),
            Some(SymbolType::Function),
        );
        let cls = make_test_result(
            "a.py",
            Language::Python,
            Some("Bar"),
            Some(SymbolType::Class),
        );
        let none_sym = make_test_result("a.rs", Language::Rust, None, None);
        assert!(filters.matches(&func));
        assert!(!filters.matches(&cls));
        assert!(!filters.matches(&none_sym));
    }

    #[test]
    fn filter_by_symbol_type_case_insensitive() {
        let filters = SearchFilters {
            symbol_type: Some("Function".to_string()),
            ..Default::default()
        };
        let func = make_test_result(
            "a.rs",
            Language::Rust,
            Some("foo"),
            Some(SymbolType::Function),
        );
        assert!(filters.matches(&func));
    }

    #[test]
    fn filter_by_path_glob_extension() {
        let filters = SearchFilters {
            path_glob: Some("*.rs".to_string()),
            ..Default::default()
        };
        let rs = make_test_result("main.rs", Language::Rust, None, None);
        let py = make_test_result("main.py", Language::Python, None, None);
        assert!(filters.matches(&rs));
        assert!(!filters.matches(&py));
    }

    #[test]
    fn filter_by_path_glob_directory() {
        let filters = SearchFilters {
            path_glob: Some("src/**/*.rs".to_string()),
            ..Default::default()
        };
        let in_src = make_test_result("src/lib.rs", Language::Rust, None, None);
        let deep = make_test_result("src/a/b/c.rs", Language::Rust, None, None);
        let outside = make_test_result("tests/test.rs", Language::Rust, None, None);
        assert!(filters.matches(&in_src));
        assert!(filters.matches(&deep));
        assert!(!filters.matches(&outside));
    }

    #[test]
    fn filter_by_path_glob_doublestar_prefix() {
        let filters = SearchFilters {
            path_glob: Some("**/test_*.py".to_string()),
            ..Default::default()
        };
        let deep = make_test_result("tests/unit/test_auth.py", Language::Python, None, None);
        let top = make_test_result("test_main.py", Language::Python, None, None);
        let no_match = make_test_result("src/auth.py", Language::Python, None, None);
        assert!(filters.matches(&deep));
        assert!(filters.matches(&top));
        assert!(!filters.matches(&no_match));
    }

    #[test]
    fn filter_combined_lang_and_type() {
        let filters = SearchFilters {
            language: Some("rust".to_string()),
            symbol_type: Some("struct".to_string()),
            ..Default::default()
        };
        let rust_struct = make_test_result(
            "a.rs",
            Language::Rust,
            Some("Foo"),
            Some(SymbolType::Struct),
        );
        let rust_func = make_test_result(
            "b.rs",
            Language::Rust,
            Some("bar"),
            Some(SymbolType::Function),
        );
        let py_class = make_test_result(
            "c.py",
            Language::Python,
            Some("Baz"),
            Some(SymbolType::Class),
        );
        assert!(filters.matches(&rust_struct));
        assert!(!filters.matches(&rust_func));
        assert!(!filters.matches(&py_class));
    }

    // ── glob_matches tests ──────────────────────────────────────

    #[test]
    fn glob_star_matches_extension() {
        assert!(glob_matches("*.rs", "main.rs"));
        assert!(!glob_matches("*.rs", "main.py"));
    }

    #[test]
    fn glob_star_does_not_cross_slash() {
        assert!(!glob_matches("*.rs", "src/main.rs"));
    }

    #[test]
    fn glob_doublestar_matches_any_depth() {
        assert!(glob_matches("**/*.rs", "main.rs"));
        assert!(glob_matches("**/*.rs", "src/main.rs"));
        assert!(glob_matches("**/*.rs", "src/a/b/main.rs"));
    }

    #[test]
    fn glob_literal_prefix() {
        assert!(glob_matches("src/*.rs", "src/lib.rs"));
        assert!(!glob_matches("src/*.rs", "tests/lib.rs"));
    }

    #[test]
    fn glob_exact_match() {
        assert!(glob_matches("src/main.rs", "src/main.rs"));
        assert!(!glob_matches("src/main.rs", "src/lib.rs"));
    }

    #[test]
    fn glob_empty_pattern_matches_empty() {
        assert!(glob_matches("", ""));
        assert!(!glob_matches("", "something"));
    }
}

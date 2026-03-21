//! Tree-sitter grammar loading for supported languages.
//!
//! Maps [`Language`] variants to tree-sitter grammar definitions.
//! Tier 1A languages get full AST-based parsing; others fall back to Tier 0.

use tree_sitter::Language as TsLanguage;

use crate::types::Language;

/// Returns the tree-sitter grammar for a given language, if supported.
///
/// Returns `None` for languages without tree-sitter support (Tier 0 fallback).
pub fn tree_sitter_grammar(lang: Language) -> Option<TsLanguage> {
    let lang_fn = match lang {
        Language::Rust => tree_sitter_rust::LANGUAGE,
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
        Language::JavaScript => tree_sitter_javascript::LANGUAGE,
        Language::Python => tree_sitter_python::LANGUAGE,
        Language::Go => tree_sitter_go::LANGUAGE,
        Language::Java => tree_sitter_java::LANGUAGE,
        Language::C => tree_sitter_c::LANGUAGE,
        Language::Cpp => tree_sitter_cpp::LANGUAGE,
        Language::Ruby => tree_sitter_ruby::LANGUAGE,
        Language::Bash => tree_sitter_bash::LANGUAGE,
        Language::Kotlin => tree_sitter_kotlin_sg::LANGUAGE,
        Language::Swift => tree_sitter_swift::LANGUAGE,
        Language::Zig => tree_sitter_zig::LANGUAGE,
        Language::Lua => tree_sitter_lua::LANGUAGE,
        Language::Scala => tree_sitter_scala::LANGUAGE,
        Language::CSharp => tree_sitter_c_sharp::LANGUAGE,
        Language::Php => tree_sitter_php::LANGUAGE_PHP,
        Language::Haskell => tree_sitter_haskell::LANGUAGE,
        Language::Elixir => tree_sitter_elixir::LANGUAGE,
        Language::Dart => tree_sitter_dart::LANGUAGE,
        // Languages without tree-sitter grammar support → Tier 0 fallback
        Language::Toml
        | Language::Yaml
        | Language::Json
        | Language::Markdown
        | Language::Unknown => return None,
    };
    Some(lang_fn.into())
}

/// Returns whether a language has tree-sitter grammar support (Tier 1A).
pub fn has_grammar(lang: Language) -> bool {
    tree_sitter_grammar(lang).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_1a_languages_have_grammars() {
        let tier_1a = [
            Language::Rust,
            Language::TypeScript,
            Language::JavaScript,
            Language::Python,
            Language::Go,
            Language::Java,
            Language::C,
            Language::Cpp,
            Language::Ruby,
            Language::Bash,
            Language::Kotlin,
            Language::Swift,
            Language::Zig,
            Language::Lua,
            Language::Scala,
            Language::CSharp,
            Language::Php,
            Language::Haskell,
            Language::Elixir,
            Language::Dart,
        ];
        for lang in tier_1a {
            assert!(
                has_grammar(lang),
                "{lang} should have a tree-sitter grammar"
            );
        }
    }

    #[test]
    fn tier_0_languages_have_no_grammar() {
        let tier_0 = [
            Language::Unknown,
            Language::Toml,
            Language::Yaml,
            Language::Json,
            Language::Markdown,
        ];
        for lang in tier_0 {
            assert!(
                !has_grammar(lang),
                "{lang} should NOT have a tree-sitter grammar"
            );
        }
    }

    #[test]
    fn grammar_creates_valid_parser() {
        let grammar = tree_sitter_grammar(Language::Rust).unwrap();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&grammar).expect("grammar should load");
    }
}

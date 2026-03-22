//! Tree-sitter grammar loading for supported languages.
//!
//! Maps [`Language`] variants to tree-sitter grammar definitions.
//! Tier 1A languages get full AST-based parsing; others fall back to Tier 0.

use tree_sitter::Language as TsLanguage;

use crate::types::Language;

extern crate tree_sitter_hcl;

unsafe extern "C" {
    fn tree_sitter_sql() -> *const ();
    fn tree_sitter_hcl() -> *const ();
    fn tree_sitter_proto() -> *const ();
    fn tree_sitter_vue() -> *const ();
    fn tree_sitter_dockerfile() -> *const ();
}

/// Returns the tree-sitter grammar for a given language, if supported.
///
/// Returns `None` for languages without tree-sitter support (Tier 0 fallback).
pub fn tree_sitter_grammar(lang: Language) -> Option<TsLanguage> {
    let lang_fn = match lang {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::Java => tree_sitter_java::LANGUAGE.into(),
        Language::C => tree_sitter_c::LANGUAGE.into(),
        Language::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        Language::Ruby => tree_sitter_ruby::LANGUAGE.into(),
        Language::Bash => tree_sitter_bash::LANGUAGE.into(),
        Language::Kotlin => tree_sitter_kotlin_sg::LANGUAGE.into(),
        Language::Swift => tree_sitter_swift::LANGUAGE.into(),
        Language::Zig => tree_sitter_zig::LANGUAGE.into(),
        Language::Lua => tree_sitter_lua::LANGUAGE.into(),
        Language::Scala => tree_sitter_scala::LANGUAGE.into(),
        Language::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
        Language::Php => tree_sitter_php::LANGUAGE_PHP.into(),
        Language::Haskell => tree_sitter_haskell::LANGUAGE.into(),
        Language::Elixir => tree_sitter_elixir::LANGUAGE.into(),
        Language::Dart => tree_sitter_dart::LANGUAGE.into(),
        Language::Sql => unsafe { std::mem::transmute::<*const (), TsLanguage>(tree_sitter_sql()) },
        Language::Hcl => unsafe { std::mem::transmute::<*const (), TsLanguage>(tree_sitter_hcl()) },
        Language::Protobuf => unsafe {
            std::mem::transmute::<*const (), TsLanguage>(tree_sitter_proto())
        },
        Language::Html => tree_sitter_html::LANGUAGE.into(),
        Language::Css => tree_sitter_css::LANGUAGE.into(),
        Language::Scss => tree_sitter_scss::language(),
        Language::Vue => unsafe { std::mem::transmute::<*const (), TsLanguage>(tree_sitter_vue()) },
        Language::GraphQl => tree_sitter_graphql::LANGUAGE.into(),
        Language::CMake => tree_sitter_cmake::LANGUAGE.into(),
        Language::Dockerfile => unsafe {
            std::mem::transmute::<*const (), TsLanguage>(tree_sitter_dockerfile())
        },
        Language::Xml => tree_sitter_xml::LANGUAGE_XML.into(),
        // Languages without tree-sitter grammar support → Tier 0 fallback
        Language::Toml
        | Language::Yaml
        | Language::Json
        | Language::Markdown
        | Language::Unknown => return None,
    };
    Some(lang_fn)
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
            Language::Sql,
            Language::Hcl,
            Language::Protobuf,
        ];
        for lang in tier_1a {
            assert!(
                has_grammar(lang),
                "{lang} should have a tree-sitter grammar"
            );
        }
    }

    #[test]
    fn tier_1b_languages_have_grammars() {
        let tier_1b = [
            Language::Html,
            Language::Css,
            Language::Scss,
            Language::Vue,
            Language::GraphQl,
            Language::CMake,
            Language::Dockerfile,
            Language::Xml,
        ];
        for lang in tier_1b {
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

    #[test]
    fn html_grammar_creates_valid_parser() {
        let grammar = tree_sitter_grammar(Language::Html).unwrap();
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&grammar)
            .expect("HTML grammar should load");
        let tree = parser.parse("<div></div>", None).unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn css_grammar_creates_valid_parser() {
        let grammar = tree_sitter_grammar(Language::Css).unwrap();
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&grammar)
            .expect("CSS grammar should load");
        let tree = parser.parse("body { color: red; }", None).unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn scss_grammar_creates_valid_parser() {
        let grammar = tree_sitter_grammar(Language::Scss).unwrap();
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&grammar)
            .expect("SCSS grammar should load");
        let tree = parser
            .parse("$color: red; body { color: $color; }", None)
            .unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn vue_grammar_creates_valid_parser() {
        let grammar = tree_sitter_grammar(Language::Vue).unwrap();
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&grammar)
            .expect("Vue grammar should load");
        let tree = parser
            .parse("<template><div></div></template>", None)
            .unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn graphql_grammar_creates_valid_parser() {
        let grammar = tree_sitter_grammar(Language::GraphQl).unwrap();
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&grammar)
            .expect("GraphQL grammar should load");
        let tree = parser.parse("type Query { hello: String }", None).unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn cmake_grammar_creates_valid_parser() {
        let grammar = tree_sitter_grammar(Language::CMake).unwrap();
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&grammar)
            .expect("CMake grammar should load");
        let tree = parser
            .parse("cmake_minimum_required(VERSION 3.10)", None)
            .unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn dockerfile_grammar_creates_valid_parser() {
        let grammar = tree_sitter_grammar(Language::Dockerfile).unwrap();
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&grammar)
            .expect("Dockerfile grammar should load");
        let tree = parser.parse("FROM ubuntu:20.04\n", None).unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn xml_grammar_creates_valid_parser() {
        let grammar = tree_sitter_grammar(Language::Xml).unwrap();
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&grammar)
            .expect("XML grammar should load");
        let tree = parser
            .parse("<?xml version=\"1.0\"?><root><item/></root>", None)
            .unwrap();
        assert!(!tree.root_node().has_error());
    }
}

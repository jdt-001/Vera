//! Source code parsing using tree-sitter.
//!
//! This module is responsible for:
//! - Loading tree-sitter grammars for supported languages
//! - Parsing source files into ASTs
//! - Extracting symbol-level chunks (functions, classes, structs, etc.)
//! - Tier 0 fallback chunking for unsupported languages
//!
//! # Architecture
//!
//! - [`languages`] — Grammar loading and language detection
//! - [`extractor`] — AST node extraction rules per language
//! - [`chunker`] — Symbol-to-chunk conversion and large symbol splitting

pub mod chunker;
pub mod extractor;
pub mod languages;
pub mod references;
pub mod signatures;
pub mod sphinx;

use anyhow::{Context, Result};
use tree_sitter::Parser;

use crate::config::IndexingConfig;
use crate::types::{Chunk, Language};

/// Parse a source file and produce chunks.
///
/// For Tier 1A languages (with tree-sitter support), uses AST-based
/// symbol-aware chunking. For other languages, falls back to Tier 0
/// line-based sliding-window chunking.
///
/// # Arguments
/// - `source` — Full text content of the file
/// - `file_path` — Repository-relative path (used in chunk IDs and metadata)
/// - `language` — Detected programming language
/// - `config` — Indexing configuration (max chunk lines, etc.)
///
/// # Errors
/// Returns an error if tree-sitter parsing fails for a supported language.
pub fn parse_and_chunk(
    source: &str,
    file_path: &str,
    language: Language,
    config: &IndexingConfig,
) -> Result<Vec<Chunk>> {
    let chunks = if language == Language::Markdown {
        chunker::markdown_section_chunks(source, file_path)
    } else if language == Language::Rst {
        parse_rst_section_chunks(source, file_path)?
    } else if language.prefers_file_chunking() {
        chunker::whole_file_chunk(source, file_path, language)
    } else {
        match languages::tree_sitter_grammar(language) {
            Some(grammar) => parse_with_treesitter(source, file_path, language, grammar, config)?,
            None => chunker::tier0_line_chunks(source, file_path, language),
        }
    };

    Ok(chunker::split_oversized_chunks(
        chunks,
        config.max_chunk_bytes,
    ))
}

fn parse_rst_section_chunks(source: &str, file_path: &str) -> Result<Vec<Chunk>> {
    let grammar = languages::tree_sitter_grammar(Language::Rst)
        .context("missing tree-sitter grammar for reStructuredText")?;

    let mut parser = Parser::new();
    parser
        .set_language(&grammar)
        .context("failed to load reStructuredText grammar")?;

    let tree = parser
        .parse(source, None)
        .context("tree-sitter parsing returned None")?;

    let headings = extractor::extract_rst_section_titles(&tree, source.as_bytes());
    if headings.is_empty() {
        return Ok(chunker::tier0_line_chunks(source, file_path, Language::Rst));
    }

    Ok(chunker::rst_section_chunks(source, file_path, &headings))
}

/// Parse source using tree-sitter and produce symbol-aware chunks.
fn parse_with_treesitter(
    source: &str,
    file_path: &str,
    language: Language,
    grammar: tree_sitter::Language,
    config: &IndexingConfig,
) -> Result<Vec<Chunk>> {
    let mut parser = Parser::new();
    parser
        .set_language(&grammar)
        .context("failed to load tree-sitter grammar")?;

    let tree = parser
        .parse(source, None)
        .context("tree-sitter parsing returned None")?;

    let symbols = extractor::extract_symbols(&tree, source.as_bytes(), language);
    let chunks = chunker::chunks_from_symbols(&symbols, source, file_path, language, config);

    // If no symbols were extracted (e.g., empty file or unparseable),
    // fall back to Tier 0 to ensure content is still indexed.
    if chunks.is_empty() && !source.trim().is_empty() {
        Ok(chunker::tier0_line_chunks(source, file_path, language))
    } else {
        Ok(chunks)
    }
}

/// Parse a source file and extract call-site references.
///
/// Only works for languages with tree-sitter grammars. Returns an empty
/// vec for unsupported languages or parse failures.
pub fn parse_and_extract_references(
    source: &str,
    language: Language,
) -> Vec<references::RawReference> {
    let grammar = match languages::tree_sitter_grammar(language) {
        Some(g) => g,
        None => return Vec::new(),
    };
    let mut parser = Parser::new();
    if parser.set_language(&grammar).is_err() {
        return Vec::new();
    }
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };
    references::extract_references(&tree, source.as_bytes(), language)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod metadata_tests;

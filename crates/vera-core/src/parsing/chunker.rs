//! Symbol-aware chunking and Tier 0 fallback.
//!
//! Converts extracted AST symbols into [`Chunk`]s. Handles:
//! - Symbol → chunk mapping with metadata
//! - Large symbol splitting (>configured threshold)
//! - Gap chunks for inter-symbol code (imports, module-level statements)
//! - Tier 0 fallback: sliding-window line-based chunking for unknown languages

use crate::config::IndexingConfig;
use crate::types::{Chunk, Language, SymbolType};

use super::extractor::RawSymbol;

/// Default sliding-window size for Tier 0 fallback (lines).
const TIER0_WINDOW_SIZE: u32 = 50;
/// Default overlap for Tier 0 sliding-window (lines).
const TIER0_OVERLAP: u32 = 10;
/// Minimum lines for a symbol to be kept as a chunk (skip trivial ones).
const MIN_SYMBOL_LINES: u32 = 1;

/// Create chunks from extracted symbols (Tier 1A: symbol-aware chunking).
///
/// Produces one chunk per symbol. Large symbols exceeding `max_chunk_lines`
/// are split into sub-chunks with no content gaps. Inter-symbol gaps
/// (imports, blank lines, module-level code) are captured as gap chunks.
pub fn chunks_from_symbols(
    symbols: &[RawSymbol],
    source: &str,
    file_path: &str,
    language: Language,
    config: &IndexingConfig,
) -> Vec<Chunk> {
    let lines: Vec<&str> = source.lines().collect();
    let total_lines = lines.len() as u32;
    let mut chunks = Vec::new();
    let mut chunk_index: u32 = 0;

    // Track coverage to identify gaps
    let mut covered_end_row: u32 = 0;

    for symbol in symbols {
        let sym_start = symbol.start_row as u32;
        let sym_end = symbol.end_row as u32;
        let sym_lines = sym_end.saturating_sub(sym_start) + 1;

        // Skip trivially small symbols (e.g., single-line forward declarations)
        if sym_lines < MIN_SYMBOL_LINES {
            continue;
        }

        // Capture gap before this symbol (imports, blank lines, etc.)
        if sym_start > covered_end_row {
            let gap_content = join_lines(&lines, covered_end_row, sym_start.saturating_sub(1));
            if !gap_content.trim().is_empty() {
                chunks.push(Chunk {
                    id: format!("{file_path}:{chunk_index}"),
                    file_path: file_path.to_string(),
                    line_start: covered_end_row + 1, // 1-based
                    line_end: sym_start,             // 1-based
                    content: gap_content,
                    language,
                    symbol_type: Some(SymbolType::Block),
                    symbol_name: None,
                });
                chunk_index += 1;
            }
        }

        // Split large symbols into sub-chunks
        if sym_lines > config.max_chunk_lines {
            let sub_chunks = split_large_symbol(
                symbol,
                &lines,
                file_path,
                language,
                config.max_chunk_lines,
                &mut chunk_index,
            );
            chunks.extend(sub_chunks);
        } else {
            let content = join_lines(&lines, sym_start, sym_end);
            chunks.push(Chunk {
                id: format!("{file_path}:{chunk_index}"),
                file_path: file_path.to_string(),
                line_start: sym_start + 1, // 1-based
                line_end: sym_end + 1,     // 1-based
                content,
                language,
                symbol_type: Some(symbol.symbol_type),
                symbol_name: symbol.name.clone(),
            });
            chunk_index += 1;
        }

        covered_end_row = sym_end + 1;
    }

    // Trailing gap after last symbol
    if covered_end_row < total_lines {
        let gap_content = join_lines(&lines, covered_end_row, total_lines.saturating_sub(1));
        if !gap_content.trim().is_empty() {
            chunks.push(Chunk {
                id: format!("{file_path}:{chunk_index}"),
                file_path: file_path.to_string(),
                line_start: covered_end_row + 1, // 1-based
                line_end: total_lines,           // 1-based
                content: gap_content,
                language,
                symbol_type: Some(SymbolType::Block),
                symbol_name: None,
            });
        }
    }

    chunks
}

/// Split a large symbol into sub-chunks of at most `max_lines` lines.
///
/// Sub-chunks have no content gaps — every line of the symbol appears
/// in exactly one sub-chunk. Each sub-chunk inherits the symbol's metadata.
fn split_large_symbol(
    symbol: &RawSymbol,
    lines: &[&str],
    file_path: &str,
    language: Language,
    max_lines: u32,
    chunk_index: &mut u32,
) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let start = symbol.start_row as u32;
    let end = symbol.end_row as u32;
    let mut current = start;
    let mut part = 1u32;

    while current <= end {
        let chunk_end = (current + max_lines - 1).min(end);
        let content = join_lines(lines, current, chunk_end);
        let sub_name = symbol.name.as_ref().map(|n| format!("{n} (part {part})"));

        chunks.push(Chunk {
            id: format!("{file_path}:{}", *chunk_index),
            file_path: file_path.to_string(),
            line_start: current + 1, // 1-based
            line_end: chunk_end + 1, // 1-based
            content,
            language,
            symbol_type: Some(symbol.symbol_type),
            symbol_name: sub_name,
        });

        *chunk_index += 1;
        part += 1;
        current = chunk_end + 1;
    }

    chunks
}

/// Tier 0 fallback: sliding-window line-based chunking.
///
/// Used for files with no tree-sitter grammar support. Produces overlapping
/// chunks of `TIER0_WINDOW_SIZE` lines with `TIER0_OVERLAP` overlap.
pub fn tier0_line_chunks(source: &str, file_path: &str, language: Language) -> Vec<Chunk> {
    let lines: Vec<&str> = source.lines().collect();
    let total = lines.len() as u32;

    if total == 0 {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut start: u32 = 0;
    let mut chunk_index: u32 = 0;
    let step = TIER0_WINDOW_SIZE.saturating_sub(TIER0_OVERLAP);

    while start < total {
        let end = (start + TIER0_WINDOW_SIZE - 1).min(total - 1);
        let content = join_lines(&lines, start, end);

        if !content.trim().is_empty() {
            chunks.push(Chunk {
                id: format!("{file_path}:{chunk_index}"),
                file_path: file_path.to_string(),
                line_start: start + 1, // 1-based
                line_end: end + 1,     // 1-based
                content,
                language,
                symbol_type: Some(SymbolType::Block),
                symbol_name: None,
            });
            chunk_index += 1;
        }

        // Avoid infinite loop when step is 0
        if step == 0 {
            break;
        }
        start += step;
    }

    chunks
}

/// Join lines from `start_row` to `end_row` (inclusive, 0-based) into a string.
fn join_lines(lines: &[&str], start_row: u32, end_row: u32) -> String {
    let start = start_row as usize;
    let end = (end_row as usize).min(lines.len().saturating_sub(1));
    if start > end || start >= lines.len() {
        return String::new();
    }
    lines[start..=end].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::extractor::RawSymbol;
    use crate::types::SymbolType;

    fn default_config() -> IndexingConfig {
        IndexingConfig {
            max_chunk_lines: 200,
            ..Default::default()
        }
    }

    #[test]
    fn single_symbol_becomes_one_chunk() {
        let source = "fn hello() {\n    println!(\"hi\");\n}\n";
        let symbols = vec![RawSymbol {
            name: Some("hello".to_string()),
            symbol_type: SymbolType::Function,
            start_byte: 0,
            end_byte: source.len(),
            start_row: 0,
            end_row: 2,
        }];
        let chunks = chunks_from_symbols(
            &symbols,
            source,
            "test.rs",
            Language::Rust,
            &default_config(),
        );
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].symbol_name, Some("hello".to_string()));
        assert_eq!(chunks[0].symbol_type, Some(SymbolType::Function));
        assert_eq!(chunks[0].line_start, 1);
        assert_eq!(chunks[0].line_end, 3);
    }

    #[test]
    fn gap_between_symbols_captured() {
        let source = "use std::io;\n\nfn hello() {\n}\n\nfn world() {\n}\n";
        let symbols = vec![
            RawSymbol {
                name: Some("hello".to_string()),
                symbol_type: SymbolType::Function,
                start_byte: 0,
                end_byte: 0,
                start_row: 2,
                end_row: 3,
            },
            RawSymbol {
                name: Some("world".to_string()),
                symbol_type: SymbolType::Function,
                start_byte: 0,
                end_byte: 0,
                start_row: 5,
                end_row: 6,
            },
        ];
        let chunks = chunks_from_symbols(
            &symbols,
            source,
            "test.rs",
            Language::Rust,
            &default_config(),
        );
        // Should have: gap (imports), hello, world
        assert!(
            chunks.len() >= 2,
            "expected >= 2 chunks, got {}",
            chunks.len()
        );
        // First chunk should be the gap (imports)
        assert_eq!(chunks[0].symbol_type, Some(SymbolType::Block));
        assert!(chunks[0].content.contains("use std::io"));
    }

    #[test]
    fn large_symbol_split_into_sub_chunks() {
        // Create a large function (10 lines, with max_chunk_lines=3)
        let mut lines = vec!["fn big() {".to_string()];
        for i in 0..8 {
            lines.push(format!("    let x{i} = {i};"));
        }
        lines.push("}".to_string());
        let source = lines.join("\n");

        let symbols = vec![RawSymbol {
            name: Some("big".to_string()),
            symbol_type: SymbolType::Function,
            start_byte: 0,
            end_byte: source.len(),
            start_row: 0,
            end_row: 9,
        }];

        let config = IndexingConfig {
            max_chunk_lines: 3,
            ..Default::default()
        };
        let chunks = chunks_from_symbols(&symbols, &source, "test.rs", Language::Rust, &config);

        // 10 lines / 3 lines per chunk = 4 sub-chunks (3+3+3+1)
        assert_eq!(
            chunks.len(),
            4,
            "expected 4 sub-chunks, got {}",
            chunks.len()
        );

        // Verify no content gaps: reconstruct and compare
        let mut all_content = String::new();
        for (i, chunk) in chunks.iter().enumerate() {
            if i > 0 {
                all_content.push('\n');
            }
            all_content.push_str(&chunk.content);
            // Sub-chunks should have part numbers in name
            assert!(
                chunk.symbol_name.as_ref().unwrap().contains("part"),
                "sub-chunk should have part number"
            );
        }
        assert_eq!(all_content, source);
    }

    #[test]
    fn tier0_fallback_produces_chunks() {
        let mut lines = Vec::new();
        for i in 0..120 {
            lines.push(format!("line {i}"));
        }
        let source = lines.join("\n");

        let chunks = tier0_line_chunks(&source, "data.xyz", Language::Unknown);
        // 120 lines, window=50, overlap=10, step=40
        // Chunks: [0..49], [40..89], [80..119] = 3 chunks
        assert_eq!(
            chunks.len(),
            3,
            "expected 3 tier0 chunks, got {}",
            chunks.len()
        );

        // All chunks should have Block type
        for chunk in &chunks {
            assert_eq!(chunk.symbol_type, Some(SymbolType::Block));
            assert_eq!(chunk.language, Language::Unknown);
        }

        // First chunk starts at line 1
        assert_eq!(chunks[0].line_start, 1);
        assert_eq!(chunks[0].line_end, 50);

        // Second chunk overlaps
        assert_eq!(chunks[1].line_start, 41);
        assert_eq!(chunks[1].line_end, 90);
    }

    #[test]
    fn tier0_empty_source_no_chunks() {
        let chunks = tier0_line_chunks("", "empty.xyz", Language::Unknown);
        assert!(chunks.is_empty());
    }

    #[test]
    fn tier0_small_file_one_chunk() {
        let source = "line 1\nline 2\nline 3";
        let chunks = tier0_line_chunks(source, "small.xyz", Language::Unknown);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].line_start, 1);
        assert_eq!(chunks[0].line_end, 3);
    }

    #[test]
    fn chunks_have_correct_file_path() {
        let source = "fn foo() {}\n";
        let symbols = vec![RawSymbol {
            name: Some("foo".to_string()),
            symbol_type: SymbolType::Function,
            start_byte: 0,
            end_byte: source.len(),
            start_row: 0,
            end_row: 0,
        }];
        let chunks = chunks_from_symbols(
            &symbols,
            source,
            "src/lib.rs",
            Language::Rust,
            &default_config(),
        );
        assert_eq!(chunks[0].file_path, "src/lib.rs");
    }

    #[test]
    fn chunks_have_unique_ids() {
        let source = "fn a() {}\nfn b() {}\n";
        let symbols = vec![
            RawSymbol {
                name: Some("a".to_string()),
                symbol_type: SymbolType::Function,
                start_byte: 0,
                end_byte: 0,
                start_row: 0,
                end_row: 0,
            },
            RawSymbol {
                name: Some("b".to_string()),
                symbol_type: SymbolType::Function,
                start_byte: 0,
                end_byte: 0,
                start_row: 1,
                end_row: 1,
            },
        ];
        let chunks = chunks_from_symbols(
            &symbols,
            source,
            "test.rs",
            Language::Rust,
            &default_config(),
        );
        let ids: Vec<_> = chunks.iter().map(|c| &c.id).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len(), "all chunk IDs should be unique");
    }
}

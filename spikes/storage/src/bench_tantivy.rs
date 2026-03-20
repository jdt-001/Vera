//! Benchmark: Tantivy for BM25 full-text search.

use crate::common::{BenchmarkReport, TestChunk, dir_size, percentile};
use std::path::PathBuf;
use tantivy::{
    Index, IndexWriter,
    collector::TopDocs,
    doc,
    query::QueryParser,
    schema::{FAST, INDEXED, STORED, Schema, TEXT, Value},
};

const DB_DIR: &str = "bench_data/tantivy";

pub fn run_benchmark(chunks: &[TestChunk], num_queries: usize, top_k: usize) -> BenchmarkReport {
    let db_dir = PathBuf::from(DB_DIR);
    let _ = std::fs::remove_dir_all(&db_dir);
    std::fs::create_dir_all(&db_dir).unwrap();

    // Build schema
    let mut schema_builder = Schema::builder();
    let id_field = schema_builder.add_u64_field("id", INDEXED | STORED | FAST);
    let file_path_field = schema_builder.add_text_field("file_path", TEXT | STORED);
    let symbol_name_field = schema_builder.add_text_field("symbol_name", TEXT | STORED);
    let symbol_type_field = schema_builder.add_text_field("symbol_type", TEXT | STORED);
    let language_field = schema_builder.add_text_field("language", TEXT | STORED);
    let content_field = schema_builder.add_text_field("content", TEXT | STORED);
    let schema = schema_builder.build();

    // Create index
    let index = Index::create_in_dir(&db_dir, schema.clone()).unwrap();
    let mut writer: IndexWriter = index.writer(50_000_000).unwrap(); // 50MB heap

    // --- Write benchmark ---
    let write_start = std::time::Instant::now();

    for chunk in chunks {
        writer
            .add_document(doc!(
                id_field => chunk.id,
                file_path_field => chunk.file_path.clone(),
                symbol_name_field => chunk.symbol_name.clone(),
                symbol_type_field => chunk.symbol_type.clone(),
                language_field => chunk.language.clone(),
                content_field => chunk.content.clone(),
            ))
            .unwrap();
    }
    writer.commit().unwrap();

    let write_elapsed = write_start.elapsed();
    let write_total_ms = write_elapsed.as_secs_f64() * 1000.0;

    // Open reader
    let reader = index.reader().unwrap();
    let searcher = reader.searcher();
    let query_parser = QueryParser::for_index(&index, vec![content_field, symbol_name_field]);

    // --- BM25 query benchmark ---
    // Use realistic search terms
    let search_terms = [
        "request response handler",
        "database query execute",
        "config options params",
        "cache transform process",
        "authenticate authorize context",
        "serialize deserialize data",
        "connect disconnect session",
        "filter sort aggregate",
        "render format output",
        "compress decompress buffer",
        "handleRequest",
        "processResponse",
        "createClient",
        "validateInput",
        "fetchData",
        "Result",
        "Context",
        "Config",
        "fn execute",
        "async def handle",
    ];

    // Warmup
    for term in search_terms.iter().take(5) {
        let query = query_parser.parse_query(term).unwrap();
        let _results = searcher
            .search(&query, &TopDocs::with_limit(top_k))
            .unwrap();
    }

    let mut query_latencies = Vec::with_capacity(num_queries);
    for i in 0..num_queries {
        let term = search_terms[i % search_terms.len()];
        let query = query_parser.parse_query(term).unwrap();

        let start = std::time::Instant::now();
        let _results = searcher
            .search(&query, &TopDocs::with_limit(top_k))
            .unwrap();
        query_latencies.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    query_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // --- Storage size ---
    let storage_size = dir_size(&db_dir);

    let mean = query_latencies.iter().sum::<f64>() / query_latencies.len() as f64;

    // Verify correctness: search for a known symbol name
    let test_query = query_parser.parse_query(&chunks[0].symbol_name).unwrap();
    let results = searcher
        .search(&test_query, &TopDocs::with_limit(1))
        .unwrap();

    let mut notes = vec![
        "Tantivy BM25 with default tokenizer".to_string(),
        format!("Writer heap: 50MB"),
        format!("Indexed fields: content, symbol_name, file_path, symbol_type, language"),
    ];

    if !results.is_empty() {
        let doc = searcher
            .doc::<tantivy::TantivyDocument>(results[0].1)
            .unwrap();
        let retrieved_name = doc
            .get_first(symbol_name_field)
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if retrieved_name == chunks[0].symbol_name {
            notes.push("Correctness check: symbol name search returns correct doc ✓".to_string());
        } else {
            notes.push(format!(
                "Correctness check: expected '{}', got '{}'",
                chunks[0].symbol_name, retrieved_name
            ));
        }
    }

    BenchmarkReport {
        backend: "Tantivy (BM25)".to_string(),
        num_chunks: chunks.len(),
        vector_dim: 0,
        write_total_ms,
        write_throughput_per_sec: chunks.len() as f64 / write_elapsed.as_secs_f64(),
        vector_query_p50_ms: None,
        vector_query_p95_ms: None,
        vector_query_p99_ms: None,
        vector_query_mean_ms: None,
        bm25_query_p50_ms: Some(percentile(&query_latencies, 50.0)),
        bm25_query_p95_ms: Some(percentile(&query_latencies, 95.0)),
        bm25_query_p99_ms: Some(percentile(&query_latencies, 99.0)),
        bm25_query_mean_ms: Some(mean),
        storage_size_bytes: storage_size,
        notes,
    }
}

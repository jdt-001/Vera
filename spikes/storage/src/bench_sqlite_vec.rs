//! Benchmark: SQLite + sqlite-vec for vector storage and metadata.

use crate::common::{BenchmarkReport, TestChunk, dir_size, percentile};
use rusqlite::{Connection, ffi::sqlite3_auto_extension, params};
use sqlite_vec::sqlite3_vec_init;
use std::path::PathBuf;
use zerocopy::IntoBytes;

const DB_DIR: &str = "bench_data/sqlite_vec";

pub fn run_benchmark(
    chunks: &[TestChunk],
    vector_dim: usize,
    num_queries: usize,
    top_k: usize,
) -> BenchmarkReport {
    let db_dir = PathBuf::from(DB_DIR);
    // Clean up previous run
    let _ = std::fs::remove_dir_all(&db_dir);
    std::fs::create_dir_all(&db_dir).unwrap();

    let db_path = db_dir.join("vera.db");

    // Register sqlite-vec extension
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    }

    let db = Connection::open(&db_path).unwrap();

    // Enable WAL mode for better write performance
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
        .unwrap();

    // Create metadata table
    db.execute_batch(
        "CREATE TABLE chunks (
            id INTEGER PRIMARY KEY,
            file_path TEXT NOT NULL,
            symbol_name TEXT NOT NULL,
            symbol_type TEXT NOT NULL,
            language TEXT NOT NULL,
            content TEXT NOT NULL,
            line_start INTEGER NOT NULL,
            line_end INTEGER NOT NULL
        )",
    )
    .unwrap();

    // Create virtual table for vector search
    db.execute_batch(&format!(
        "CREATE VIRTUAL TABLE vec_chunks USING vec0(embedding float[{vector_dim}])"
    ))
    .unwrap();

    // --- Write benchmark ---
    let write_start = std::time::Instant::now();

    // Insert in batches for realistic throughput measurement
    let batch_size = 500;
    for batch in chunks.chunks(batch_size) {
        let tx = db.unchecked_transaction().unwrap();
        {
            let mut meta_stmt = db
                .prepare_cached(
                    "INSERT INTO chunks (id, file_path, symbol_name, symbol_type, language, content, line_start, line_end)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                )
                .unwrap();
            let mut vec_stmt = db
                .prepare_cached("INSERT INTO vec_chunks (rowid, embedding) VALUES (?1, ?2)")
                .unwrap();

            for chunk in batch {
                meta_stmt
                    .execute(params![
                        chunk.id,
                        chunk.file_path,
                        chunk.symbol_name,
                        chunk.symbol_type,
                        chunk.language,
                        chunk.content,
                        chunk.line_start,
                        chunk.line_end,
                    ])
                    .unwrap();

                vec_stmt
                    .execute(params![chunk.id, chunk.vector.as_bytes()])
                    .unwrap();
            }
        }
        tx.commit().unwrap();
    }

    let write_elapsed = write_start.elapsed();
    let write_total_ms = write_elapsed.as_secs_f64() * 1000.0;

    // Force flush
    db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").unwrap();

    // --- Vector query benchmark ---
    let query_vectors: Vec<&[f32]> = chunks
        .iter()
        .take(num_queries)
        .map(|c| c.vector.as_slice())
        .collect();

    // Warmup
    for qvec in query_vectors.iter().take(5) {
        let _: Vec<(i64, f64)> = db
            .prepare_cached(&format!(
                "SELECT rowid, distance FROM vec_chunks WHERE embedding MATCH ?1 ORDER BY distance LIMIT {top_k}"
            ))
            .unwrap()
            .query_map([qvec.as_bytes()], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
    }

    let mut query_latencies = Vec::with_capacity(num_queries);
    for qvec in &query_vectors {
        let start = std::time::Instant::now();
        let _results: Vec<(i64, f64)> = db
            .prepare_cached(&format!(
                "SELECT rowid, distance FROM vec_chunks WHERE embedding MATCH ?1 ORDER BY distance LIMIT {top_k}"
            ))
            .unwrap()
            .query_map([qvec.as_bytes()], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        query_latencies.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    query_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // --- Storage size ---
    let storage_size = dir_size(&db_dir);

    // Clean up
    drop(db);

    let mean = query_latencies.iter().sum::<f64>() / query_latencies.len() as f64;

    let mut notes = vec![
        format!("SQLite WAL mode, batch size {batch_size}"),
        format!("sqlite-vec virtual table with float[{vector_dim}]"),
        "Brute-force KNN (no ANN index)".to_string(),
    ];

    // Verify correctness: query a known vector and check it returns itself
    let db2 = Connection::open(&db_path).unwrap();
    let result: Vec<(i64, f64)> = db2
        .prepare(&format!(
            "SELECT rowid, distance FROM vec_chunks WHERE embedding MATCH ?1 ORDER BY distance LIMIT 1"
        ))
        .unwrap()
        .query_map([chunks[0].vector.as_bytes()], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if result[0].0 == chunks[0].id as i64 {
        notes.push("Correctness check: self-query returns self ✓".to_string());
    } else {
        notes.push(format!(
            "Correctness check: expected id {}, got {}",
            chunks[0].id, result[0].0
        ));
    }

    BenchmarkReport {
        backend: "SQLite + sqlite-vec".to_string(),
        num_chunks: chunks.len(),
        vector_dim,
        write_total_ms,
        write_throughput_per_sec: chunks.len() as f64 / write_elapsed.as_secs_f64(),
        vector_query_p50_ms: Some(percentile(&query_latencies, 50.0)),
        vector_query_p95_ms: Some(percentile(&query_latencies, 95.0)),
        vector_query_p99_ms: Some(percentile(&query_latencies, 99.0)),
        vector_query_mean_ms: Some(mean),
        bm25_query_p50_ms: None,
        bm25_query_p95_ms: None,
        bm25_query_p99_ms: None,
        bm25_query_mean_ms: None,
        storage_size_bytes: storage_size,
        notes,
    }
}

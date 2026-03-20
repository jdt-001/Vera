//! Storage Backend Spike: SQLite+sqlite-vec vs LanceDB vs Tantivy
//!
//! Tests write throughput, vector query latency, BM25 query latency,
//! and storage size for Vera's storage backend decision.

mod bench_lancedb;
mod bench_sqlite_vec;
mod bench_tantivy;
mod common;

use common::{BenchmarkReport, generate_test_chunks};
use serde_json;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let num_chunks = 10_000;
    let vector_dim = 768; // Typical embedding dimension
    let num_queries = 100;
    let top_k = 10;

    println!("=== Vera Storage Backend Spike ===");
    println!(
        "Chunks: {num_chunks}, Vector dim: {vector_dim}, Queries: {num_queries}, Top-K: {top_k}"
    );
    println!();

    // Generate test data once
    println!("Generating {num_chunks} test chunks with {vector_dim}-dim vectors...");
    let start = std::time::Instant::now();
    let chunks = generate_test_chunks(num_chunks, vector_dim);
    println!(
        "Generated in {:.1}ms\n",
        start.elapsed().as_secs_f64() * 1000.0
    );

    let results_dir = PathBuf::from("results");
    std::fs::create_dir_all(&results_dir).unwrap();

    // --- SQLite + sqlite-vec ---
    println!("━━━ SQLite + sqlite-vec ━━━");
    let sqlite_report = bench_sqlite_vec::run_benchmark(&chunks, vector_dim, num_queries, top_k);
    let json = serde_json::to_string_pretty(&sqlite_report).unwrap();
    std::fs::write(results_dir.join("sqlite_vec.json"), &json).unwrap();
    print_report(&sqlite_report);

    // --- LanceDB ---
    println!("\n━━━ LanceDB ━━━");
    let lancedb_report =
        bench_lancedb::run_benchmark(&chunks, vector_dim, num_queries, top_k).await;
    let json = serde_json::to_string_pretty(&lancedb_report).unwrap();
    std::fs::write(results_dir.join("lancedb.json"), &json).unwrap();
    print_report(&lancedb_report);

    // --- Tantivy (BM25) ---
    println!("\n━━━ Tantivy (BM25) ━━━");
    let tantivy_report = bench_tantivy::run_benchmark(&chunks, num_queries, top_k);
    let json = serde_json::to_string_pretty(&tantivy_report).unwrap();
    std::fs::write(results_dir.join("tantivy.json"), &json).unwrap();
    print_report(&tantivy_report);

    // --- Comparison summary ---
    println!("\n━━━ Comparison Summary ━━━");
    println!(
        "{:<25} {:>12} {:>14} {:>14} {:>12}",
        "Backend", "Write (ms)", "Vec p50 (ms)", "Vec p95 (ms)", "Size (KB)"
    );
    println!("{}", "─".repeat(80));

    print_summary_row("SQLite+sqlite-vec", &sqlite_report);
    print_summary_row("LanceDB", &lancedb_report);
    print_summary_row("Tantivy (BM25)", &tantivy_report);

    // Save combined summary
    let summary = serde_json::json!({
        "config": {
            "num_chunks": num_chunks,
            "vector_dim": vector_dim,
            "num_queries": num_queries,
            "top_k": top_k,
        },
        "results": {
            "sqlite_vec": sqlite_report,
            "lancedb": lancedb_report,
            "tantivy": tantivy_report,
        }
    });
    std::fs::write(
        results_dir.join("summary.json"),
        serde_json::to_string_pretty(&summary).unwrap(),
    )
    .unwrap();
    println!("\nResults saved to results/");
}

fn print_report(report: &BenchmarkReport) {
    println!(
        "  Write throughput: {:.1}ms total, {:.0} chunks/sec",
        report.write_total_ms, report.write_throughput_per_sec
    );
    if let Some(p50) = report.vector_query_p50_ms {
        println!(
            "  Vector query p50: {:.3}ms, p95: {:.3}ms, p99: {:.3}ms",
            p50,
            report.vector_query_p95_ms.unwrap_or(0.0),
            report.vector_query_p99_ms.unwrap_or(0.0)
        );
    }
    if let Some(p50) = report.bm25_query_p50_ms {
        println!(
            "  BM25 query p50: {:.3}ms, p95: {:.3}ms, p99: {:.3}ms",
            p50,
            report.bm25_query_p95_ms.unwrap_or(0.0),
            report.bm25_query_p99_ms.unwrap_or(0.0)
        );
    }
    println!(
        "  Storage size: {:.1} KB ({:.1} MB)",
        report.storage_size_bytes as f64 / 1024.0,
        report.storage_size_bytes as f64 / (1024.0 * 1024.0)
    );
}

fn print_summary_row(name: &str, report: &BenchmarkReport) {
    println!(
        "{:<25} {:>12.1} {:>14} {:>14} {:>12.1}",
        name,
        report.write_total_ms,
        report
            .vector_query_p50_ms
            .map(|v| format!("{v:.3}"))
            .unwrap_or_else(|| "N/A".to_string()),
        report
            .vector_query_p95_ms
            .map(|v| format!("{v:.3}"))
            .unwrap_or_else(|| "N/A".to_string()),
        report.storage_size_bytes as f64 / 1024.0
    );
}

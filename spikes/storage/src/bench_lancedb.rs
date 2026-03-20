//! Benchmark: LanceDB for vector storage and metadata.

use crate::common::{BenchmarkReport, TestChunk, dir_size, percentile};
use arrow_array::{
    FixedSizeListArray, Int64Array, RecordBatch, RecordBatchIterator, RecordBatchReader,
    StringArray, UInt32Array, types::Float32Type,
};
use arrow_schema::{DataType, Field, Schema};
use lancedb::query::{ExecutableQuery, QueryBase};
use std::path::PathBuf;
use std::sync::Arc;

const DB_DIR: &str = "bench_data/lancedb";

pub async fn run_benchmark(
    chunks: &[TestChunk],
    vector_dim: usize,
    num_queries: usize,
    top_k: usize,
) -> BenchmarkReport {
    let db_dir = PathBuf::from(DB_DIR);
    // Clean up previous run
    let _ = std::fs::remove_dir_all(&db_dir);
    std::fs::create_dir_all(&db_dir).unwrap();

    let db = lancedb::connect(db_dir.to_str().unwrap())
        .execute()
        .await
        .unwrap();

    // Define schema
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("file_path", DataType::Utf8, false),
        Field::new("symbol_name", DataType::Utf8, false),
        Field::new("symbol_type", DataType::Utf8, false),
        Field::new("language", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        Field::new("line_start", DataType::UInt32, false),
        Field::new("line_end", DataType::UInt32, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                vector_dim as i32,
            ),
            true,
        ),
    ]));

    // --- Write benchmark ---
    let write_start = std::time::Instant::now();

    let batch_size = 1000;
    let mut table = None;

    for (batch_idx, batch) in chunks.chunks(batch_size).enumerate() {
        let ids = Int64Array::from(batch.iter().map(|c| c.id as i64).collect::<Vec<_>>());
        let file_paths = StringArray::from(
            batch
                .iter()
                .map(|c| c.file_path.as_str())
                .collect::<Vec<_>>(),
        );
        let symbol_names = StringArray::from(
            batch
                .iter()
                .map(|c| c.symbol_name.as_str())
                .collect::<Vec<_>>(),
        );
        let symbol_types = StringArray::from(
            batch
                .iter()
                .map(|c| c.symbol_type.as_str())
                .collect::<Vec<_>>(),
        );
        let languages = StringArray::from(
            batch
                .iter()
                .map(|c| c.language.as_str())
                .collect::<Vec<_>>(),
        );
        let contents =
            StringArray::from(batch.iter().map(|c| c.content.as_str()).collect::<Vec<_>>());
        let line_starts = UInt32Array::from(batch.iter().map(|c| c.line_start).collect::<Vec<_>>());
        let line_ends = UInt32Array::from(batch.iter().map(|c| c.line_end).collect::<Vec<_>>());

        let vectors = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
            batch
                .iter()
                .map(|c| Some(c.vector.iter().map(|v| Some(*v)).collect::<Vec<_>>())),
            vector_dim as i32,
        );

        let record_batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(ids),
                Arc::new(file_paths),
                Arc::new(symbol_names),
                Arc::new(symbol_types),
                Arc::new(languages),
                Arc::new(contents),
                Arc::new(line_starts),
                Arc::new(line_ends),
                Arc::new(vectors),
            ],
        )
        .unwrap();

        if batch_idx == 0 {
            let batches: Box<dyn RecordBatchReader + Send> = Box::new(RecordBatchIterator::new(
                vec![Ok(record_batch)],
                schema.clone(),
            ));
            table = Some(db.create_table("chunks", batches).execute().await.unwrap());
        } else if let Some(ref tbl) = table {
            let batches: Box<dyn RecordBatchReader + Send> = Box::new(RecordBatchIterator::new(
                vec![Ok(record_batch)],
                schema.clone(),
            ));
            tbl.add(batches).execute().await.unwrap();
        }
    }

    let write_elapsed = write_start.elapsed();
    let write_total_ms = write_elapsed.as_secs_f64() * 1000.0;

    let tbl = table.unwrap();

    // --- Vector query benchmark ---
    let query_vectors: Vec<Vec<f32>> = chunks
        .iter()
        .take(num_queries)
        .map(|c| c.vector.clone())
        .collect();

    // Warmup queries
    for qvec in query_vectors.iter().take(5) {
        use futures::TryStreamExt;
        let _results: Vec<RecordBatch> = tbl
            .query()
            .nearest_to(qvec.as_slice())
            .unwrap()
            .limit(top_k)
            .execute()
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();
    }

    let mut query_latencies = Vec::with_capacity(num_queries);
    for qvec in &query_vectors {
        use futures::TryStreamExt;
        let start = std::time::Instant::now();
        let _results: Vec<RecordBatch> = tbl
            .query()
            .nearest_to(qvec.as_slice())
            .unwrap()
            .limit(top_k)
            .execute()
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();
        query_latencies.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    query_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // --- Storage size ---
    let storage_size = dir_size(&db_dir);

    let mean = query_latencies.iter().sum::<f64>() / query_latencies.len() as f64;

    let mut notes = vec![
        format!("LanceDB local, batch size {batch_size}"),
        "Arrow-native RecordBatch API".to_string(),
        "No ANN index (brute-force flat scan, same as sqlite-vec)".to_string(),
    ];

    // Verify correctness
    {
        use futures::TryStreamExt;
        let results: Vec<RecordBatch> = tbl
            .query()
            .nearest_to(chunks[0].vector.as_slice())
            .unwrap()
            .limit(1)
            .execute()
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();
        if !results.is_empty() {
            let ids = results[0]
                .column_by_name("id")
                .unwrap()
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap();
            if ids.value(0) == chunks[0].id as i64 {
                notes.push("Correctness check: self-query returns self ✓".to_string());
            } else {
                notes.push(format!(
                    "Correctness check: expected id {}, got {}",
                    chunks[0].id,
                    ids.value(0)
                ));
            }
        }
    }

    BenchmarkReport {
        backend: "LanceDB".to_string(),
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

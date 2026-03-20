//! Index construction and maintenance.
//!
//! This module is responsible for:
//! - Orchestrating the indexing pipeline (discover → parse → chunk → embed → store)
//! - Building BM25 full-text indexes via Tantivy
//! - Building vector indexes via sqlite-vec
//! - Incremental update logic (detect changed files, re-index only those)

pub mod pipeline;
pub mod update;

pub use pipeline::{FileError, IndexSummary, index_dir, index_repository};
pub use update::{UpdateSummary, content_hash, update_repository};

use crate::config::OnnxExecutionProvider;
use crate::embedding::provider::{EmbeddingError, EmbeddingProvider};
use crate::local_models::ensure_model_file;
use anyhow::{Context, Result};
use ort::session::{Session, builder::GraphOptimizationLevel};
use std::sync::{Arc, Mutex};
use tokenizers::Tokenizer;
use tokio::task;

const EMBEDDING_REPO: &str = "jinaai/jina-embeddings-v5-text-nano-retrieval";
const ONNX_FILE: &str = "onnx/model_quantized.onnx";
const ONNX_DATA_FILE: &str = "onnx/model_quantized.onnx_data";
const TOKENIZER_FILE: &str = "tokenizer.json";
const EMBEDDING_DIM: usize = 768;

#[derive(Clone)]
pub struct LocalEmbeddingProvider {
    session: Arc<Mutex<Session>>,
    tokenizer: Arc<Tokenizer>,
}

impl LocalEmbeddingProvider {
    pub async fn new_with_ep(ep: OnnxExecutionProvider) -> Result<Self, EmbeddingError> {
        let ort_path = crate::local_models::ensure_ort_library_for_ep(ep)
            .await
            .map_err(|e| EmbeddingError::ApiError {
                status: 500,
                message: e.to_string(),
            })?;
        crate::local_models::ensure_ort_runtime(Some(&ort_path)).map_err(|e| {
            EmbeddingError::ApiError {
                status: 500,
                message: e.to_string(),
            }
        })?;
        crate::local_models::ensure_provider_dependencies(ep, &ort_path).map_err(|e| {
            EmbeddingError::ApiError {
                status: 500,
                message: e.to_string(),
            }
        })?;

        let onnx_path = ensure_model_file(EMBEDDING_REPO, ONNX_FILE)
            .await
            .map_err(|e| EmbeddingError::ApiError {
                status: 500,
                message: format!("Failed to download ONNX model: {}", e),
            })?;

        let _ = ensure_model_file(EMBEDDING_REPO, ONNX_DATA_FILE)
            .await
            .map_err(|e| EmbeddingError::ApiError {
                status: 500,
                message: format!("Failed to download ONNX data: {}", e),
            })?;

        let tokenizer_path = ensure_model_file(EMBEDDING_REPO, TOKENIZER_FILE)
            .await
            .map_err(|e| EmbeddingError::ApiError {
                status: 500,
                message: format!("Failed to download tokenizer: {}", e),
            })?;

        let tokenizer = task::spawn_blocking(move || load_tokenizer(tokenizer_path))
            .await
            .map_err(|e| EmbeddingError::ApiError {
                status: 500,
                message: e.to_string(),
            })?
            .map_err(|e| EmbeddingError::ApiError {
                status: 500,
                message: e.to_string(),
            })?;

        let session = task::spawn_blocking(move || build_session(ep, onnx_path))
            .await
            .map_err(|e| EmbeddingError::ApiError {
                status: 500,
                message: e.to_string(),
            })?
            .map_err(|e| EmbeddingError::ApiError {
                status: 500,
                message: crate::local_models::wrap_ort_error(e),
            })?;

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            tokenizer: Arc::new(tokenizer),
        })
    }

    pub fn probe_provider_registration(ep: OnnxExecutionProvider) -> Result<()> {
        let builder = ort::session::builder::SessionBuilder::new()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(1)?;
        let _ = register_execution_provider(builder, ep)?;
        Ok(())
    }

    pub fn probe_session(ep: OnnxExecutionProvider) -> Result<()> {
        let ort_path = crate::local_models::ort_library_path_for_ep(ep)?;
        crate::local_models::ensure_ort_runtime(Some(&ort_path))?;
        let (onnx_path, _, _) = default_asset_paths()?;
        let _ = build_session(ep, onnx_path)?;
        Ok(())
    }

    pub fn probe_inference(ep: OnnxExecutionProvider) -> Result<()> {
        let ort_path = crate::local_models::ort_library_path_for_ep(ep)?;
        crate::local_models::ensure_ort_runtime(Some(&ort_path))?;
        let (onnx_path, _, tokenizer_path) = default_asset_paths()?;
        let mut session = build_session(ep, onnx_path)?;
        let tokenizer = load_tokenizer(tokenizer_path)?;
        run_probe_inference(&mut session, &tokenizer)
    }

    #[allow(clippy::needless_range_loop)]
    fn do_embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut encodings = Vec::with_capacity(texts.len());
        for text in texts {
            let encoding = self
                .tokenizer
                .encode(text.as_str(), true)
                .map_err(|e| anyhow::anyhow!("Tokenizer error: {}", e))?;
            encodings.push(encoding);
        }

        let batch_size = texts.len();
        let mut max_len = encodings
            .iter()
            .map(|e| e.get_ids().len())
            .max()
            .unwrap_or(0);
        if max_len == 0 {
            max_len = 1;
        }

        let mut input_ids = ndarray::Array2::<i64>::zeros((batch_size, max_len));
        let mut attention_mask = ndarray::Array2::<i64>::zeros((batch_size, max_len));

        for (i, encoding) in encodings.iter().enumerate() {
            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();
            let len = ids.len();
            for j in 0..len {
                input_ids[[i, j]] = ids[j] as i64;
                attention_mask[[i, j]] = mask[j] as i64;
            }
        }

        let input_ids_tensor = ort::value::Tensor::from_array(input_ids)
            .map_err(|e| anyhow::anyhow!("Tensor error: {}", e))?;
        let attention_mask_tensor = ort::value::Tensor::from_array(attention_mask.clone())
            .map_err(|e| anyhow::anyhow!("Tensor error: {}", e))?;

        let inputs = ort::inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => attention_mask_tensor,
        ];

        let mut session = self.session.lock().unwrap();
        let outputs = session.run(inputs)?;

        let output_value = outputs.values().next().unwrap();
        let (shape, data) = output_value.try_extract_tensor::<f32>()?;
        let ndim = shape.len();

        let mut result = Vec::with_capacity(batch_size);

        if ndim == 2 {
            let dim = shape[1] as usize;
            for i in 0..batch_size {
                let start = i * dim;
                let mut emb = data[start..start + dim].to_vec();
                let norm: f32 = emb.iter().map(|v| v * v).sum::<f32>().sqrt();
                if norm > 1e-6 {
                    for d in 0..EMBEDDING_DIM {
                        emb[d] /= norm;
                    }
                }
                result.push(emb);
            }
        } else if ndim == 3 {
            let seq_len = shape[1] as usize;
            let dim = shape[2] as usize;
            for i in 0..batch_size {
                let mut emb = vec![0.0; dim];
                let mut valid_tokens = 0.0;
                for j in 0..max_len {
                    if attention_mask[[i, j]] == 1 {
                        valid_tokens += 1.0;
                        for d in 0..dim {
                            emb[d] += data[i * seq_len * dim + j * dim + d];
                        }
                    }
                }
                if valid_tokens > 0.0 {
                    for d in 0..dim {
                        emb[d] /= valid_tokens;
                    }
                }

                let norm: f32 = emb.iter().map(|v| v * v).sum::<f32>().sqrt();
                if norm > 1e-6 {
                    for d in 0..EMBEDDING_DIM {
                        emb[d] /= norm;
                    }
                }
                result.push(emb);
            }
        } else {
            anyhow::bail!("Unexpected tensor shape: {:?}", shape);
        }

        Ok(result)
    }
}

fn default_asset_paths() -> Result<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf)> {
    let model_dir = crate::local_models::vera_home_dir()?
        .join("models")
        .join(EMBEDDING_REPO);
    Ok((
        model_dir.join(ONNX_FILE),
        model_dir.join(ONNX_DATA_FILE),
        model_dir.join(TOKENIZER_FILE),
    ))
}

fn load_tokenizer(tokenizer_path: std::path::PathBuf) -> Result<Tokenizer> {
    let mut tokenizer = Tokenizer::from_file(&tokenizer_path)
        .map_err(|e| anyhow::anyhow!("Tokenizer init failed: {}", e))?;
    tokenizer
        .with_truncation(Some(tokenizers::TruncationParams {
            max_length: 512,
            strategy: tokenizers::TruncationStrategy::LongestFirst,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("Tokenizer truncation init failed: {}", e))?;
    Ok(tokenizer)
}

fn build_session(ep: OnnxExecutionProvider, onnx_path: std::path::PathBuf) -> Result<Session> {
    let available = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    // CPU EP benefits from all cores; GPU EPs do compute on device,
    // so limit CPU threads to avoid contention.
    let threads = if ep == OnnxExecutionProvider::Cpu {
        available
    } else {
        available.min(4)
    };
    tracing::info!("ONNX session: {threads} intra-op threads (available: {available})");
    let builder = ort::session::builder::SessionBuilder::new()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(threads)?;
    let builder = register_execution_provider(builder, ep)?;
    builder
        .commit_from_file(&onnx_path)
        .with_context(|| format!("failed to load embedding model {}", onnx_path.display()))
}

fn run_probe_inference(session: &mut Session, tokenizer: &Tokenizer) -> Result<()> {
    let encoding = tokenizer
        .encode("vera doctor probe", true)
        .map_err(|e| anyhow::anyhow!("Tokenizer error: {}", e))?;

    let ids = encoding.get_ids();
    let mask = encoding.get_attention_mask();
    let max_len = ids.len().max(1);
    let mut input_ids = ndarray::Array2::<i64>::zeros((1, max_len));
    let mut attention_mask = ndarray::Array2::<i64>::zeros((1, max_len));

    for (index, token_id) in ids.iter().enumerate() {
        input_ids[[0, index]] = *token_id as i64;
    }
    for (index, mask_value) in mask.iter().enumerate() {
        attention_mask[[0, index]] = *mask_value as i64;
    }

    let inputs = ort::inputs![
        "input_ids" => ort::value::Tensor::from_array(input_ids)?,
        "attention_mask" => ort::value::Tensor::from_array(attention_mask)?,
    ];

    let outputs = session.run(inputs)?;
    let output = outputs
        .values()
        .next()
        .context("embedding model produced no outputs")?;
    let (_, data) = output.try_extract_tensor::<f32>()?;
    if data.is_empty() {
        anyhow::bail!("embedding output tensor was empty");
    }
    if !data.iter().all(|value| value.is_finite()) {
        anyhow::bail!("embedding output contained non-finite values");
    }
    Ok(())
}

impl EmbeddingProvider for LocalEmbeddingProvider {
    fn expected_dim(&self) -> Option<usize> {
        Some(EMBEDDING_DIM)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let provider = self.clone();
        let texts = texts.to_vec();

        task::spawn_blocking(move || {
            provider
                .do_embed(&texts)
                .map_err(|e| EmbeddingError::ApiError {
                    status: 500,
                    message: e.to_string(),
                })
        })
        .await
        .map_err(|e| EmbeddingError::ApiError {
            status: 500,
            message: e.to_string(),
        })?
    }
}

/// Register the appropriate ONNX execution provider on a session builder.
fn register_execution_provider(
    builder: ort::session::builder::SessionBuilder,
    ep: OnnxExecutionProvider,
) -> ort::Result<ort::session::builder::SessionBuilder> {
    match ep {
        OnnxExecutionProvider::Cpu => {
            tracing::info!("using CPU execution provider");
            Ok(builder)
        }
        OnnxExecutionProvider::Cuda => {
            tracing::info!("registering CUDA execution provider");
            let result = builder.with_execution_providers([
                ort::execution_providers::CUDAExecutionProvider::default().build(),
            ]);
            if result.is_ok() {
                tracing::info!(
                    "CUDA execution provider registered (will fall back to CPU if unavailable)"
                );
            }
            result
        }
        OnnxExecutionProvider::Rocm => {
            tracing::info!("registering ROCm execution provider");
            builder.with_execution_providers([
                ort::execution_providers::ROCmExecutionProvider::default().build(),
            ])
        }
        OnnxExecutionProvider::DirectMl => {
            tracing::info!("registering DirectML execution provider");
            builder.with_execution_providers([
                ort::execution_providers::DirectMLExecutionProvider::default().build(),
            ])
        }
        OnnxExecutionProvider::CoreMl => {
            tracing::info!("registering CoreML execution provider");
            let result = builder.with_execution_providers([
                ort::execution_providers::CoreMLExecutionProvider::default().build(),
            ]);
            if result.is_ok() {
                tracing::info!(
                    "CoreML execution provider registered (will fall back to CPU if unavailable)"
                );
            }
            result
        }
        OnnxExecutionProvider::OpenVino => {
            tracing::info!("registering OpenVINO execution provider");
            builder.with_execution_providers([
                ort::execution_providers::OpenVINOExecutionProvider::default().build(),
            ])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_embedding_provider() {
        // Skip if ONNX Runtime is not installed (requires libonnxruntime.so)
        if crate::local_models::ensure_ort_runtime(None).is_err() {
            eprintln!("Skipping: ONNX Runtime not available");
            return;
        }
        // Since test downloads ~150MB, this could take a moment.
        let provider = LocalEmbeddingProvider::new_with_ep(OnnxExecutionProvider::Cpu)
            .await
            .unwrap();
        let texts = vec!["Hello world".to_string(), "Another test".to_string()];
        let embeddings = provider.embed_batch(&texts).await.unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), EMBEDDING_DIM);

        assert!(embeddings[0].iter().all(|x| x.is_finite()));
        let sum_abs: f32 = embeddings[0].iter().map(|x| x.abs()).sum();
        assert!(sum_abs > 0.1);
    }
}

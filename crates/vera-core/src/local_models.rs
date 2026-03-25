use anyhow::{Context, Result};
use futures::StreamExt;
use reqwest::Client;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;

const HUB_URL: &str = "https://huggingface.co";
const EMBEDDING_REPO: &str = "jinaai/jina-embeddings-v5-text-nano-retrieval";
const EMBEDDING_ONNX_FILE: &str = "onnx/model_quantized.onnx";
const EMBEDDING_ONNX_DATA_FILE: &str = "onnx/model_quantized.onnx_data";
const EMBEDDING_TOKENIZER_FILE: &str = "tokenizer.json";
const RERANKER_REPO: &str = "jinaai/jina-reranker-v2-base-multilingual";
const RERANKER_ONNX_FILE: &str = "onnx/model_quantized.onnx";
const RERANKER_TOKENIZER_FILE: &str = "tokenizer.json";

static ORT_INIT_RESULT: OnceLock<std::result::Result<(), String>> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
pub struct LocalModelAssetStatus {
    pub name: &'static str,
    pub path: PathBuf,
    pub exists: bool,
}

/// Ensure the ONNX Runtime shared library is loaded and initialized.
///
/// With `ort`'s `load-dynamic` feature, the ONNX Runtime library is loaded at
/// runtime via dlopen/LoadLibrary. This function proactively loads the library
/// using `ort::init_from()` so we get a clean error instead of a panic.
///
/// Safe to call multiple times — only the first call takes effect.
///
/// Returns a graceful error if ONNX Runtime is not installed, suggesting
/// the user install it or use API mode instead.
pub fn ensure_ort_runtime() -> Result<()> {
    let result = ORT_INIT_RESULT.get_or_init(|| {
        let lib_name = ort_lib_filename();
        match ort::init_from(lib_name) {
            Ok(builder) => {
                builder.commit();
                Ok(())
            }
            Err(e) => Err(format!(
                "ONNX Runtime shared library not found. Local inference requires ONNX Runtime to be installed.\n\
                 Install it from: https://github.com/microsoft/onnxruntime/releases\n\
                 Or use API mode instead by setting EMBEDDING_MODEL_BASE_URL, EMBEDDING_MODEL_ID, and EMBEDDING_MODEL_API_KEY.\n\
                 Original error: {e}"
            )),
        }
    });

    match result {
        Ok(()) => Ok(()),
        Err(msg) => anyhow::bail!("{msg}"),
    }
}

/// Return Vera's home directory.
///
/// Uses `VERA_HOME` when set, otherwise defaults to `~/.vera`.
pub fn vera_home_dir() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("VERA_HOME") {
        if !path.trim().is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    Ok(dirs::home_dir()
        .context("Could not find home directory")?
        .join(".vera"))
}

/// Get the platform-specific ONNX Runtime shared library filename.
///
/// Also checks the `ORT_DYLIB_PATH` environment variable for a custom path.
fn ort_lib_filename() -> String {
    if let Ok(path) = std::env::var("ORT_DYLIB_PATH") {
        if !path.is_empty() {
            return path;
        }
    }

    #[cfg(target_os = "windows")]
    {
        "onnxruntime.dll".to_string()
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        "libonnxruntime.so".to_string()
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        "libonnxruntime.dylib".to_string()
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    )))]
    {
        "libonnxruntime.so".to_string()
    }
}

/// Wrap an ort error with a user-friendly message suggesting alternatives.
pub fn wrap_ort_error(e: impl std::fmt::Display) -> String {
    let err_msg = e.to_string();
    if err_msg.contains("load")
        || err_msg.contains("libonnxruntime")
        || err_msg.contains("onnxruntime")
        || err_msg.contains("dylib")
        || err_msg.contains("dll")
        || err_msg.contains(".so")
    {
        format!(
            "ONNX Runtime shared library not found. Local inference requires ONNX Runtime to be installed.\n\
             Install it from: https://github.com/microsoft/onnxruntime/releases\n\
             Or use API mode instead by setting EMBEDDING_MODEL_BASE_URL, EMBEDDING_MODEL_ID, and EMBEDDING_MODEL_API_KEY.\n\
             Original error: {err_msg}"
        )
    } else {
        format!("Failed to initialize ONNX Runtime: {err_msg}")
    }
}

/// Download a file from HuggingFace Hub using atomic writes.
pub async fn ensure_model_file(repo_id: &str, file_path: &str) -> Result<PathBuf> {
    ensure_model_file_impl(repo_id, file_path, HUB_URL, None).await
}

/// Download the default local embedding and reranker assets.
pub async fn prefetch_default_local_models() -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    paths.push(ensure_model_file(EMBEDDING_REPO, EMBEDDING_ONNX_FILE).await?);
    paths.push(ensure_model_file(EMBEDDING_REPO, EMBEDDING_ONNX_DATA_FILE).await?);
    paths.push(ensure_model_file(EMBEDDING_REPO, EMBEDDING_TOKENIZER_FILE).await?);
    paths.push(ensure_model_file(RERANKER_REPO, RERANKER_ONNX_FILE).await?);
    paths.push(ensure_model_file(RERANKER_REPO, RERANKER_TOKENIZER_FILE).await?);
    Ok(paths)
}

/// Inspect the default local assets without downloading anything.
pub fn inspect_default_local_model_files() -> Result<Vec<LocalModelAssetStatus>> {
    let vera_home = vera_home_dir()?;
    let assets = [
        (
            "embedding-onnx",
            vera_home
                .join("models")
                .join(EMBEDDING_REPO)
                .join(EMBEDDING_ONNX_FILE),
        ),
        (
            "embedding-onnx-data",
            vera_home
                .join("models")
                .join(EMBEDDING_REPO)
                .join(EMBEDDING_ONNX_DATA_FILE),
        ),
        (
            "embedding-tokenizer",
            vera_home
                .join("models")
                .join(EMBEDDING_REPO)
                .join(EMBEDDING_TOKENIZER_FILE),
        ),
        (
            "reranker-onnx",
            vera_home
                .join("models")
                .join(RERANKER_REPO)
                .join(RERANKER_ONNX_FILE),
        ),
        (
            "reranker-tokenizer",
            vera_home
                .join("models")
                .join(RERANKER_REPO)
                .join(RERANKER_TOKENIZER_FILE),
        ),
    ];

    Ok(assets
        .into_iter()
        .map(|(name, path)| LocalModelAssetStatus {
            name,
            exists: path.exists(),
            path,
        })
        .collect())
}

async fn ensure_model_file_impl(
    repo_id: &str,
    file_path: &str,
    base_url: &str,
    home_override: Option<&std::path::Path>,
) -> Result<PathBuf> {
    let home_dir = match home_override {
        Some(p) => p.to_path_buf(),
        None => vera_home_dir()?,
    };
    let models_dir = home_dir.join("models").join(repo_id);
    let target_path = models_dir.join(file_path);

    if target_path.exists() {
        return Ok(target_path);
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let url = format!("{}/{}/resolve/main/{}", base_url, repo_id, file_path);
    eprintln!("Downloading {}...", url);

    let client = Client::new();
    let res = client.get(&url).send().await?.error_for_status()?;
    let total_size = res.content_length();

    let temp_path = target_path.with_extension(format!("part.{}", std::process::id()));
    let mut file = File::create(&temp_path).await?;
    let mut stream = res.bytes_stream();
    let mut downloaded = 0;

    let download_result: Result<()> = async {
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| anyhow::anyhow!("Download error: {}", e))?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            if let Some(total) = total_size {
                eprint!(
                    "\rProgress: {} MB / {} MB",
                    downloaded / 1_000_000,
                    total / 1_000_000
                );
            } else {
                eprint!("\rProgress: {} MB", downloaded / 1_000_000);
            }
        }
        file.flush().await?;
        file.sync_all().await?;
        eprintln!("\nDownload complete: {}", file_path);

        if let Err(e) = fs::rename(&temp_path, &target_path).await {
            if target_path.exists() {
                // Another process won the race
                let _ = fs::remove_file(&temp_path).await;
            } else {
                return Err(e.into());
            }
        }
        Ok(())
    }
    .await;

    if let Err(e) = download_result {
        let _ = fs::remove_file(&temp_path).await;
        return Err(e).context(format!(
            "Expected path: {}. Hint: check network connection or manually place model at {}",
            target_path.display(),
            target_path.display()
        ));
    }

    Ok(target_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;

    #[tokio::test]
    async fn test_download_failure_cleanup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let home = temp_dir.path().join(".vera");

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                // Return a valid HTTP response header but truncate the body
                let response = "HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\nPartialData";
                let _ = stream.write_all(response.as_bytes());
                // abruptly close the connection
            }
        });

        let base_url = format!("http://127.0.0.1:{}", port);

        let res =
            ensure_model_file_impl("test-repo", "test-file.bin", &base_url, Some(&home)).await;

        assert!(res.is_err(), "Download should fail due to truncated stream");

        let target_dir = home.join("models").join("test-repo");
        let part_file = target_dir
            .join("test-file.bin")
            .with_extension(format!("part.{}", std::process::id()));
        assert!(
            !part_file.exists(),
            "Partial file should be cleaned up on failure"
        );
    }
}

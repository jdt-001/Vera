use anyhow::{Context, Result};
use futures::StreamExt;
use reqwest::Client;
use std::path::PathBuf;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;

const HUB_URL: &str = "https://huggingface.co";

/// Download a file from HuggingFace Hub using atomic writes.
pub async fn ensure_model_file(repo_id: &str, file_path: &str) -> Result<PathBuf> {
    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    let models_dir = home_dir.join(".vera").join("models").join(repo_id);
    let target_path = models_dir.join(file_path);

    if target_path.exists() {
        return Ok(target_path);
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let url = format!("{}/{}/resolve/main/{}", HUB_URL, repo_id, file_path);
    eprintln!("Downloading {}...", url);

    let client = Client::new();
    let res = client.get(&url).send().await?.error_for_status()?;
    let total_size = res.content_length();

    let temp_path = target_path.with_extension("part");
    let mut file = File::create(&temp_path).await?;
    let mut stream = res.bytes_stream();
    let mut downloaded = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
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

    fs::rename(&temp_path, &target_path).await?;

    Ok(target_path)
}

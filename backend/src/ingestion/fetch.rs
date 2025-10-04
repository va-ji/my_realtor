//! Fetch functions - retrieve raw data from various sources

use crate::ingestion::types::RawData;
use crate::ingestion::utils::{extract_csv_from_zip, http_get};
use anyhow::Result;
use std::fs;
use std::path::Path;
use tracing::info;

/// Fetch NSW property sales data (ZIP containing CSV)
pub async fn fetch_nsw_sales(url: &str, temp_dir: &Path) -> Result<RawData> {
    info!("Fetching NSW sales data from {}", url);

    // Download ZIP file
    let zip_bytes = http_get(url).await?;

    // Save to temp directory
    let zip_path = temp_dir.join("nsw_sales.zip");
    fs::write(&zip_path, zip_bytes)?;
    info!("Saved ZIP to {:?}", zip_path);

    // Extract CSV from ZIP
    let csv_path = extract_csv_from_zip(&zip_path)?;

    Ok(RawData::File(csv_path))
}

/// Fetch NSW rental bond data (XLSX)
pub async fn fetch_nsw_rentals(url: &str) -> Result<RawData> {
    info!("Fetching NSW rental bond data from {}", url);

    let bytes = http_get(url).await?;

    Ok(RawData::Bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    #[ignore] // Ignore by default since it hits real API
    async fn test_fetch_nsw_sales() {
        let temp = tempdir().unwrap();
        let url = "https://nswpropertysalesdata.com/data/archive.zip";

        let result = fetch_nsw_sales(url, temp.path()).await;
        assert!(result.is_ok());

        let raw_data = result.unwrap();
        match raw_data {
            RawData::File(path) => {
                assert!(path.exists());
                assert!(path.extension().unwrap() == "csv");
            }
            _ => panic!("Expected File variant"),
        }
    }
}

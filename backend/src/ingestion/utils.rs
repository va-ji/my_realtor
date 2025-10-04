//! Utility functions for common operations

use anyhow::Result;
use reqwest::Client;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tracing::info;

/// Download a file via HTTP
pub async fn http_get(url: &str) -> Result<Vec<u8>> {
    info!("Downloading from {}", url);
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 5 min timeout
        .build()?;

    let response = client.get(url).send().await?;
    let status = response.status();

    if !status.is_success() {
        return Err(anyhow::anyhow!("HTTP request failed: {}", status));
    }

    let bytes = response.bytes().await?;
    info!("Downloaded {} bytes", bytes.len());
    Ok(bytes.to_vec())
}

/// Extract the first CSV file from a ZIP archive
pub fn extract_csv_from_zip(zip_path: &Path) -> Result<PathBuf> {
    info!("Extracting CSV from {:?}", zip_path);

    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // Find first CSV file
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let filename = file.name().to_string();

        if filename.ends_with(".csv") {
            info!("Found CSV file: {}", filename);

            // Extract to same directory as ZIP
            let output_dir = zip_path.parent().unwrap();
            let output_path = output_dir.join(&filename);

            let mut output_file = fs::File::create(&output_path)?;
            io::copy(&mut file, &mut output_file)?;

            info!("Extracted to {:?}", output_path);
            return Ok(output_path);
        }
    }

    Err(anyhow::anyhow!("No CSV file found in ZIP archive"))
}

/// Parse property type from NSW "Nature of property" field
pub fn parse_nsw_property_type(nature: &str) -> crate::ingestion::types::PropertyType {
    let lower = nature.to_lowercase();

    if lower.contains("house") || lower.contains("dwelling") {
        crate::ingestion::types::PropertyType::House
    } else if lower.contains("unit") || lower.contains("apartment") || lower.contains("flat") {
        crate::ingestion::types::PropertyType::Unit
    } else if lower.contains("townhouse") || lower.contains("terrace") {
        crate::ingestion::types::PropertyType::Townhouse
    } else if lower.contains("vacant") || lower.contains("land") {
        crate::ingestion::types::PropertyType::VacantLand
    } else if lower.contains("commercial") || lower.contains("retail") || lower.contains("office")
    {
        crate::ingestion::types::PropertyType::Commercial
    } else {
        crate::ingestion::types::PropertyType::Other
    }
}

/// Format NSW address from components
pub fn format_nsw_address(
    unit: Option<&str>,
    house_number: Option<&str>,
    street_name: &str,
) -> String {
    let mut parts = Vec::new();

    if let Some(u) = unit {
        if !u.trim().is_empty() {
            parts.push(u.trim().to_string());
        }
    }

    if let Some(h) = house_number {
        if !h.trim().is_empty() {
            parts.push(h.trim().to_string());
        }
    }

    parts.push(street_name.trim().to_string());

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_property_type() {
        use crate::ingestion::types::PropertyType;

        assert_eq!(
            parse_nsw_property_type("Residential - House"),
            PropertyType::House
        );
        assert_eq!(parse_nsw_property_type("Unit"), PropertyType::Unit);
        assert_eq!(
            parse_nsw_property_type("Townhouse"),
            PropertyType::Townhouse
        );
        assert_eq!(
            parse_nsw_property_type("Vacant Land"),
            PropertyType::VacantLand
        );
    }

    #[test]
    fn test_format_address() {
        assert_eq!(
            format_nsw_address(None, Some("10"), "Smith Street"),
            "10 Smith Street"
        );

        assert_eq!(
            format_nsw_address(Some("2"), Some("10"), "Smith Street"),
            "2 10 Smith Street"
        );

        assert_eq!(
            format_nsw_address(None, None, "Smith Street"),
            "Smith Street"
        );
    }
}

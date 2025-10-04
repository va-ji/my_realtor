//! Parse functions - transform raw data into PropertyRecord structs

use crate::ingestion::types::{
    DataQuality, PropertyRecord, RawData, RentalMedian, SourceMetadata, State,
};
use crate::ingestion::utils::{format_nsw_address, parse_nsw_property_type};
use anyhow::Result;
use calamine::{open_workbook_auto_from_rs, Reader, Data};
use chrono::{NaiveDate, Utc};
use csv;
use serde::Deserialize;
use std::io::Cursor;
use tracing::{info, warn};

/// NSW Sales CSV row structure
#[derive(Debug, Deserialize)]
struct NswSalesRow {
    #[serde(rename = "Property ID")]
    property_id: String,

    #[serde(rename = "Property unit number")]
    property_unit_number: Option<String>,

    #[serde(rename = "Property house number")]
    property_house_number: Option<String>,

    #[serde(rename = "Property street name")]
    property_street_name: String,

    #[serde(rename = "Property locality")]
    property_locality: String,

    #[serde(rename = "Property post code")]
    property_post_code: String,

    #[serde(rename = "Purchase price")]
    purchase_price: String, // CSV has it as string with $ and commas

    #[serde(rename = "Settlement date")]
    settlement_date: String, // Format: DD/MM/YYYY

    #[serde(rename = "Contract date")]
    contract_date: Option<String>,

    #[serde(rename = "Nature of property")]
    nature_of_property: String,
}

/// Parse NSW sales CSV into PropertyRecord structs
pub async fn parse_nsw_sales(raw: RawData, source_id: String) -> Result<Vec<PropertyRecord>> {
    let csv_path = raw.as_file_path()?;
    info!("Parsing NSW sales CSV from {:?}", csv_path);

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(csv_path)?;

    let mut records = Vec::new();
    let mut parse_errors = 0;

    for (idx, result) in reader.deserialize::<NswSalesRow>().enumerate() {
        match result {
            Ok(row) => {
                match parse_nsw_row(row, &source_id) {
                    Ok(record) => records.push(record),
                    Err(e) => {
                        parse_errors += 1;
                        if parse_errors <= 10 {
                            // Only log first 10 errors
                            warn!("Failed to parse row {}: {}", idx, e);
                        }
                    }
                }
            }
            Err(e) => {
                parse_errors += 1;
                if parse_errors <= 10 {
                    warn!("Failed to deserialize row {}: {}", idx, e);
                }
            }
        }
    }

    info!(
        "Parsed {} records from NSW sales CSV ({} errors)",
        records.len(),
        parse_errors
    );

    Ok(records)
}

fn parse_nsw_row(row: NswSalesRow, source_id: &str) -> Result<PropertyRecord> {
    // Parse price (remove $ and commas)
    let price_str = row
        .purchase_price
        .replace("$", "")
        .replace(",", "")
        .trim()
        .to_string();
    let sale_price = price_str.parse::<i32>().ok();

    // Parse settlement date (DD/MM/YYYY)
    let sale_date = parse_date(&row.settlement_date);

    // Format address
    let address = format_nsw_address(
        row.property_unit_number.as_deref(),
        row.property_house_number.as_deref(),
        &row.property_street_name,
    );

    // Parse property type
    let property_type = parse_nsw_property_type(&row.nature_of_property);

    Ok(PropertyRecord {
        external_id: Some(row.property_id),
        address,
        suburb: row.property_locality,
        state: State::NSW,
        postcode: Some(row.property_post_code),
        property_type,
        bedrooms: None, // Will be estimated in enrichment
        bathrooms: None,
        land_area_sqm: None,
        sale_price,
        sale_date,
        weekly_rent: None, // Will be matched in enrichment
        rental_yield: None,
        latitude: None,
        longitude: None,
        source_metadata: SourceMetadata {
            source_id: source_id.to_string(),
            data_quality: DataQuality::Individual,
            fetched_at: Utc::now(),
            is_rental_estimated: false,
            confidence_score: 0.9, // High confidence for government data
        },
    })
}

/// Parse date string in DD/MM/YYYY format
fn parse_date(date_str: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(date_str, "%d/%m/%Y").ok()
}

/// Parse NSW rental bond XLSX into RentalMedian structs
pub async fn parse_nsw_rentals(raw: RawData, period: NaiveDate) -> Result<Vec<RentalMedian>> {
    let bytes = raw.as_bytes()?;
    info!("Parsing NSW rental bond XLSX ({} bytes)", bytes.len());

    let cursor = Cursor::new(bytes);
    let mut workbook = open_workbook_auto_from_rs(cursor)?;

    // NSW rental bond files typically have data in first sheet
    let sheet_names = workbook.sheet_names();
    if sheet_names.is_empty() {
        return Err(anyhow::anyhow!("No sheets found in workbook"));
    }

    let sheet_name = &sheet_names[0];
    info!("Reading sheet: {}", sheet_name);

    let range = workbook.worksheet_range(sheet_name)?;

    let mut rentals = Vec::new();

    // Skip header row (assuming first row is headers)
    for (idx, row) in range.rows().enumerate().skip(1) {
        if row.len() < 4 {
            continue; // Skip incomplete rows
        }

        // Expected columns: Postcode, Suburb, Bedrooms, Median Weekly Rent
        // Note: Actual format may vary, adjust as needed
        let postcode = match &row[0] {
            Data::String(s) => s.trim().to_string(),
            Data::Int(i) => i.to_string(),
            Data::Float(f) => format!("{:.0}", f),
            _ => continue,
        };

        let suburb = match &row[1] {
            Data::String(s) => Some(s.trim().to_string()),
            _ => None,
        };

        let bedrooms = match &row[2] {
            Data::Int(i) => *i as i32,
            Data::Float(f) => *f as i32,
            Data::String(s) => match s.trim().parse::<i32>() {
                Ok(b) => b,
                Err(_) => continue,
            },
            _ => continue,
        };

        let median_rent = match &row[3] {
            Data::Int(i) => *i as i32,
            Data::Float(f) => *f as i32,
            Data::String(s) => {
                let clean = s.replace("$", "").replace(",", "").trim().to_string();
                match clean.parse::<i32>() {
                    Ok(r) => r,
                    Err(_) => continue,
                }
            }
            _ => continue,
        };

        rentals.push(RentalMedian {
            state: State::NSW,
            postcode,
            suburb,
            bedrooms,
            median_weekly_rent: median_rent,
            sample_size: None,
            period,
        });
    }

    info!("Parsed {} rental medians from XLSX", rentals.len());

    Ok(rentals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date() {
        assert_eq!(
            parse_date("25/12/2023"),
            Some(NaiveDate::from_ymd_opt(2023, 12, 25).unwrap())
        );

        assert_eq!(
            parse_date("01/01/2024"),
            Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap())
        );

        assert_eq!(parse_date("invalid"), None);
    }

    #[test]
    fn test_parse_nsw_row() {
        let row = NswSalesRow {
            property_id: "12345".to_string(),
            property_unit_number: None,
            property_house_number: Some("10".to_string()),
            property_street_name: "Smith Street".to_string(),
            property_locality: "Sydney".to_string(),
            property_post_code: "2000".to_string(),
            purchase_price: "$750,000".to_string(),
            settlement_date: "15/06/2023".to_string(),
            contract_date: None,
            nature_of_property: "Residential - House".to_string(),
        };

        let record = parse_nsw_row(row, "nsw_sales").unwrap();

        assert_eq!(record.address, "10 Smith Street");
        assert_eq!(record.suburb, "Sydney");
        assert_eq!(record.state, State::NSW);
        assert_eq!(record.postcode, Some("2000".to_string()));
        assert_eq!(record.sale_price, Some(750_000));
        assert_eq!(record.property_type, PropertyType::House);
    }
}

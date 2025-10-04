//! Core data types for the ingestion pipeline
//! Pure data structures with no behavior

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::path::PathBuf;

/// Raw data from various sources - tagged unions
#[derive(Debug)]
pub enum RawData {
    File(PathBuf),
    Bytes(Vec<u8>),
    Json(serde_json::Value),
    Csv(String),
}

impl RawData {
    pub fn as_file_path(&self) -> anyhow::Result<&PathBuf> {
        match self {
            RawData::File(path) => Ok(path),
            _ => Err(anyhow::anyhow!("Expected File, got {:?}", self)),
        }
    }

    pub fn as_bytes(&self) -> anyhow::Result<&[u8]> {
        match self {
            RawData::Bytes(bytes) => Ok(bytes),
            _ => Err(anyhow::anyhow!("Expected Bytes, got {:?}", self)),
        }
    }

    pub fn as_json(&self) -> anyhow::Result<&serde_json::Value> {
        match self {
            RawData::Json(json) => Ok(json),
            _ => Err(anyhow::anyhow!("Expected Json, got {:?}", self)),
        }
    }
}

/// Australian states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "state_enum", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum State {
    NSW,
    VIC,
    QLD,
    WA,
    SA,
    TAS,
    ACT,
    NT,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::NSW => write!(f, "NSW"),
            State::VIC => write!(f, "VIC"),
            State::QLD => write!(f, "QLD"),
            State::WA => write!(f, "WA"),
            State::SA => write!(f, "SA"),
            State::TAS => write!(f, "TAS"),
            State::ACT => write!(f, "ACT"),
            State::NT => write!(f, "NT"),
        }
    }
}

/// Property types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "property_type_enum", rename_all = "snake_case")]
pub enum PropertyType {
    House,
    Unit,
    Townhouse,
    #[sqlx(rename = "vacant_land")]
    VacantLand,
    Commercial,
    Other,
}

impl std::fmt::Display for PropertyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PropertyType::House => write!(f, "house"),
            PropertyType::Unit => write!(f, "unit"),
            PropertyType::Townhouse => write!(f, "townhouse"),
            PropertyType::VacantLand => write!(f, "vacant_land"),
            PropertyType::Commercial => write!(f, "commercial"),
            PropertyType::Other => write!(f, "other"),
        }
    }
}

/// Data quality levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "data_quality_enum", rename_all = "snake_case")]
pub enum DataQuality {
    Individual,  // Real property records (NSW, WA)
    Aggregated,  // Suburb/postcode medians (VIC, SA)
    Estimated,   // Calculated/derived data
    Listing,     // Current market listings (Domain API)
}

impl DataQuality {
    /// Quality score for conflict resolution (higher = better)
    pub fn score(&self) -> i32 {
        match self {
            DataQuality::Individual => 100,
            DataQuality::Listing => 90,
            DataQuality::Aggregated => 50,
            DataQuality::Estimated => 25,
        }
    }
}

/// Property record - pure data, no behavior
#[derive(Debug, Clone)]
pub struct PropertyRecord {
    // Core identification
    pub external_id: Option<String>,
    pub address: String,
    pub suburb: String,
    pub state: State,
    pub postcode: Option<String>,

    // Property attributes
    pub property_type: PropertyType,
    pub bedrooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub land_area_sqm: Option<Decimal>,

    // Financial data
    pub sale_price: Option<i32>,
    pub sale_date: Option<NaiveDate>,
    pub weekly_rent: Option<i32>,
    pub rental_yield: Option<Decimal>,

    // Geolocation
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,

    // Data provenance
    pub source_metadata: SourceMetadata,
}

/// Metadata about where this record came from
#[derive(Debug, Clone)]
pub struct SourceMetadata {
    pub source_id: String,
    pub data_quality: DataQuality,
    pub fetched_at: DateTime<Utc>,
    pub is_rental_estimated: bool,
    pub confidence_score: f32, // 0.0-1.0
}

/// Rental median data (for matching)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RentalMedian {
    pub state: State,
    pub postcode: String,
    pub suburb: Option<String>,
    pub bedrooms: i32,
    pub median_weekly_rent: i32,
    pub sample_size: Option<i32>,
    pub period: NaiveDate,
}

/// Database row from properties table
#[derive(Debug, sqlx::FromRow)]
pub struct PropertyRow {
    pub id: i32,
    pub address: String,
    pub suburb: String,
    pub state: State,
    pub postcode: Option<String>,
    pub bedrooms: Option<i32>,
    pub price: Option<i32>,
    pub weekly_rent: Option<i32>,
    pub property_type: Option<PropertyType>,
    pub data_source: Option<String>,
    pub data_quality: Option<DataQuality>,
    pub confidence_score: Option<Decimal>,
    pub external_id: Option<String>,
}

impl PropertyRow {
    /// Calculate quality score for conflict resolution
    pub fn quality_score(&self) -> f32 {
        let base_score = self
            .data_quality
            .map(|q| q.score() as f32)
            .unwrap_or(0.0);

        let confidence = self
            .confidence_score
            .map(|c| c.to_string().parse::<f32>().unwrap_or(1.0))
            .unwrap_or(1.0);

        base_score * confidence
    }
}

/// Write operation statistics
#[derive(Debug, Default, Clone)]
pub struct WriteStats {
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: usize,
}

impl std::fmt::Display for WriteStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "inserted: {}, updated: {}, skipped: {}, errors: {}",
            self.inserted, self.updated, self.skipped, self.errors
        )
    }
}

/// Ingestion run record
#[derive(Debug, sqlx::FromRow)]
pub struct IngestionRun {
    pub id: i32,
    pub source_id: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub records_fetched: i32,
    pub records_inserted: i32,
    pub records_updated: i32,
    pub records_skipped: i32,
    pub error_message: Option<String>,
}

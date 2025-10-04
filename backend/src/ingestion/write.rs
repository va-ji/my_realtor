//! Write functions - persist data to PostgreSQL with conflict resolution

use crate::ingestion::types::{PropertyRecord, PropertyRow, RentalMedian, WriteStats};
use anyhow::Result;
use sqlx::PgPool;
use tracing::{debug, info, warn};

/// Write property records to database with intelligent conflict resolution
pub async fn write_properties(db: &PgPool, records: Vec<PropertyRecord>) -> Result<WriteStats> {
    info!("Writing {} property records to database", records.len());

    let mut stats = WriteStats::default();

    for record in records {
        match write_single_property(db, &record).await {
            Ok(inserted) => {
                if inserted {
                    stats.inserted += 1;
                } else {
                    stats.updated += 1;
                }
            }
            Err(e) => {
                warn!("Failed to write property {}: {}", record.address, e);
                stats.errors += 1;
            }
        }
    }

    info!("Write complete: {}", stats);

    Ok(stats)
}

/// Write a single property record with conflict resolution
/// Returns true if inserted, false if updated
async fn write_single_property(db: &PgPool, record: &PropertyRecord) -> Result<bool> {
    // Check if property exists (by address + postcode or external_id)
    let existing = find_existing_property(db, record).await?;

    match existing {
        None => {
            // Insert new property
            insert_property(db, record).await?;
            debug!("Inserted new property: {}", record.address);
            Ok(true)
        }
        Some(existing) => {
            // Decide if we should update based on data quality
            if should_replace(&existing, record) {
                update_property(db, existing.id, record).await?;
                debug!("Updated property: {} (id: {})", record.address, existing.id);
                Ok(false)
            } else {
                debug!(
                    "Skipped property: {} (existing data is better quality)",
                    record.address
                );
                Ok(false)
            }
        }
    }
}

/// Find existing property by external_id or address+postcode
async fn find_existing_property(
    db: &PgPool,
    record: &PropertyRecord,
) -> Result<Option<PropertyRow>> {
    // First try to find by external_id (most reliable)
    if let Some(ref external_id) = record.external_id {
        let result = sqlx::query_as::<_, PropertyRow>(
            "SELECT * FROM properties WHERE external_id = $1 AND state = $2",
        )
        .bind(external_id)
        .bind(&record.state)
        .fetch_optional(db)
        .await?;

        if result.is_some() {
            return Ok(result);
        }
    }

    // Fallback: find by address + postcode
    if let Some(ref postcode) = record.postcode {
        let result = sqlx::query_as::<_, PropertyRow>(
            "SELECT * FROM properties WHERE address = $1 AND postcode = $2 AND state = $3",
        )
        .bind(&record.address)
        .bind(postcode)
        .bind(&record.state)
        .fetch_optional(db)
        .await?;

        return Ok(result);
    }

    Ok(None)
}

/// Determine if new record should replace existing one
fn should_replace(existing: &PropertyRow, new: &PropertyRecord) -> bool {
    let existing_score = existing.quality_score();
    let new_score = new.source_metadata.data_quality.score() as f32
        * new.source_metadata.confidence_score;

    // Replace if new data is significantly better (10% threshold to avoid churn)
    new_score > existing_score * 1.1
}

/// Insert a new property record
async fn insert_property(db: &PgPool, record: &PropertyRecord) -> Result<i32> {
    let id = sqlx::query_scalar::<_, i32>(
        r#"
        INSERT INTO properties (
            address, suburb, state, postcode, bedrooms, bathrooms, property_type,
            price, weekly_rent, rental_yield, latitude, longitude, sale_date,
            data_source, data_quality, is_rental_estimated, confidence_score,
            external_id, land_area_sqm, last_updated
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
            $14, $15, $16, $17, $18, $19, NOW()
        )
        RETURNING id
        "#,
    )
    .bind(&record.address)
    .bind(&record.suburb)
    .bind(&record.state)
    .bind(&record.postcode)
    .bind(record.bedrooms)
    .bind(record.bathrooms)
    .bind(&record.property_type)
    .bind(record.sale_price)
    .bind(record.weekly_rent)
    .bind(record.rental_yield)
    .bind(record.latitude)
    .bind(record.longitude)
    .bind(record.sale_date)
    .bind(&record.source_metadata.source_id)
    .bind(&record.source_metadata.data_quality)
    .bind(record.source_metadata.is_rental_estimated)
    .bind(record.source_metadata.confidence_score)
    .bind(&record.external_id)
    .bind(record.land_area_sqm)
    .fetch_one(db)
    .await?;

    // Also insert into sales history if we have sale data
    if let (Some(price), Some(date)) = (record.sale_price, record.sale_date) {
        insert_sale_history(db, id, price, date, &record.source_metadata.source_id).await?;
    }

    Ok(id)
}

/// Update an existing property record
async fn update_property(db: &PgPool, id: i32, record: &PropertyRecord) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE properties SET
            address = $1, suburb = $2, state = $3, postcode = $4,
            bedrooms = $5, bathrooms = $6, property_type = $7,
            price = $8, weekly_rent = $9, rental_yield = $10,
            latitude = $11, longitude = $12, sale_date = $13,
            data_source = $14, data_quality = $15, is_rental_estimated = $16,
            confidence_score = $17, external_id = $18, land_area_sqm = $19,
            last_updated = NOW()
        WHERE id = $20
        "#,
    )
    .bind(&record.address)
    .bind(&record.suburb)
    .bind(&record.state)
    .bind(&record.postcode)
    .bind(record.bedrooms)
    .bind(record.bathrooms)
    .bind(&record.property_type)
    .bind(record.sale_price)
    .bind(record.weekly_rent)
    .bind(record.rental_yield)
    .bind(record.latitude)
    .bind(record.longitude)
    .bind(record.sale_date)
    .bind(&record.source_metadata.source_id)
    .bind(&record.source_metadata.data_quality)
    .bind(record.source_metadata.is_rental_estimated)
    .bind(record.source_metadata.confidence_score)
    .bind(&record.external_id)
    .bind(record.land_area_sqm)
    .bind(id)
    .execute(db)
    .await?;

    // Also insert into sales history if we have sale data
    if let (Some(price), Some(date)) = (record.sale_price, record.sale_date) {
        insert_sale_history(db, id, price, date, &record.source_metadata.source_id).await?;
    }

    Ok(())
}

/// Insert a sale into sales history
async fn insert_sale_history(
    db: &PgPool,
    property_id: i32,
    price: i32,
    sale_date: chrono::NaiveDate,
    data_source: &str,
) -> Result<()> {
    // Only insert if this sale doesn't already exist
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM sales_history
            WHERE property_id = $1 AND sale_date = $2 AND sale_price = $3
        )
        "#,
    )
    .bind(property_id)
    .bind(sale_date)
    .bind(price)
    .fetch_one(db)
    .await?;

    if !exists {
        sqlx::query(
            r#"
            INSERT INTO sales_history (property_id, sale_price, sale_date, data_source)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(property_id)
        .bind(price)
        .bind(sale_date)
        .bind(data_source)
        .execute(db)
        .await?;

        debug!("Inserted sale history: property_id={}, price={}, date={}", property_id, price, sale_date);
    }

    Ok(())
}

/// Write rental medians to database
pub async fn write_rental_medians(db: &PgPool, rentals: Vec<RentalMedian>) -> Result<WriteStats> {
    info!("Writing {} rental medians to database", rentals.len());

    let mut stats = WriteStats::default();

    for rental in rentals {
        match insert_rental_median(db, &rental).await {
            Ok(inserted) => {
                if inserted {
                    stats.inserted += 1;
                } else {
                    stats.skipped += 1; // Already exists
                }
            }
            Err(e) => {
                warn!(
                    "Failed to write rental median for {} ({}br): {}",
                    rental.postcode, rental.bedrooms, e
                );
                stats.errors += 1;
            }
        }
    }

    info!("Rental medians write complete: {}", stats);

    Ok(stats)
}

/// Insert a rental median (with conflict handling via UNIQUE constraint)
async fn insert_rental_median(db: &PgPool, rental: &RentalMedian) -> Result<bool> {
    let result = sqlx::query(
        r#"
        INSERT INTO rental_medians (
            state, postcode, suburb, bedrooms, median_weekly_rent,
            sample_size, data_source, period
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (state, postcode, bedrooms, period, data_source) DO NOTHING
        "#,
    )
    .bind(&rental.state)
    .bind(&rental.postcode)
    .bind(&rental.suburb)
    .bind(rental.bedrooms)
    .bind(rental.median_weekly_rent)
    .bind(rental.sample_size)
    .bind("nsw_rentals") // data_source
    .bind(rental.period)
    .execute(db)
    .await?;

    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion::types::{DataQuality, PropertyType, SourceMetadata, State};
    use chrono::Utc;

    fn mock_record() -> PropertyRecord {
        PropertyRecord {
            external_id: Some("test-123".to_string()),
            address: "10 Test St".to_string(),
            suburb: "Testville".to_string(),
            state: State::NSW,
            postcode: Some("2000".to_string()),
            property_type: PropertyType::House,
            bedrooms: Some(3),
            bathrooms: Some(2),
            land_area_sqm: None,
            sale_price: Some(800_000),
            sale_date: Some(chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()),
            weekly_rent: Some(600),
            rental_yield: Some(rust_decimal::Decimal::new(390, 2)), // 3.90%
            latitude: None,
            longitude: None,
            source_metadata: SourceMetadata {
                source_id: "nsw_sales".to_string(),
                data_quality: DataQuality::Individual,
                fetched_at: Utc::now(),
                is_rental_estimated: true,
                confidence_score: 0.8,
            },
        }
    }

    #[test]
    fn test_should_replace_better_quality() {
        let existing = PropertyRow {
            id: 1,
            address: "10 Test St".to_string(),
            suburb: "Testville".to_string(),
            state: State::NSW,
            postcode: Some("2000".to_string()),
            bedrooms: Some(3),
            price: Some(800_000),
            weekly_rent: Some(600),
            property_type: Some(PropertyType::House),
            data_source: Some("old_source".to_string()),
            data_quality: Some(DataQuality::Estimated), // Low quality (25 score)
            confidence_score: Some(rust_decimal::Decimal::new(5, 1)), // 0.5
            external_id: Some("test-123".to_string()),
        };

        let new = mock_record(); // Individual quality (100 score) * 0.8 confidence = 80

        // Existing: 25 * 0.5 = 12.5
        // New: 100 * 0.8 = 80
        // 80 > 12.5 * 1.1 (13.75) -> should replace
        assert!(should_replace(&existing, &new));
    }

    #[test]
    fn test_should_not_replace_similar_quality() {
        let existing = PropertyRow {
            id: 1,
            address: "10 Test St".to_string(),
            suburb: "Testville".to_string(),
            state: State::NSW,
            postcode: Some("2000".to_string()),
            bedrooms: Some(3),
            price: Some(800_000),
            weekly_rent: Some(600),
            property_type: Some(PropertyType::House),
            data_source: Some("old_source".to_string()),
            data_quality: Some(DataQuality::Individual), // Same quality
            confidence_score: Some(rust_decimal::Decimal::new(85, 2)), // 0.85
            external_id: Some("test-123".to_string()),
        };

        let new = mock_record(); // Individual quality * 0.8 confidence = 80

        // Existing: 100 * 0.85 = 85
        // New: 100 * 0.8 = 80
        // 80 < 85 * 1.1 (93.5) -> should NOT replace
        assert!(!should_replace(&existing, &new));
    }
}

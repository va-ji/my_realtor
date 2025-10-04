//! Enrichment functions - add calculated/matched data to property records

use crate::ingestion::types::{PropertyRecord, PropertyType, RentalMedian, SourceMetadata};
use anyhow::Result;
use rust_decimal::Decimal;
use sqlx::PgPool;
use tracing::{debug, info};

/// Estimate bedrooms based on property characteristics
/// Pure function - no side effects
pub fn estimate_bedrooms(record: PropertyRecord) -> PropertyRecord {
    if record.bedrooms.is_some() {
        return record; // Already has bedrooms
    }

    let estimated = match (&record.property_type, record.sale_price) {
        // Units - generally smaller
        (PropertyType::Unit, Some(price)) if price < 400_000 => 1,
        (PropertyType::Unit, Some(price)) if price < 600_000 => 2,
        (PropertyType::Unit, _) => 2, // Default for units

        // Houses - generally larger
        (PropertyType::House, Some(price)) if price < 500_000 => 2,
        (PropertyType::House, Some(price)) if price < 800_000 => 3,
        (PropertyType::House, Some(price)) if price < 1_200_000 => 4,
        (PropertyType::House, _) => 4, // Default for houses

        // Townhouses - middle ground
        (PropertyType::Townhouse, Some(price)) if price < 600_000 => 2,
        (PropertyType::Townhouse, _) => 3,

        // Vacant land - no bedrooms
        (PropertyType::VacantLand, _) => 0,

        // Default fallback
        _ => 3,
    };

    debug!(
        "Estimated {} bedrooms for {} (type: {:?}, price: {:?})",
        estimated, record.address, record.property_type, record.sale_price
    );

    PropertyRecord {
        bedrooms: Some(estimated),
        source_metadata: SourceMetadata {
            confidence_score: record.source_metadata.confidence_score * 0.7, // Reduce confidence
            ..record.source_metadata
        },
        ..record
    }
}

/// Match property to rental data by postcode + bedrooms
/// Requires database access to query rental_medians table
pub async fn match_rental(record: PropertyRecord, db: &PgPool) -> Result<PropertyRecord> {
    if record.weekly_rent.is_some() {
        return Ok(record); // Already has rental data
    }

    // Need postcode and bedrooms to match
    let (postcode, bedrooms) = match (&record.postcode, record.bedrooms) {
        (Some(pc), Some(br)) => (pc, br),
        _ => {
            debug!(
                "Cannot match rental for {} - missing postcode or bedrooms",
                record.address
            );
            return Ok(record);
        }
    };

    // Query most recent rental median for this postcode + bedroom combo
    let rental = sqlx::query_as::<_, RentalMedian>(
        r#"
        SELECT state, postcode, suburb, bedrooms, median_weekly_rent, sample_size, period
        FROM rental_medians
        WHERE state = $1 AND postcode = $2 AND bedrooms = $3
        ORDER BY period DESC
        LIMIT 1
        "#,
    )
    .bind(&record.state)
    .bind(postcode)
    .bind(bedrooms)
    .fetch_optional(db)
    .await?;

    match rental {
        Some(rental) => {
            debug!(
                "Matched rental for {}: ${}/week (postcode: {}, bedrooms: {})",
                record.address, rental.median_weekly_rent, postcode, bedrooms
            );

            Ok(PropertyRecord {
                weekly_rent: Some(rental.median_weekly_rent),
                source_metadata: SourceMetadata {
                    is_rental_estimated: true,
                    confidence_score: record.source_metadata.confidence_score * 0.85,
                    ..record.source_metadata
                },
                ..record
            })
        }
        None => {
            debug!(
                "No rental data found for {} (postcode: {}, bedrooms: {})",
                record.address, postcode, bedrooms
            );
            Ok(record)
        }
    }
}

/// Calculate rental yield based on price and rent
/// Pure function - no side effects
pub fn calculate_yield(record: PropertyRecord) -> PropertyRecord {
    let yield_pct = match (record.sale_price, record.weekly_rent) {
        (Some(price), Some(rent)) if price > 0 => {
            // Formula: (weekly_rent * 52 / price) * 100
            let annual_rent = Decimal::from(rent) * Decimal::from(52);
            let price_decimal = Decimal::from(price);
            Some((annual_rent / price_decimal) * Decimal::from(100))
        }
        _ => None,
    };

    if let Some(yield_val) = yield_pct {
        debug!(
            "Calculated yield for {}: {:.2}%",
            record.address, yield_val
        );
    }

    PropertyRecord {
        rental_yield: yield_pct,
        ..record
    }
}

/// Run all enrichment functions in sequence
/// This is a convenience function that composes the enrichers
pub async fn enrich_all(
    records: Vec<PropertyRecord>,
    db: &PgPool,
) -> Result<Vec<PropertyRecord>> {
    info!("Enriching {} records", records.len());

    let mut enriched = Vec::new();

    for record in records {
        // Step 1: Estimate bedrooms if missing
        let record = estimate_bedrooms(record);

        // Step 2: Match rental data
        let record = match_rental(record, db).await?;

        // Step 3: Calculate yield
        let record = calculate_yield(record);

        enriched.push(record);
    }

    info!("Enrichment complete: {} records", enriched.len());

    Ok(enriched)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion::types::{DataQuality, SourceMetadata, State};
    use chrono::Utc;

    fn mock_record() -> PropertyRecord {
        PropertyRecord {
            external_id: Some("test-123".to_string()),
            address: "10 Smith St".to_string(),
            suburb: "Sydney".to_string(),
            state: State::NSW,
            postcode: Some("2000".to_string()),
            property_type: PropertyType::House,
            bedrooms: None,
            bathrooms: None,
            land_area_sqm: None,
            sale_price: Some(800_000),
            sale_date: None,
            weekly_rent: None,
            rental_yield: None,
            latitude: None,
            longitude: None,
            source_metadata: SourceMetadata {
                source_id: "test".to_string(),
                data_quality: DataQuality::Individual,
                fetched_at: Utc::now(),
                is_rental_estimated: false,
                confidence_score: 1.0,
            },
        }
    }

    #[test]
    fn test_estimate_bedrooms_house() {
        let record = mock_record();
        let enriched = estimate_bedrooms(record);

        assert_eq!(enriched.bedrooms, Some(3)); // $800k house = 3br
        assert!(enriched.source_metadata.confidence_score < 1.0); // Confidence reduced
    }

    #[test]
    fn test_estimate_bedrooms_unit() {
        let mut record = mock_record();
        record.property_type = PropertyType::Unit;
        record.sale_price = Some(500_000);

        let enriched = estimate_bedrooms(record);

        assert_eq!(enriched.bedrooms, Some(2)); // $500k unit = 2br
    }

    #[test]
    fn test_calculate_yield() {
        let mut record = mock_record();
        record.sale_price = Some(800_000);
        record.weekly_rent = Some(600);

        let enriched = calculate_yield(record);

        // (600 * 52 / 800000) * 100 = 3.9%
        assert!(enriched.rental_yield.is_some());
        let yield_val = enriched.rental_yield.unwrap();
        assert!(yield_val > Decimal::from(3) && yield_val < Decimal::from(4));
    }

    #[test]
    fn test_calculate_yield_no_data() {
        let record = mock_record(); // No rent data
        let enriched = calculate_yield(record);

        assert!(enriched.rental_yield.is_none());
    }
}

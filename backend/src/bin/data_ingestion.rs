use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use csv::ReaderBuilder;
use realtor_api::calculate_rental_yield;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::collections::HashMap;
use std::io::Cursor;
use tracing::{error, info, warn};
use zip::ZipArchive;

// Configuration from environment
struct Config {
    database_url: String,
    nsw_sales_url: String,
    min_rental_yield: f32,
    max_properties: usize,
}

impl Config {
    fn from_env() -> Result<Self> {
        Ok(Config {
            database_url: std::env::var("DATABASE_URL")
                .context("DATABASE_URL must be set")?,
            nsw_sales_url: std::env::var("NSW_SALES_URL")
                .unwrap_or_else(|_| "https://nswpropertysalesdata.com/data/archive.zip".to_string()),
            min_rental_yield: std::env::var("MIN_RENTAL_YIELD")
                .unwrap_or_else(|_| "4.0".to_string())
                .parse()
                .context("MIN_RENTAL_YIELD must be a valid number")?,
            max_properties: std::env::var("MAX_PROPERTIES")
                .unwrap_or_else(|_| "100000".to_string())
                .parse()
                .context("MAX_PROPERTIES must be a valid number")?,
        })
    }
}

// NSW Property Sale record (raw CSV data)
#[derive(Debug, Clone)]
struct PropertySale {
    property_id: String,
    address: String,
    suburb: String,
    postcode: String,
    settlement_date: Option<NaiveDate>,
    purchase_price: Option<i32>,
}

// Enriched property with calculated yield
#[derive(Debug, Clone)]
struct EnrichedProperty {
    address: String,
    suburb: String,
    postcode: String,
    bedrooms: Option<i32>,
    price: i32,
    weekly_rent: i32,
    rental_yield: f32,
    sale_date: Option<NaiveDate>,
    quality_score: i16,
}

// Suburb statistics (aggregated)
#[derive(Debug)]
struct SuburbStatistics {
    suburb: String,
    postcode: String,
    bedrooms: i32,
    median_price: i32,
    median_weekly_rent: i32,
    median_rental_yield: f32,
    property_count: i32,
    min_yield: f32,
    max_yield: f32,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("🚀 Starting data ingestion worker...");

    // Load configuration
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;

    // Connect to database
    info!("📦 Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .context("Failed to connect to database")?;

    info!("✅ Database connected");

    // Log job start
    let job_id = log_job_start(&pool, "nsw_sales_ingestion").await?;

    // Run ingestion pipeline
    match run_nsw_ingestion(&pool, &config).await {
        Ok(stats) => {
            info!("✅ Ingestion completed successfully: {:?}", stats);
            log_job_complete(&pool, job_id, &stats).await?;
        }
        Err(e) => {
            error!("❌ Ingestion failed: {}", e);
            log_job_failed(&pool, job_id, &e.to_string()).await?;
            return Err(e);
        }
    }

    info!("🎉 Data ingestion worker finished");
    Ok(())
}

#[derive(Debug)]
struct IngestionStats {
    records_downloaded: i32,
    records_processed: i32,
    records_stored: i32,
    records_skipped: i32,
}

async fn run_nsw_ingestion(pool: &PgPool, config: &Config) -> Result<IngestionStats> {
    info!("📥 Step 1: Downloading NSW sales data...");
    let sales_data = download_and_parse_nsw_sales(&config.nsw_sales_url).await?;
    info!("✅ Downloaded {} property sales", sales_data.len());

    // For MVP, we'll use mock rental data
    // TODO: Implement actual NSW rental bond data fetcher
    info!("📥 Step 2: Loading rental bond data...");
    let rental_data = load_mock_rental_data();
    info!("✅ Loaded rental data for {} postcodes", rental_data.len());

    info!("🔄 Step 3: Enriching properties with rental yields...");
    let enriched = enrich_properties_with_yields(sales_data, rental_data, config)?;
    info!("✅ Enriched {} properties with yields >= {}%",
          enriched.len(), config.min_rental_yield);

    info!("📊 Step 4: Aggregating suburb statistics...");
    let suburb_stats = aggregate_by_suburb(&enriched);
    info!("✅ Calculated statistics for {} suburbs", suburb_stats.len());

    info!("💾 Step 5: Storing to database...");
    let stored_count = store_to_database(pool, &enriched, &suburb_stats, config).await?;
    info!("✅ Stored {} properties and {} suburb statistics",
          stored_count, suburb_stats.len());

    Ok(IngestionStats {
        records_downloaded: enriched.len() as i32,
        records_processed: enriched.len() as i32,
        records_stored: stored_count as i32,
        records_skipped: 0,
    })
}

async fn download_and_parse_nsw_sales(url: &str) -> Result<Vec<PropertySale>> {
    info!("Fetching ZIP file from: {}", url);

    // For MVP, return mock data since downloading 250MB will take time
    // TODO: Implement actual download and ZIP extraction
    warn!("Using mock data for MVP - implement actual download in production");

    Ok(generate_mock_nsw_sales())
}

fn generate_mock_nsw_sales() -> Vec<PropertySale> {
    // Generate mock NSW sales data for testing
    vec![
        PropertySale {
            property_id: "NSW001".to_string(),
            address: "10 George Street".to_string(),
            suburb: "Sydney".to_string(),
            postcode: "2000".to_string(),
            settlement_date: Some(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()),
            purchase_price: Some(850000),
        },
        PropertySale {
            property_id: "NSW002".to_string(),
            address: "25 Oxford Street".to_string(),
            suburb: "Darlinghurst".to_string(),
            postcode: "2010".to_string(),
            settlement_date: Some(NaiveDate::from_ymd_opt(2024, 7, 20).unwrap()),
            purchase_price: Some(720000),
        },
        PropertySale {
            property_id: "NSW003".to_string(),
            address: "50 Bondi Road".to_string(),
            suburb: "Bondi".to_string(),
            postcode: "2026".to_string(),
            settlement_date: Some(NaiveDate::from_ymd_opt(2024, 8, 10).unwrap()),
            purchase_price: Some(1200000),
        },
    ]
}

fn load_mock_rental_data() -> HashMap<(String, i32), i32> {
    // Mock rental bond data: (postcode, bedrooms) -> weekly_rent
    let mut rental_data = HashMap::new();

    // Sydney CBD
    rental_data.insert(("2000".to_string(), 1), 600);
    rental_data.insert(("2000".to_string(), 2), 850);
    rental_data.insert(("2000".to_string(), 3), 1100);

    // Darlinghurst
    rental_data.insert(("2010".to_string(), 1), 500);
    rental_data.insert(("2010".to_string(), 2), 700);
    rental_data.insert(("2010".to_string(), 3), 900);

    // Bondi
    rental_data.insert(("2026".to_string(), 1), 550);
    rental_data.insert(("2026".to_string(), 2), 800);
    rental_data.insert(("2026".to_string(), 3), 1050);

    rental_data
}

fn enrich_properties_with_yields(
    sales: Vec<PropertySale>,
    rentals: HashMap<(String, i32), i32>,
    config: &Config,
) -> Result<Vec<EnrichedProperty>> {
    let mut enriched = Vec::new();

    for sale in sales {
        let price = match sale.purchase_price {
            Some(p) if p > 0 => p,
            _ => continue, // Skip properties without valid price
        };

        // Try different bedroom counts to find rental data
        for bedrooms in [1, 2, 3] {
            if let Some(&weekly_rent) = rentals.get(&(sale.postcode.clone(), bedrooms)) {
                if let Some(yield_value) = calculate_rental_yield(price, weekly_rent) {
                    if yield_value >= config.min_rental_yield {
                        enriched.push(EnrichedProperty {
                            address: sale.address.clone(),
                            suburb: sale.suburb.clone(),
                            postcode: sale.postcode.clone(),
                            bedrooms: Some(bedrooms),
                            price,
                            weekly_rent,
                            rental_yield: yield_value,
                            sale_date: sale.settlement_date,
                            quality_score: 7, // Medium quality for postcode-matched data
                        });
                        break; // Found a match, move to next property
                    }
                }
            }
        }
    }

    // Sort by yield descending and take top N
    enriched.sort_by(|a, b| b.rental_yield.partial_cmp(&a.rental_yield).unwrap());
    enriched.truncate(config.max_properties);

    Ok(enriched)
}

fn aggregate_by_suburb(properties: &[EnrichedProperty]) -> Vec<SuburbStatistics> {
    let mut groups: HashMap<(String, String, i32), Vec<&EnrichedProperty>> = HashMap::new();

    // Group by suburb, postcode, bedrooms
    for prop in properties {
        if let Some(bedrooms) = prop.bedrooms {
            let key = (prop.suburb.clone(), prop.postcode.clone(), bedrooms);
            groups.entry(key).or_insert_with(Vec::new).push(prop);
        }
    }

    // Calculate statistics for each group
    let mut stats = Vec::new();
    for ((suburb, postcode, bedrooms), props) in groups {
        if props.is_empty() {
            continue;
        }

        let mut yields: Vec<f32> = props.iter().map(|p| p.rental_yield).collect();
        yields.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut prices: Vec<i32> = props.iter().map(|p| p.price).collect();
        prices.sort();

        let mut rents: Vec<i32> = props.iter().map(|p| p.weekly_rent).collect();
        rents.sort();

        let median_idx = props.len() / 2;

        stats.push(SuburbStatistics {
            suburb: suburb.clone(),
            postcode: postcode.clone(),
            bedrooms,
            median_price: prices[median_idx],
            median_weekly_rent: rents[median_idx],
            median_rental_yield: yields[median_idx],
            property_count: props.len() as i32,
            min_yield: yields[0],
            max_yield: yields[yields.len() - 1],
        });
    }

    stats
}

async fn store_to_database(
    pool: &PgPool,
    properties: &[EnrichedProperty],
    suburbs: &[SuburbStatistics],
    _config: &Config,
) -> Result<usize> {
    let mut stored_count = 0;

    // Store properties
    for prop in properties {
        let result = sqlx::query!(
            r#"
            INSERT INTO properties (
                address, suburb, postcode, state, bedrooms, price, weekly_rent,
                rental_yield, sale_date, data_source, quality_score
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (address, suburb, state, postcode)
            DO UPDATE SET
                price = EXCLUDED.price,
                weekly_rent = EXCLUDED.weekly_rent,
                rental_yield = EXCLUDED.rental_yield,
                sale_date = EXCLUDED.sale_date,
                last_updated = NOW()
            "#,
            prop.address,
            prop.suburb,
            prop.postcode,
            "NSW" as &str, // state_enum
            prop.bedrooms,
            prop.price,
            prop.weekly_rent,
            rust_decimal::Decimal::from_f32_retain(prop.rental_yield),
            prop.sale_date,
            "nsw_sales",
            prop.quality_score,
        )
        .execute(pool)
        .await;

        match result {
            Ok(_) => stored_count += 1,
            Err(e) => warn!("Failed to store property {}: {}", prop.address, e),
        }
    }

    // Store suburb statistics
    for stat in suburbs {
        sqlx::query!(
            r#"
            INSERT INTO suburb_statistics (
                suburb, postcode, state, bedrooms,
                median_price, median_weekly_rent, median_rental_yield,
                property_count, min_yield, max_yield,
                calculated_date, data_source
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (suburb, postcode, state, bedrooms, calculated_date)
            DO UPDATE SET
                median_price = EXCLUDED.median_price,
                median_weekly_rent = EXCLUDED.median_weekly_rent,
                median_rental_yield = EXCLUDED.median_rental_yield,
                property_count = EXCLUDED.property_count,
                last_updated = NOW()
            "#,
            stat.suburb,
            stat.postcode,
            "NSW" as &str,
            stat.bedrooms,
            stat.median_price,
            stat.median_weekly_rent,
            rust_decimal::Decimal::from_f32_retain(stat.median_rental_yield),
            stat.property_count,
            rust_decimal::Decimal::from_f32_retain(stat.min_yield),
            rust_decimal::Decimal::from_f32_retain(stat.max_yield),
            Utc::now().date_naive(),
            "nsw_sales",
        )
        .execute(pool)
        .await
        .context("Failed to store suburb statistics")?;
    }

    Ok(stored_count)
}

async fn log_job_start(pool: &PgPool, job_name: &str) -> Result<i32> {
    let record = sqlx::query!(
        r#"
        INSERT INTO ingestion_logs (job_name, state, status, started_at)
        VALUES ($1, $2, $3, NOW())
        RETURNING id
        "#,
        job_name,
        "NSW" as &str,
        "started",
    )
    .fetch_one(pool)
    .await
    .context("Failed to log job start")?;

    Ok(record.id)
}

async fn log_job_complete(pool: &PgPool, job_id: i32, stats: &IngestionStats) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE ingestion_logs
        SET status = $1,
            completed_at = NOW(),
            duration_seconds = EXTRACT(EPOCH FROM (NOW() - started_at))::INTEGER,
            records_downloaded = $2,
            records_processed = $3,
            records_stored = $4,
            records_skipped = $5
        WHERE id = $6
        "#,
        "completed",
        stats.records_downloaded,
        stats.records_processed,
        stats.records_stored,
        stats.records_skipped,
        job_id,
    )
    .execute(pool)
    .await
    .context("Failed to log job completion")?;

    Ok(())
}

async fn log_job_failed(pool: &PgPool, job_id: i32, error: &str) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE ingestion_logs
        SET status = $1,
            completed_at = NOW(),
            duration_seconds = EXTRACT(EPOCH FROM (NOW() - started_at))::INTEGER,
            error_message = $2
        WHERE id = $3
        "#,
        "failed",
        error,
        job_id,
    )
    .execute(pool)
    .await
    .context("Failed to log job failure")?;

    Ok(())
}
//! Data ingestion orchestrator - runs fetch, parse, enrich, write pipelines

use anyhow::Result;
use chrono::{NaiveDate, Utc};
use real_estate_backend::ingestion::{
    enrich, fetch, parse, write, PropertyRecord, RentalMedian, WriteStats,
};
use sqlx::PgPool;
use std::env;
use std::path::PathBuf;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();

    info!("Starting data ingestion pipeline");

    // Load configuration from environment
    let config = Config::from_env()?;
    info!("Configuration loaded");

    // Connect to database
    let db = PgPool::connect(&config.database_url).await?;
    info!("Database connected");

    // Determine which sources to run (from command line args or run all)
    let args: Vec<String> = env::args().collect();
    let sources = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        vec!["nsw_sales".to_string(), "nsw_rentals".to_string()]
    };

    // Run each source
    for source_id in sources {
        info!("Running ingestion for: {}", source_id);

        let result = match source_id.as_str() {
            "nsw_sales" => run_nsw_sales(&config, &db).await,
            "nsw_rentals" => run_nsw_rentals(&config, &db).await,
            _ => {
                warn!("Unknown source: {}", source_id);
                continue;
            }
        };

        match result {
            Ok(stats) => {
                info!("✓ {} completed: {}", source_id, stats);
            }
            Err(e) => {
                error!("✗ {} failed: {}", source_id, e);
            }
        }
    }

    info!("Data ingestion pipeline complete");

    Ok(())
}

/// Run NSW sales data ingestion
async fn run_nsw_sales(config: &Config, db: &PgPool) -> Result<WriteStats> {
    info!("=== NSW Sales Pipeline ===");

    // Step 1: Fetch raw data
    info!("Step 1/4: Fetching data...");
    let raw_data = fetch::fetch_nsw_sales(&config.nsw_sales_url, &config.temp_dir).await?;
    info!("✓ Fetch complete");

    // Step 2: Parse into PropertyRecord structs
    info!("Step 2/4: Parsing data...");
    let records = parse::parse_nsw_sales(raw_data, "nsw_sales".to_string()).await?;
    info!("✓ Parsed {} records", records.len());

    // Limit to first N records for testing (optional)
    let records = if config.limit_records > 0 {
        let limit = config.limit_records.min(records.len());
        warn!("Limiting to first {} records (testing mode)", limit);
        records.into_iter().take(limit).collect()
    } else {
        records
    };

    // Step 3: Enrich (estimate bedrooms, match rentals, calculate yields)
    info!("Step 3/4: Enriching data...");
    let enriched = enrich::enrich_all(records, db).await?;
    info!("✓ Enriched {} records", enriched.len());

    // Step 4: Write to database
    info!("Step 4/4: Writing to database...");
    let stats = write::write_properties(db, enriched).await?;
    info!("✓ Write complete");

    Ok(stats)
}

/// Run NSW rental bond data ingestion
async fn run_nsw_rentals(config: &Config, db: &PgPool) -> Result<WriteStats> {
    info!("=== NSW Rentals Pipeline ===");

    // Step 1: Fetch raw data
    info!("Step 1/3: Fetching data...");
    let raw_data = fetch::fetch_nsw_rentals(&config.nsw_rentals_url).await?;
    info!("✓ Fetch complete");

    // Step 2: Parse into RentalMedian structs
    info!("Step 2/3: Parsing data...");
    let period = Utc::now().naive_utc().date();
    let rentals = parse::parse_nsw_rentals(raw_data, period).await?;
    info!("✓ Parsed {} rental medians", rentals.len());

    // Step 3: Write to database
    info!("Step 3/3: Writing to database...");
    let stats = write::write_rental_medians(db, rentals).await?;
    info!("✓ Write complete");

    Ok(stats)
}

/// Configuration loaded from environment variables
#[derive(Debug, Clone)]
struct Config {
    database_url: String,
    temp_dir: PathBuf,
    nsw_sales_url: String,
    nsw_rentals_url: String,
    limit_records: usize, // 0 = no limit
}

impl Config {
    fn from_env() -> Result<Self> {
        Ok(Config {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://realtor_user:realtor_pass@localhost:5432/realtor_db".to_string()),

            temp_dir: env::var("TEMP_DIR")
                .unwrap_or_else(|_| "/tmp/real_estate_ingestion".to_string())
                .into(),

            nsw_sales_url: env::var("NSW_SALES_URL")
                .unwrap_or_else(|_| "https://nswpropertysalesdata.com/data/archive.zip".to_string()),

            nsw_rentals_url: env::var("NSW_RENTALS_URL")
                .unwrap_or_else(|_| {
                    // Default to a recent monthly file - user should update this
                    "https://www.nsw.gov.au/sites/default/files/2024-12/rental-bond-data-december-2024.xlsx".to_string()
                }),

            limit_records: env::var("LIMIT_RECORDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
        })
    }
}

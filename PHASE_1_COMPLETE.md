# Phase 1: NSW Data Ingestion Pipeline - Complete ✅

## Summary

Phase 1 of the data ingestion pipeline is now implemented. The codebase uses a **functional, modular architecture** that makes it easy to add new data sources without coupling.

---

## What Was Built

### 1. Database Schema (`database/init/03_ingestion_schema.sql`)
- Extended properties table with multi-source tracking
- Added `rental_medians` table for postcode + bedroom matching
- Added `sales_history` table for tracking price changes
- Added `ingestion_runs` table for monitoring pipeline runs
- Created indexes for fast lookups

### 2. Core Data Types (`backend/src/ingestion/types.rs`)
- `RawData` - Tagged union for different data formats (File, Bytes, Json, CSV)
- `PropertyRecord` - Universal property data model
- `RentalMedian` - Rental bond data structure
- `SourceMetadata` - Data provenance and quality tracking
- `WriteStats` - Pipeline execution statistics

### 3. Utility Functions (`backend/src/ingestion/utils.rs`)
- `http_get()` - Download files with timeout
- `extract_csv_from_zip()` - Extract CSV from ZIP archives
- `parse_nsw_property_type()` - Parse NSW property type strings
- `format_nsw_address()` - Format addresses from components

### 4. Fetch Functions (`backend/src/ingestion/fetch.rs`)
- `fetch_nsw_sales()` - Download and extract NSW sales ZIP
- `fetch_nsw_rentals()` - Download NSW rental bond XLSX

### 5. Parse Functions (`backend/src/ingestion/parse.rs`)
- `parse_nsw_sales()` - Parse NSW sales CSV to PropertyRecord
- `parse_nsw_rentals()` - Parse NSW rental XLSX to RentalMedian

### 6. Enrichment Functions (`backend/src/ingestion/enrich.rs`)
- `estimate_bedrooms()` - Estimate bedrooms from price and property type
- `match_rental()` - Match properties to rental data by postcode + bedrooms
- `calculate_yield()` - Calculate rental yield percentage
- `enrich_all()` - Convenience function to run all enrichers

### 7. Write Functions (`backend/src/ingestion/write.rs`)
- `write_properties()` - Write properties with conflict resolution
- `write_rental_medians()` - Write rental bond data
- Smart conflict resolution based on data quality scores
- Automatic sales history tracking

### 8. Orchestrator Binary (`backend/src/bin/data_ingestion/main.rs`)
- Runs NSW sales and rentals pipelines
- Environment-based configuration
- Structured logging
- Command-line source selection

---

## Architecture Highlights

### Functional Design ✅
- **Pure functions** instead of trait-based inheritance
- **No coupling** between data sources
- **Easy to test** - functions are pure and composable
- **Clear data flow**: Fetch → Parse → Enrich → Write

### Modular Structure ✅
- Each state/source has its own `fetch_*()` and `parse_*()` functions
- Shared utilities extracted automatically when duplication appears
- Adding a new source = writing 2 functions + registering in orchestrator
- Remove a source = delete its functions (no side effects)

### Example: Adding a New Source
```rust
// 1. Write fetch function
async fn fetch_vic_medians(url: &str) -> Result<RawData> { ... }

// 2. Write parse function
async fn parse_vic_medians(raw: RawData) -> Result<Vec<PropertyRecord>> { ... }

// 3. Add to orchestrator
"vic_medians" => run_vic_medians(&config, &db).await,
```

No inheritance, no traits, just functions!

---

## File Structure

```
backend/src/
├── lib.rs                        # Exports ingestion module
├── main.rs                       # API server binary
├── bin/
│   └── data_ingestion/
│       └── main.rs              # Data ingestion orchestrator
│
└── ingestion/                   # Ingestion library
    ├── mod.rs                   # Module exports
    ├── types.rs                 # Pure data structures
    ├── utils.rs                 # Shared utilities
    ├── fetch.rs                 # Fetch functions (nsw_sales, nsw_rentals)
    ├── parse.rs                 # Parse functions (nsw_sales, nsw_rentals)
    ├── enrich.rs                # Enrichment pipeline
    └── write.rs                 # Database writers with conflict resolution
```

---

## How to Run

### 1. Apply Database Schema
```bash
docker-compose up -d
docker exec -i real_estate-postgres-1 psql -U realtor_user -d realtor_db < database/init/03_ingestion_schema.sql
```

### 2. Build the Ingestion Binary
```bash
cd backend
cargo build --bin data-ingestion
```

### 3. Set Environment Variables (Optional)
```bash
export DATABASE_URL="postgresql://realtor_user:realtor_pass@localhost:5432/realtor_db"
export NSW_SALES_URL="https://nswpropertysalesdata.com/data/archive.zip"
export NSW_RENTALS_URL="https://www.nsw.gov.au/sites/default/files/2024-12/rental-bond-data-december-2024.xlsx"
export TEMP_DIR="/tmp/real_estate_ingestion"
export LIMIT_RECORDS=1000  # Optional: limit records for testing
```

### 4. Run the Pipeline
```bash
# Run both NSW sources
cargo run --bin data-ingestion

# Run specific source
cargo run --bin data-ingestion nsw_sales
cargo run --bin data-ingestion nsw_rentals
```

---

## Data Flow Example

### NSW Sales Pipeline

```
1. FETCH
   ↓ fetch_nsw_sales()
   Downloads: https://nswpropertysalesdata.com/data/archive.zip
   Extracts: archive.csv
   Returns: RawData::File(path)

2. PARSE
   ↓ parse_nsw_sales()
   Reads CSV rows
   Transforms to PropertyRecord:
   - address: "10 Smith Street"
   - suburb: "Sydney"
   - state: NSW
   - postcode: "2000"
   - sale_price: 750000
   - bedrooms: None  ← Will estimate
   - weekly_rent: None  ← Will match

3. ENRICH
   ↓ estimate_bedrooms()
   Estimates bedrooms: 3 (based on $750k house)

   ↓ match_rental()
   Queries rental_medians table
   Finds: postcode=2000, bedrooms=3 → $600/week

   ↓ calculate_yield()
   Calculates: (600 * 52 / 750000) * 100 = 4.16%

4. WRITE
   ↓ write_properties()
   - Checks if property exists
   - Compares data quality scores
   - Inserts or updates property
   - Tracks sale in sales_history
```

---

## Data Quality & Conflict Resolution

Properties are ranked by quality score:
- **Individual** (NSW sales): 100 points × confidence
- **Listing** (Domain API): 90 points × confidence
- **Aggregated** (VIC medians): 50 points × confidence
- **Estimated**: 25 points × confidence

When a property exists, new data only replaces old data if quality score is >10% better.

Example:
- Existing: Estimated data (25 × 0.5 = 12.5)
- New: Individual data (100 × 0.8 = 80)
- 80 > 12.5 × 1.1 → **Replace** ✅

---

## Next Steps (Future Phases)

### Phase 2: WA Integration
- `fetch_wa_sales()` - WA sales evidence data
- `parse_wa_sales()` - Parse WA CSV format
- Individual property records back to 1988

### Phase 3: Domain API
- `fetch_domain_api()` - OAuth2 + API calls
- `parse_domain_json()` - Parse JSON responses
- Rental data for all states
- Current market listings

### Phase 4: VIC/QLD/SA
- `fetch_vic_medians()` - Quarterly XLSX
- `parse_vic_medians()` - Suburb-level aggregates
- Generate "estimated" properties from medians

---

## Testing

### Unit Tests Included
- `utils.rs` - Property type parsing, address formatting
- `enrich.rs` - Bedroom estimation, yield calculation
- `write.rs` - Conflict resolution logic

### Run Tests
```bash
cd backend
cargo test ingestion
```

---

## Configuration

All sources are configured via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | localhost:5432 | PostgreSQL connection string |
| `TEMP_DIR` | /tmp/real_estate_ingestion | Temporary file storage |
| `NSW_SALES_URL` | nswpropertysalesdata.com | NSW sales data ZIP |
| `NSW_RENTALS_URL` | nsw.gov.au | NSW rental bond XLSX |
| `LIMIT_RECORDS` | 0 (no limit) | Limit records for testing |

---

## Success Metrics

✅ Modular functional architecture
✅ Zero coupling between data sources
✅ Easy to add/remove sources
✅ Conflict resolution based on data quality
✅ Full data provenance tracking
✅ Compiles successfully
✅ Ready for Phase 2 (WA) and Phase 3 (Domain API)

---

## Notes

- The ingestion binary is separate from the API server
- Each source runs independently
- Failed sources don't block others
- All data sources are tracked in `ingestion_runs` table
- Properties maintain full history in `sales_history` table

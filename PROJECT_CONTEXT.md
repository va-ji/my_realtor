# Australian Real Estate Analysis Tool - Project Context

## Project Overview
A real estate analysis tool for Australian property investors focusing on rentvesting strategies and rental yield calculations.

## Current Status (as of 2025-10-02)

### ✅ Completed Components

#### 1. Backend (Rust + Axum)
- **Location**: `/backend/src/`
- **API Server Binary**: `src/main.rs` - Running on `http://localhost:3001`
- **Data Ingestion Binary**: `src/bin/data_ingestion/main.rs` - Standalone ingestion worker
- **Endpoints**:
  - `/api/health` - Health check
  - `/api/properties` - Returns all properties with calculated rental yields
- **Database**: Connected to PostgreSQL
- **Tests**: 6 unit tests for rental yield calculations (all passing)
- **Key Function**: `calculate_rental_yield(price, weekly_rent)` in `src/lib.rs`
- **Formula**: `(weekly_rent × 52 / price) × 100`

#### 2. Database (PostgreSQL + PostGIS)
- **Container**: `real_estate-postgres-1` (Docker)
- **Connection**: `postgresql://realtor_user:realtor_pass@localhost:5432/realtor_db`
- **Schema**:
  - `properties` table: Extended with multi-source tracking fields (postcode, property_type, data_source, data_quality, confidence_score, external_id, etc.)
  - `rental_medians` table: Postcode + bedroom rental data for matching
  - `sales_history` table: Tracks all sales events per property
  - `price_history` table: Legacy price tracking
  - `ingestion_runs` table: Monitors pipeline execution
  - State enum: NSW, VIC, QLD, WA, SA, TAS, ACT, NT
  - Property type enum: house, unit, townhouse, vacant_land, commercial, other
  - Data quality enum: individual, aggregated, estimated, listing
- **Init Scripts**:
  - `/database/init/01_simple_init.sql` - Basic schema
  - `/database/init/02_seed_data.sql` - Sample data
  - `/database/init/03_ingestion_schema.sql` - **NEW** Multi-source ingestion schema

#### 3. Frontend (SvelteKit + TypeScript)
- **Location**: `/frontend/src/`
- **Dev Server**: Running on `http://localhost:5173`
- **Features**:
  - Property listing with card-based layout
  - State filter dropdown (ALL, VIC, NSW, QLD, etc.)
  - Displays: address, suburb, bedrooms, price, weekly rent, rental yield
  - Australian currency formatting
  - Responsive grid with hover effects
- **API Integration**: Fetches from backend via `/src/lib/api.ts`
- **Types**: Defined in `/src/lib/types.ts`
- **Tests**: 9 unit tests for formatting functions (all passing)

#### 4. Docker Configuration
- **File**: `/docker-compose.yml`
- **Services**: PostgreSQL (postgis/postgis:15-3.3)
- **Volumes**:
  - `./database/data` - PostgreSQL data
  - `./database/init` - Init scripts
- **Network**: `realtor_network`

#### 5. ✅ **NEW: Data Ingestion Pipeline (Phase 1 - NSW Complete)**
- **Architecture**: Functional, modular design (no trait inheritance, pure functions)
- **Location**: `/backend/src/ingestion/`
- **Binary**: `cargo run --bin data-ingestion`
- **Status**: ✅ **COMPILES SUCCESSFULLY**

##### Components Built:
1. **Core Types** (`types.rs`): RawData, PropertyRecord, RentalMedian, SourceMetadata, WriteStats
2. **Utils** (`utils.rs`): http_get, extract_csv_from_zip, parse_nsw_property_type
3. **Fetch** (`fetch.rs`): fetch_nsw_sales, fetch_nsw_rentals
4. **Parse** (`parse.rs`): parse_nsw_sales, parse_nsw_rentals
5. **Enrich** (`enrich.rs`): estimate_bedrooms, match_rental, calculate_yield, enrich_all
6. **Write** (`write.rs`): write_properties, write_rental_medians (with conflict resolution)
7. **Orchestrator** (`bin/data_ingestion/main.rs`): Runs fetch → parse → enrich → write pipelines

##### Data Flow:
```
NSW Sales: Fetch ZIP → Extract CSV → Parse 1.8M properties → Estimate bedrooms → Match rentals → Calculate yields → Write to DB
NSW Rentals: Fetch XLSX → Parse postcode+bedroom medians → Write to rental_medians table
```

##### Key Features:
- **Functional Design**: Pure functions, no coupling between sources
- **Smart Conflict Resolution**: Data quality scoring (Individual > Listing > Aggregated > Estimated)
- **Full Provenance**: Tracks source, quality, confidence score for every field
- **Incremental Updates**: Only replaces data if new data is significantly better (>10% quality score)
- **Sales History**: Tracks all price changes over time
- **Extensible**: Add new source = write 2 functions (fetch + parse)

##### Configuration (Environment Variables):
```bash
DATABASE_URL=postgresql://realtor_user:realtor_pass@localhost:5432/realtor_db
TEMP_DIR=/tmp/real_estate_ingestion
NSW_SALES_URL=https://nswpropertysalesdata.com/data/archive.zip
NSW_RENTALS_URL=https://www.nsw.gov.au/sites/default/files/2024-12/rental-bond-data-december-2024.xlsx
LIMIT_RECORDS=0  # 0 = no limit, >0 = limit for testing
```

---

## 🚧 **NEXT STEPS: Docker + Cron Deployment**

### **Immediate Tasks (Next Session)**

#### 1. **Apply Database Schema to Running Database**
```bash
docker exec -i real_estate-postgres-1 psql -U realtor_user -d realtor_db < database/init/03_ingestion_schema.sql
```

#### 2. **Create Dockerfile for Data Ingestion Worker**
**File**: `backend/Dockerfile.ingestion`
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --bin data-ingestion

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/data-ingestion /usr/local/bin/
CMD ["data-ingestion"]
```

#### 3. **Add Data Ingestion Service to docker-compose.yml**
```yaml
services:
  postgres:
    # ... existing config ...

  ingestion-worker:
    build:
      context: ./backend
      dockerfile: Dockerfile.ingestion
    container_name: real_estate-ingestion
    depends_on:
      - postgres
    environment:
      DATABASE_URL: postgresql://realtor_user:realtor_pass@postgres:5432/realtor_db
      NSW_SALES_URL: https://nswpropertysalesdata.com/data/archive.zip
      NSW_RENTALS_URL: https://www.nsw.gov.au/sites/default/files/2024-12/rental-bond-data-december-2024.xlsx
      TEMP_DIR: /tmp/ingestion
      RUST_LOG: info
    volumes:
      - ./ingestion_logs:/var/log/ingestion
    networks:
      - realtor_network
    # For now, run manually or on cron outside container
    # Later: Add supercronic for scheduled runs
```

#### 4. **Add Cron Scheduling (Two Options)**

**Option A: Supercronic (Inside Container)**
```dockerfile
# Add to Dockerfile.ingestion
RUN apt-get install -y curl && \
    curl -fsSLO https://github.com/aptible/supercronic/releases/download/v0.2.29/supercronic-linux-amd64 && \
    chmod +x supercronic-linux-amd64 && \
    mv supercronic-linux-amd64 /usr/local/bin/supercronic

COPY crontab /etc/crontab
CMD ["supercronic", "/etc/crontab"]
```

**File**: `backend/crontab`
```cron
# Run NSW sales daily at 6am (after NSW updates at 5am)
0 6 * * * /usr/local/bin/data-ingestion nsw_sales >> /var/log/ingestion/nsw_sales.log 2>&1

# Run NSW rentals monthly on 5th (after monthly data published)
0 7 5 * * /usr/local/bin/data-ingestion nsw_rentals >> /var/log/ingestion/nsw_rentals.log 2>&1
```

**Option B: Server-Level Cron (Outside Container)**
```bash
# On your server, add to crontab:
0 6 * * * docker exec real_estate-ingestion data-ingestion nsw_sales >> /var/log/real_estate/nsw_sales.log 2>&1
0 7 5 * * docker exec real_estate-ingestion data-ingestion nsw_rentals >> /var/log/real_estate/nsw_rentals.log 2>&1
```

#### 5. **Test End-to-End Stack**
```bash
# 1. Apply schema
docker exec -i real_estate-postgres-1 psql -U realtor_user -d realtor_db < database/init/03_ingestion_schema.sql

# 2. Build and start ingestion worker
docker-compose build ingestion-worker
docker-compose up -d ingestion-worker

# 3. Run manual ingestion test (limit to 100 records)
docker exec -e LIMIT_RECORDS=100 real_estate-ingestion data-ingestion nsw_sales

# 4. Check database for new properties
docker exec -it real_estate-postgres-1 psql -U realtor_user -d realtor_db -c "SELECT COUNT(*) FROM properties WHERE data_source = 'nsw_sales';"

# 5. Check API returns new data
curl http://localhost:3001/api/properties | jq '.[0]'

# 6. Check frontend displays new data
# Visit http://localhost:5173
```

#### 6. **Add Monitoring & Logging**
- Check `ingestion_runs` table for pipeline status
- Add Slack/email alerts for failures
- Add Grafana dashboard for ingestion metrics

#### 7. **Create Deployment Scripts**
**File**: `scripts/deploy.sh`
```bash
#!/bin/bash
set -e

echo "Deploying Real Estate Backend..."

# 1. Pull latest code
git pull origin main

# 2. Apply database migrations
docker exec -i real_estate-postgres-1 psql -U realtor_user -d realtor_db < database/init/03_ingestion_schema.sql

# 3. Rebuild and restart services
docker-compose build
docker-compose up -d

# 4. Run health checks
sleep 5
curl http://localhost:3001/api/health

echo "✅ Deployment complete!"
```

---

## Multi-State Data Strategy (Documented)

See `MULTI_STATE_DATA_STRATEGY.md` for full details.

### Phase 1: ✅ NSW (Complete)
- NSW Sales (1.8M properties, daily)
- NSW Rentals (postcode + bedroom medians, monthly)

### Phase 2: 🚧 WA (Next)
- WA Sales Evidence (individual properties since 1988)
- Add `fetch_wa_sales()` and `parse_wa_sales()`

### Phase 3: 🚧 Domain API
- Register for free tier
- Test rate limits
- Add `fetch_domain_api()` and `parse_domain_json()`
- Use for rentals + smaller states

### Phase 4: 🚧 VIC/QLD/SA
- Aggregated suburb-level data
- Generate "estimated" properties from medians

---

## Tech Stack
- **Backend**: Rust (Axum, SQLx, tokio, reqwest, csv, calamine, zip, tracing)
- **Frontend**: SvelteKit, TypeScript, Vitest
- **Database**: PostgreSQL 15 + PostGIS
- **Infrastructure**: Docker, docker-compose, supercronic (cron for containers)

---

## File Structure
```
real_estate/
├── backend/
│   ├── src/
│   │   ├── main.rs                      # API server binary
│   │   ├── lib.rs                       # Exports ingestion module
│   │   ├── bin/
│   │   │   └── data_ingestion/
│   │   │       └── main.rs              # Data ingestion orchestrator
│   │   └── ingestion/                   # **NEW** Ingestion pipeline
│   │       ├── mod.rs                   # Module exports
│   │       ├── types.rs                 # Pure data structures
│   │       ├── utils.rs                 # Shared utilities
│   │       ├── fetch.rs                 # Fetch functions (NSW)
│   │       ├── parse.rs                 # Parse functions (NSW)
│   │       ├── enrich.rs                # Enrichment pipeline
│   │       └── write.rs                 # Database writers
│   ├── Cargo.toml
│   ├── Dockerfile                       # API server
│   ├── Dockerfile.ingestion             # **TODO** Ingestion worker
│   └── crontab                          # **TODO** Cron schedule
├── frontend/
│   ├── src/
│   │   ├── routes/
│   │   │   └── +page.svelte            # Main property listing page
│   │   └── lib/
│   │       ├── api.ts                  # API client
│   │       ├── types.ts                # TypeScript interfaces
│   │       └── utils.test.ts           # Unit tests
│   ├── package.json
│   └── vitest.config.ts
├── database/
│   ├── init/
│   │   ├── 01_simple_init.sql          # Basic schema
│   │   ├── 02_seed_data.sql            # Sample data
│   │   └── 03_ingestion_schema.sql     # **NEW** Multi-source schema
│   └── data/                           # PostgreSQL data volume
├── scripts/
│   └── deploy.sh                       # **TODO** Deployment script
├── docker-compose.yml                  # **TODO** Add ingestion-worker
├── .env.example
├── PROJECT_CONTEXT.md                  # This file
├── MULTI_STATE_DATA_STRATEGY.md        # Data source strategy
├── PHASE_1_COMPLETE.md                 # Phase 1 implementation details
└── IMPLEMENTATION_STATUS.md            # Legacy status

```

---

## Running the Project

### Start Database
```bash
docker-compose up -d
```

### Apply Ingestion Schema (REQUIRED BEFORE FIRST RUN)
```bash
docker exec -i real_estate-postgres-1 psql -U realtor_user -d realtor_db < database/init/03_ingestion_schema.sql
```

### Start Backend API
```bash
cd backend
cargo run --release --bin api-server  # Runs on :3001
```

### Run Data Ingestion (Manual)
```bash
cd backend

# Run both NSW sources
cargo run --bin data-ingestion

# Run specific source
cargo run --bin data-ingestion nsw_sales
cargo run --bin data-ingestion nsw_rentals

# Test with limited records
LIMIT_RECORDS=100 cargo run --bin data-ingestion nsw_sales
```

### Start Frontend
```bash
cd frontend
npm run dev  # Runs on :5173
```

### Run Tests
```bash
# Backend
cd backend
cargo test

# Frontend
cd frontend
npm test
```

---

## Key Environment Variables
```bash
# Database
DATABASE_URL=postgresql://realtor_user:realtor_pass@localhost:5432/realtor_db

# API Server
API_PORT=3001

# Data Ingestion
TEMP_DIR=/tmp/real_estate_ingestion
NSW_SALES_URL=https://nswpropertysalesdata.com/data/archive.zip
NSW_RENTALS_URL=https://www.nsw.gov.au/sites/default/files/2024-12/rental-bond-data-december-2024.xlsx
LIMIT_RECORDS=0  # 0 = no limit

# Logging
RUST_LOG=info
```

---

## Data URLs for Ingestion
- **NSW Sales**: `https://nswpropertysalesdata.com/data/archive.zip` (daily, 5am)
- **NSW Rentals**: `https://www.nsw.gov.au/housing-and-construction/rental-forms-surveys-and-data/rental-bond-data` (monthly)
- **VIC Sales**: `https://www.land.vic.gov.au/__data/assets/excel_doc/0029/709751/Houses-by-suburb-2013-2023.xlsx` (quarterly)
- **WA Sales**: `https://catalogue.data.wa.gov.au/dataset/sales-evidence-data`

---

## Architecture: Functional, Modular Design

### Why Functional Over Trait-Based?
✅ **No coupling** - Each source is independent
✅ **Explicit behavior** - No hidden trait magic
✅ **Easy to extend** - Add source = write 2 functions
✅ **Easy to test** - Pure functions, no mocking
✅ **Clear data flow** - Fetch → Parse → Enrich → Write

### Adding a New Source
```rust
// 1. Write fetch function
async fn fetch_vic_medians(url: &str) -> Result<RawData> { ... }

// 2. Write parse function
async fn parse_vic_medians(raw: RawData) -> Result<Vec<PropertyRecord>> { ... }

// 3. Add to orchestrator
"vic_medians" => run_vic_medians(&config, &db).await,
```

No traits, no inheritance, just functions!

---

## Data Quality & Conflict Resolution

Properties are ranked by quality score:
- **Individual** (NSW/WA sales): 100 × confidence
- **Listing** (Domain API): 90 × confidence
- **Aggregated** (VIC/SA medians): 50 × confidence
- **Estimated**: 25 × confidence

New data replaces old data only if quality score is >10% better.

---

## Questions Resolved
1. ✅ Postcode + bedroom matching: YES (acceptable for MVP)
2. ✅ Architecture: Functional, modular design (no traits)
3. ✅ Start with NSW only: YES (Phase 1 complete)
4. ✅ Missing bedrooms: Estimate based on price + property type
5. 🚧 Docker deployment: **NEXT SESSION**

---

## TODO Next Session

### Priority 1: Docker + Server Deployment
- [ ] Create `backend/Dockerfile.ingestion`
- [ ] Add `ingestion-worker` service to `docker-compose.yml`
- [ ] Apply database schema: `03_ingestion_schema.sql`
- [ ] Test full stack: DB → Ingestion → API → Frontend
- [ ] Add cron scheduling (supercronic or server-level)
- [ ] Create deployment scripts (`scripts/deploy.sh`)

### Priority 2: Testing & Validation
- [ ] Run ingestion with `LIMIT_RECORDS=100` to test
- [ ] Verify data appears in properties table
- [ ] Verify API serves new NSW properties
- [ ] Verify frontend displays NSW data correctly
- [ ] Check `ingestion_runs` table for pipeline status
- [ ] Test error handling (bad URLs, network failures)

### Priority 3: Monitoring & Observability
- [ ] Add logging to files (`/var/log/ingestion/`)
- [ ] Query `ingestion_runs` table for monitoring
- [ ] Add health check endpoint for ingestion worker
- [ ] Consider Grafana/Prometheus metrics

### Priority 4: Production Readiness
- [ ] Set up proper NSW Rentals URL (monthly update process)
- [ ] Add retry logic for failed fetches
- [ ] Add email/Slack alerts for failures
- [ ] Document server deployment process
- [ ] Test cron schedule on server

---

## Notes
- Phase 1 (NSW ingestion) **compiles successfully** ✅
- Code is ready to run, needs Docker + cron integration
- All ingestion code uses functional design (no traits)
- Functional design makes it easy to add WA, Domain API, VIC next
- See `PHASE_1_COMPLETE.md` for full implementation details
- Database container: `real_estate-postgres-1`
- All tests passing as of 2025-10-02

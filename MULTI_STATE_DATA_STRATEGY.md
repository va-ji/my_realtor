# Multi-State Data Ingestion Strategy

## Overview
Comprehensive strategy for ingesting real estate data across all Australian states using a combination of free public datasets and Domain API.

## Data Sources by State

### 🟢 NSW (High Quality - Individual Properties)
**Sales Data:**
- Source: https://nswpropertysalesdata.com/data/archive.zip
- Format: CSV (66MB compressed, 250MB uncompressed)
- Update: Daily at 5am
- Records: 1.8M+ properties (6 years)
- Fields: Address, postcode, price, settlement date, property type
- Quality: ✅ Individual property records with full addresses

**Rental Data:**
- Source: https://www.nsw.gov.au/housing-and-construction/rental-forms-surveys-and-data/rental-bond-data
- Format: XLSX (monthly)
- Update: Monthly
- Fields: Postcode, dwelling type, weekly rent, bedrooms
- Quality: ✅ Median rents by postcode + bedroom count

**Matching Strategy:** Join sales by postcode + estimated bedrooms to rental data

---

### 🟡 Victoria (Medium Quality - Aggregated)
**Sales Data:**
- Source: discover.data.vic.gov.au + land.vic.gov.au
- Format: XLSX
- Update: Quarterly (March, June, Sept, Dec)
- Coverage: 93% of settled sales across state
- Fields: Median prices by suburb, property type, time series
- Quality: ⚠️ Aggregated by suburb, not individual properties

**Available Datasets:**
1. Victorian Property Sales Report (VPSR) - Quarterly medians by suburb
2. Time Series - 20 years of annual data by property type
3. Median House by Suburb - 10 year trends across 79 municipalities

**Rental Data:**
- Source: Consumer Affairs Victoria
- Likely aggregated by suburb/LGA

**Limitation:** No individual property addresses - only suburb-level medians

---

### 🟡 Queensland (Medium Quality - Limited)
**Sales Data:**
- Source: data.qld.gov.au
- Format: Various (cadastral boundaries, valuation data)
- Update: Varies
- Quality: ⚠️ Limited individual property sales data

**Available Data:**
1. Queensland Globe - Free valuation and property visualization
2. Cadastral Data - Parcel boundaries and lot descriptions
3. Valuation Property Boundaries - Spatial representation of valuations

**Limitation:** Most detailed sales data requires purchase from business centres

---

### 🟡 Western Australia (Good Quality - Individual)
**Sales Data:**
- Source: catalogue.data.wa.gov.au/dataset/sales-evidence-data
- Format: CSV/dataset
- Update: Historical (back to 1988)
- Fields: Last 3 sales per property, property attributes
- Quality: ✅ Individual property records

**Available Data:**
1. Sales Evidence Dataset - Last 3 sales per freehold/leasehold property
2. Property Statistics - Annual median prices, new lots created
3. Landgate property reports

**Limitation:** Personal Use License restrictions

---

### 🟡 South Australia (Medium Quality - Aggregated)
**Sales Data:**
- Source: data.sa.gov.au + valuergeneral.sa.gov.au
- Format: XLSX (quarterly)
- Update: Quarterly
- Fields: Median house prices by suburb
- Quality: ⚠️ Aggregated by suburb

**Available Datasets:**
1. Metro Median House Sales - Quarterly by suburb
2. Office of Valuer-General - Published statistics
3. SAPPA - Property and Planning Atlas (GIS-based)

**Limitation:** Aggregated data only - no individual property records

---

### 🔴 Tasmania, ACT, NT (Low Coverage)
**Status:** Limited free public data available
- May require Domain API or other commercial sources
- Consider lower priority for MVP

---

## Domain API Strategy

### Current Understanding
- **Access:** Developer portal at developer.domain.com.au
- **Authentication:** Client credentials (requires registration)
- **Free Tier:** Unclear if still available in 2025
- **Paid Tiers:** Business Plan provides richer data
- **Rate Limits:** Not publicly documented (needs investigation)

### Available Data (General)
- Agency and agent listings
- On-market and off-market properties
- Address lookup service
- Basic property data
- Recent sales data
- Price estimates (Business Plan)
- Suburb demographics (Business Plan)

### Recommendation
1. Register for Domain API developer account
2. Test free tier limits and capabilities
3. Use strategically to fill gaps:
   - Tasmania, ACT, NT (limited public data)
   - Rental data for VIC, QLD, WA, SA
   - Individual property records for states with only aggregated data
   - Real-time on-market listings

---

## Proposed Implementation Strategy

### Phase 1: NSW Only (MVP Foundation)
**Timeline:** Week 1-2
- Implement NSW sales + rental bond matching
- Build core ingestion pipeline
- Test yield calculations
- Validate data quality

**Outcome:** Working MVP with real NSW property data

---

### Phase 2: WA Integration
**Timeline:** Week 3
- Add WA sales evidence dataset
- Handle Personal Use License compliance
- Implement incremental updates for historical data

**Outcome:** Two-state coverage with individual property records

---

### Phase 3: Domain API Integration
**Timeline:** Week 4
- Register for Domain API access
- Test free tier limits
- Implement strategic API calls for:
  - VIC/QLD/SA rental data
  - TAS/ACT/NT property data
  - On-market listings for all states

**Outcome:** Rental data for all major states

---

### Phase 4: VIC/QLD/SA Aggregated Data
**Timeline:** Week 5
- Ingest suburb-level median data
- Create "representative properties" from aggregated stats
- Flag as "estimated" in database

**Outcome:** Full eastern seaboard + SA coverage

---

## Data Architecture

### Database Schema Updates Needed

```sql
-- Add data source tracking
ALTER TABLE properties ADD COLUMN data_source VARCHAR(50); -- 'NSW_SALES', 'WA_SALES', 'VIC_MEDIAN', 'DOMAIN_API'
ALTER TABLE properties ADD COLUMN data_quality VARCHAR(20); -- 'individual', 'aggregated', 'estimated'
ALTER TABLE properties ADD COLUMN postcode VARCHAR(10);     -- For NSW rental matching
ALTER TABLE properties ADD COLUMN property_type VARCHAR(50); -- 'house', 'unit', 'vacant_land', etc.
ALTER TABLE properties ADD COLUMN last_updated TIMESTAMP;

-- Add rental data table
CREATE TABLE rental_medians (
    id SERIAL PRIMARY KEY,
    state VARCHAR(3),
    postcode VARCHAR(10),
    suburb VARCHAR(100),
    bedrooms INTEGER,
    median_weekly_rent DECIMAL(10,2),
    data_source VARCHAR(50),
    period DATE, -- Month or quarter
    created_at TIMESTAMP DEFAULT NOW()
);

-- Add sales evidence tracking
CREATE TABLE sales_history (
    id SERIAL PRIMARY KEY,
    property_id INTEGER REFERENCES properties(id),
    sale_date DATE,
    price DECIMAL(12,2),
    data_source VARCHAR(50),
    settlement_date DATE,
    contract_date DATE,
    created_at TIMESTAMP DEFAULT NOW()
);
```

### Ingestion Pipeline Structure

```
backend/src/bin/data_ingestion/
├── main.rs                    # Orchestrator
├── sources/
│   ├── nsw_sales.rs          # NSW sales CSV parser
│   ├── nsw_rentals.rs        # NSW rental bond XLSX parser
│   ├── wa_sales.rs           # WA sales evidence parser
│   ├── vic_medians.rs        # VIC aggregated data parser
│   ├── qld_data.rs           # QLD data parser
│   ├── sa_medians.rs         # SA aggregated data parser
│   └── domain_api.rs         # Domain API client
├── parsers/
│   ├── csv.rs                # CSV parsing utilities
│   └── xlsx.rs               # XLSX parsing utilities
├── matchers/
│   └── postcode_bedroom.rs   # Postcode + bedroom matching logic
└── db/
    ├── properties.rs         # Property upsert logic
    ├── rentals.rs            # Rental median storage
    └── sales.rs              # Sales history tracking
```

### Cron Schedule

```cron
# NSW sales data (after 5am update)
0 6 * * * /app/data-ingestion --source=nsw-sales

# NSW rental bonds (monthly, 2nd of month)
0 7 2 * * /app/data-ingestion --source=nsw-rentals

# WA sales data (weekly)
0 8 * * 0 /app/data-ingestion --source=wa-sales

# VIC/QLD/SA quarterly data (5th of quarter month)
0 9 5 1,4,7,10 * /app/data-ingestion --source=vic-medians
0 10 5 1,4,7,10 * /app/data-ingestion --source=qld-data
0 11 5 1,4,7,10 * /app/data-ingestion --source=sa-medians

# Domain API listings (daily, if implemented)
0 12 * * * /app/data-ingestion --source=domain-api
```

---

## Cost Analysis

### Free Public Data
- NSW Sales: ✅ Free
- NSW Rentals: ✅ Free
- VIC Sales: ✅ Free (Creative Commons 4.0)
- WA Sales: ✅ Free (Personal Use License)
- SA Sales: ✅ Free
- QLD Limited: ✅ Free (full data requires purchase)

### Domain API (To Be Confirmed)
- Free Tier: Unknown (needs investigation)
- Business Plan: Pricing not public
- Rate Limits: Unknown

**Estimated Cost:** $0-$500/month depending on Domain API usage

---

## Data Quality Matrix

| State | Individual Properties | Rental Data | Update Frequency | Data Quality |
|-------|----------------------|-------------|------------------|--------------|
| NSW   | ✅ Yes (1.8M+)       | ✅ Postcode-level | Daily/Monthly | ⭐⭐⭐⭐⭐ |
| WA    | ✅ Yes (since 1988)  | ❓ TBD      | Historical       | ⭐⭐⭐⭐ |
| VIC   | ❌ Suburb medians    | ❓ TBD      | Quarterly        | ⭐⭐⭐ |
| QLD   | ⚠️ Limited          | ❓ TBD      | Varies           | ⭐⭐ |
| SA    | ❌ Suburb medians    | ❓ TBD      | Quarterly        | ⭐⭐⭐ |
| TAS   | ❌ Limited           | ❓ TBD      | Unknown          | ⭐ |
| ACT   | ❌ Limited           | ❓ TBD      | Unknown          | ⭐ |
| NT    | ❌ Limited           | ❓ TBD      | Unknown          | ⭐ |

---

## Next Actions

### Immediate (This Week)
1. ✅ Research completed
2. 🔲 Register for Domain API developer account
3. 🔲 Test Domain API free tier (if available)
4. 🔲 Start Phase 1: NSW ingestion pipeline

### Short Term (2-4 Weeks)
1. 🔲 Complete NSW implementation
2. 🔲 Add WA sales data
3. 🔲 Integrate Domain API for rentals

### Long Term (1-2 Months)
1. 🔲 Add VIC/QLD/SA aggregated data
2. 🔲 Implement TAS/ACT/NT coverage
3. 🔲 Add price history tracking
4. 🔲 Build suburb comparison features

---

## Questions for Domain API Testing

1. Is there still a free tier in 2025?
2. What are the rate limits? (requests/hour, requests/day)
3. Which endpoints are available on free tier?
4. Can we access rental data via API?
5. What is the cost of Business Plan?
6. Can we get individual property records or only listings?
7. How far back does historical sales data go?
8. Are there geographic restrictions?

---

## Conclusion

**Recommended Approach:**
1. Build MVP with NSW data (highest quality, free, individual properties)
2. Add WA for second state coverage
3. Test Domain API for filling gaps (rentals, other states)
4. Use aggregated data for VIC/SA as "estimated" properties
5. Prioritize NSW > WA > VIC > SA > QLD > TAS/ACT/NT

**Expected Outcome:**
- High-quality individual property data for NSW (~1.8M properties)
- Historical sales data for WA (since 1988)
- Suburb-level estimates for VIC, SA, QLD
- Domain API supplementing rental data and smaller states
- Fully automated daily/monthly updates

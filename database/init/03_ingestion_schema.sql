-- Schema updates for data ingestion pipeline
-- Adds multi-source tracking, data quality, and rental medians

-- Property types
CREATE TYPE property_type_enum AS ENUM ('house', 'unit', 'townhouse', 'vacant_land', 'commercial', 'other');

-- Data quality levels
CREATE TYPE data_quality_enum AS ENUM ('individual', 'aggregated', 'estimated', 'listing');

-- Update frequency
CREATE TYPE update_frequency_enum AS ENUM ('daily', 'weekly', 'monthly', 'quarterly', 'manual');

-- Extend properties table for multi-source data
ALTER TABLE properties ADD COLUMN IF NOT EXISTS postcode VARCHAR(10);
ALTER TABLE properties ADD COLUMN IF NOT EXISTS property_type property_type_enum;
ALTER TABLE properties ADD COLUMN IF NOT EXISTS bathrooms INTEGER;
ALTER TABLE properties ADD COLUMN IF NOT EXISTS land_area_sqm DECIMAL(10, 2);
ALTER TABLE properties ADD COLUMN IF NOT EXISTS sale_date DATE;

-- Data provenance and quality tracking
ALTER TABLE properties ADD COLUMN IF NOT EXISTS data_source VARCHAR(50);
ALTER TABLE properties ADD COLUMN IF NOT EXISTS data_quality data_quality_enum;
ALTER TABLE properties ADD COLUMN IF NOT EXISTS is_rental_estimated BOOLEAN DEFAULT FALSE;
ALTER TABLE properties ADD COLUMN IF NOT EXISTS confidence_score DECIMAL(3, 2) DEFAULT 1.0;
ALTER TABLE properties ADD COLUMN IF NOT EXISTS external_id VARCHAR(255);
ALTER TABLE properties ADD COLUMN IF NOT EXISTS last_updated TIMESTAMP DEFAULT NOW();

-- Rental medians table (for postcode + bedroom matching)
CREATE TABLE IF NOT EXISTS rental_medians (
    id SERIAL PRIMARY KEY,
    state state_enum NOT NULL,
    postcode VARCHAR(10) NOT NULL,
    suburb VARCHAR(100),
    bedrooms INTEGER NOT NULL,
    median_weekly_rent INTEGER NOT NULL,
    sample_size INTEGER, -- Number of bonds in this median
    data_source VARCHAR(50) NOT NULL,
    period DATE NOT NULL, -- Month or quarter this data represents
    created_at TIMESTAMP DEFAULT NOW(),

    -- Ensure we don't duplicate data
    UNIQUE(state, postcode, bedrooms, period, data_source)
);

-- Sales history tracking (multiple sales per property)
CREATE TABLE IF NOT EXISTS sales_history (
    id SERIAL PRIMARY KEY,
    property_id INTEGER REFERENCES properties(id) ON DELETE CASCADE,
    sale_price INTEGER NOT NULL,
    sale_date DATE NOT NULL,
    settlement_date DATE,
    contract_date DATE,
    data_source VARCHAR(50) NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Ingestion run tracking (when did we last run each source?)
CREATE TABLE IF NOT EXISTS ingestion_runs (
    id SERIAL PRIMARY KEY,
    source_id VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL, -- 'running', 'completed', 'failed'
    started_at TIMESTAMP NOT NULL,
    completed_at TIMESTAMP,
    records_fetched INTEGER DEFAULT 0,
    records_inserted INTEGER DEFAULT 0,
    records_updated INTEGER DEFAULT 0,
    records_skipped INTEGER DEFAULT 0,
    error_message TEXT,

    UNIQUE(source_id, started_at)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_properties_postcode ON properties(postcode);
CREATE INDEX IF NOT EXISTS idx_properties_data_source ON properties(data_source);
CREATE INDEX IF NOT EXISTS idx_properties_external_id ON properties(external_id);
CREATE INDEX IF NOT EXISTS idx_properties_state_postcode ON properties(state, postcode);

CREATE INDEX IF NOT EXISTS idx_rental_medians_lookup ON rental_medians(state, postcode, bedrooms, period DESC);
CREATE INDEX IF NOT EXISTS idx_rental_medians_period ON rental_medians(period DESC);

CREATE INDEX IF NOT EXISTS idx_sales_history_property ON sales_history(property_id, sale_date DESC);
CREATE INDEX IF NOT EXISTS idx_sales_history_date ON sales_history(sale_date DESC);

CREATE INDEX IF NOT EXISTS idx_ingestion_runs_source ON ingestion_runs(source_id, started_at DESC);

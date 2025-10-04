-- Optimized schema for storage-efficient data ingestion
-- Option A: Top 100K properties + aggregates (~400 MB storage)

-- Add new columns to existing properties table
ALTER TABLE properties
ADD COLUMN IF NOT EXISTS postcode VARCHAR(4),
ADD COLUMN IF NOT EXISTS sale_date DATE,
ADD COLUMN IF NOT EXISTS data_source VARCHAR(50) DEFAULT 'manual',
ADD COLUMN IF NOT EXISTS quality_score SMALLINT DEFAULT 5,
ADD COLUMN IF NOT EXISTS rental_yield DECIMAL(5,2);

-- Add unique constraint to prevent duplicates
ALTER TABLE properties
DROP CONSTRAINT IF EXISTS unique_property,
ADD CONSTRAINT unique_property UNIQUE (address, suburb, state, postcode);

-- Indexes for faster lookups
CREATE INDEX IF NOT EXISTS idx_properties_postcode ON properties(postcode);
CREATE INDEX IF NOT EXISTS idx_properties_state_suburb ON properties(state, suburb);
CREATE INDEX IF NOT EXISTS idx_properties_yield ON properties(rental_yield DESC NULLS LAST);
CREATE INDEX IF NOT EXISTS idx_properties_sale_date ON properties(sale_date DESC NULLS LAST);
CREATE INDEX IF NOT EXISTS idx_properties_data_source ON properties(data_source);

-- New table: suburb_statistics (aggregated data)
CREATE TABLE IF NOT EXISTS suburb_statistics (
    id SERIAL PRIMARY KEY,
    suburb VARCHAR(100) NOT NULL,
    postcode VARCHAR(4),
    state state_enum NOT NULL,
    bedrooms INTEGER,

    -- Core metrics (calculated from raw data)
    median_price INTEGER,
    median_weekly_rent INTEGER,
    median_rental_yield DECIMAL(5,2),
    avg_rental_yield DECIMAL(5,2),

    -- Distribution metrics
    property_count INTEGER,
    min_yield DECIMAL(5,2),
    max_yield DECIMAL(5,2),
    yield_25th_percentile DECIMAL(5,2),
    yield_75th_percentile DECIMAL(5,2),

    -- Price distribution
    price_25th_percentile INTEGER,
    price_75th_percentile INTEGER,

    -- Metadata
    calculated_date DATE NOT NULL,
    data_source VARCHAR(50),
    last_updated TIMESTAMP DEFAULT NOW(),

    UNIQUE(suburb, postcode, state, bedrooms, calculated_date)
);

-- Indexes for suburb_statistics
CREATE INDEX idx_suburb_stats_location ON suburb_statistics(state, suburb, bedrooms);
CREATE INDEX idx_suburb_stats_postcode ON suburb_statistics(postcode, bedrooms);
CREATE INDEX idx_suburb_stats_yield ON suburb_statistics(median_rental_yield DESC NULLS LAST);
CREATE INDEX idx_suburb_stats_date ON suburb_statistics(calculated_date DESC);

-- Update price_history to store monthly aggregates by suburb
ALTER TABLE price_history
ADD COLUMN IF NOT EXISTS suburb VARCHAR(100),
ADD COLUMN IF NOT EXISTS postcode VARCHAR(4),
ADD COLUMN IF NOT EXISTS bedrooms INTEGER,
ADD COLUMN IF NOT EXISTS data_source VARCHAR(50);

-- Change recorded_date to monthly snapshots
ALTER TABLE price_history
DROP CONSTRAINT IF EXISTS unique_price_history,
ADD CONSTRAINT unique_price_history UNIQUE (suburb, postcode, property_id, recorded_date);

-- Index for price history
CREATE INDEX IF NOT EXISTS idx_price_history_suburb ON price_history(suburb);
CREATE INDEX IF NOT EXISTS idx_price_history_postcode ON price_history(postcode);

-- Create table for ingestion logs
CREATE TABLE IF NOT EXISTS ingestion_logs (
    id SERIAL PRIMARY KEY,
    job_name VARCHAR(100) NOT NULL,
    state state_enum NOT NULL,
    status VARCHAR(20) NOT NULL, -- 'started', 'completed', 'failed'

    -- Metrics
    records_downloaded INTEGER,
    records_processed INTEGER,
    records_stored INTEGER,
    records_skipped INTEGER,

    -- Timing
    started_at TIMESTAMP NOT NULL,
    completed_at TIMESTAMP,
    duration_seconds INTEGER,

    -- Details
    error_message TEXT,
    metadata JSONB,

    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_ingestion_logs_date ON ingestion_logs(started_at DESC);
CREATE INDEX idx_ingestion_logs_status ON ingestion_logs(status, state);

-- Add comments for documentation
COMMENT ON TABLE suburb_statistics IS 'Aggregated property statistics by suburb/postcode to minimize storage';
COMMENT ON COLUMN suburb_statistics.calculated_date IS 'Date when statistics were calculated from raw data';
COMMENT ON TABLE ingestion_logs IS 'Tracks data ingestion jobs for monitoring and debugging';
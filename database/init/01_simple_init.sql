-- Simple database for MVP - just location and rent trends

-- Australian states
CREATE TYPE state_enum AS ENUM ('NSW', 'VIC', 'QLD', 'WA', 'SA', 'TAS', 'ACT', 'NT');

-- Main properties table - keep it simple
CREATE TABLE properties (
    id SERIAL PRIMARY KEY,
    address TEXT NOT NULL,
    suburb VARCHAR(100) NOT NULL,
    state state_enum NOT NULL,

    -- Core property data
    bedrooms INTEGER,
    price INTEGER, -- Purchase price
    weekly_rent INTEGER, -- Weekly rent

    -- Simple location (lat, lng) - no fancy PostGIS yet
    latitude DECIMAL(10, 8),
    longitude DECIMAL(11, 8),

    -- When we recorded this data
    created_at TIMESTAMP DEFAULT NOW()
);

-- Track price/rent changes over time for trends
CREATE TABLE price_history (
    id SERIAL PRIMARY KEY,
    property_id INTEGER REFERENCES properties(id),
    price INTEGER,
    weekly_rent INTEGER,
    recorded_date DATE NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Simple indexes for fast queries
CREATE INDEX idx_properties_suburb ON properties(suburb, state);
CREATE INDEX idx_properties_price ON properties(price);
CREATE INDEX idx_price_history_date ON price_history(recorded_date);
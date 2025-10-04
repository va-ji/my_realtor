# Database Storage Configuration

## Default Storage (SSD)

By default, PostgreSQL data is stored in `./database/data` relative to the project directory (likely on your SSD).

For development and testing, this is fine. Estimated size:
- Empty schema: ~50MB
- NSW sample (1000 records): ~100MB
- NSW full (~1.8M properties): ~2-5GB
- With indexes and history: ~5-10GB

## Moving Database to HDD

For production with full data ingestion, you can move database storage to an HDD to save SSD space.

### Step 1: Create Directory on HDD

```bash
# Example: HDD mounted at /mnt/hdd
sudo mkdir -p /mnt/hdd/real_estate/db_data
sudo chown -R $(whoami):$(whoami) /mnt/hdd/real_estate/db_data
```

### Step 2: Configure in .env.production

```bash
# Edit .env.production
nano .env.production

# Add this line (uncomment and update path):
DB_DATA_PATH=/mnt/hdd/real_estate/db_data
```

### Step 3: Deploy

```bash
./scripts/deploy.sh
```

Docker will automatically use the HDD path for PostgreSQL data.

## Moving Existing Data to HDD

If you already have data and want to move it:

```bash
# Stop services
docker-compose -f docker-compose.production.yml down

# Move existing data
sudo mv ./database/data/* /mnt/hdd/real_estate/db_data/

# Update .env.production with DB_DATA_PATH

# Restart
docker-compose -f docker-compose.production.yml up -d
```

## Storage Estimates by Phase

| Phase | Properties | Estimated Size |
|-------|-----------|----------------|
| Testing (100 records) | 100 | ~50MB |
| Testing (1000 records) | 1,000 | ~100MB |
| NSW only | ~1.8M | 5-10GB |
| NSW + WA | ~3M | 10-15GB |
| All states (future) | ~10M+ | 30-50GB |

## Performance Considerations

### SSD vs HDD

**SSD (recommended for):**
- Fast query performance
- Quick ingestion
- Real-time API responses
- 10GB is manageable on most SSDs

**HDD (acceptable for):**
- Initial testing and development
- Lower-traffic production use
- Slower ingestion (but runs daily overnight anyway)
- Queries will be 2-3x slower but still acceptable

### Hybrid Approach

**Best of both worlds:**
- Keep PostgreSQL **data** on HDD (`DB_DATA_PATH` on HDD)
- Keep PostgreSQL **WAL logs** on SSD for write performance
- Keep **temp files** on SSD

This requires custom PostgreSQL configuration (advanced - not needed for now).

## Checking Current Storage Usage

```bash
# Check Docker volume size
docker system df -v

# Check database size from inside PostgreSQL
docker exec -it real_estate-postgres psql -U realtor_user -d realtor_db -c "
  SELECT
    pg_size_pretty(pg_database_size('realtor_db')) as total_size,
    pg_size_pretty(pg_total_relation_size('properties')) as properties_table,
    pg_size_pretty(pg_total_relation_size('sales_history')) as sales_history;
"

# Check disk usage on host
du -sh ./database/data  # or your HDD path
```

## Recommendations

### For Initial Testing (Now)
- ✅ **Use default SSD storage** (./database/data)
- ✅ **Test with LIMIT_RECORDS=100** first
- ✅ **Run full NSW ingestion** (will be ~5-10GB)
- ✅ **Monitor disk usage**: `df -h`

**Why:** 10GB on SSD is fine for MVP, and performance will be better for development/testing.

### For Production (Later)
- Consider HDD if SSD space becomes constrained
- Or upgrade server with larger SSD
- Current setup easily scales to 50GB+ with no code changes

## Cleanup Old Data

If disk space becomes an issue:

```bash
# Remove old ingestion temp files
docker exec real_estate-ingestion rm -rf /tmp/real_estate_ingestion/*

# Vacuum database to reclaim space
docker exec -it real_estate-postgres psql -U realtor_user -d realtor_db -c "VACUUM FULL;"

# Prune old Docker images/volumes
docker system prune -a --volumes
```

## Monitoring Disk Usage

Add to your monitoring routine:

```bash
# Check disk usage
df -h

# Check database size
docker exec real_estate-postgres psql -U realtor_user -d realtor_db -c "
  SELECT pg_size_pretty(pg_database_size('realtor_db'));
"

# Check largest tables
docker exec real_estate-postgres psql -U realtor_user -d realtor_db -c "
  SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
  FROM pg_tables
  WHERE schemaname = 'public'
  ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
"
```

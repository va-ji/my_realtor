# Production Deployment Guide

## Overview

This guide covers deploying the Real Estate Analysis Tool to a production server using Docker and Docker Compose.

## Architecture

The production stack consists of:
1. **PostgreSQL Database** (PostGIS-enabled) - Port 5432
2. **API Server** (Rust/Axum) - Port 3001
3. **Ingestion Worker** (Rust with supercronic scheduler)

## Prerequisites

On your production server, ensure you have:
- Docker (20.10+)
- Docker Compose (2.0+)
- Git
- At least 4GB RAM
- At least 20GB disk space (for property data)

## Testing Locally Before Production

**Test the full production stack locally first:**

```bash
# From project root
./scripts/test-local-deployment.sh
```

This script will:
1. Build production Docker images locally
2. Start all services (postgres, api, ingestion)
3. Run health checks
4. Test ingestion with 100 records
5. Verify API responses

If all tests pass, you're ready to deploy to production!

## Deployment Methods

### Method 1: Deploy from Server (Recommended)

**Step 1: Clone repository on server**
```bash
ssh user@your-server.com
cd /opt  # or wherever you want to deploy
git clone https://github.com/yourusername/real_estate.git
cd real_estate
```

**Step 2: Configure environment**
```bash
cp .env.production .env.production.local
nano .env.production.local  # Edit with your production values
```

Update the following:
- `POSTGRES_PASSWORD` - Strong password for database
- `RUST_LOG` - Keep as `info` for production
- `LIMIT_RECORDS` - Set to `0` for full data ingestion

**Step 3: Run deployment script**
```bash
chmod +x scripts/deploy.sh
./scripts/deploy.sh
```

The script will:
1. Pull latest code
2. Apply database schema
3. Build Docker images
4. Start all services
5. Run health checks

**Step 4: Verify deployment**
```bash
# Check all services are running
docker-compose -f docker-compose.production.yml ps

# Test API endpoint
curl http://localhost:3001/api/health

# Test database connection
docker exec -it real_estate-postgres psql -U realtor_user -d realtor_db

# Run manual ingestion test (limited)
docker exec -e LIMIT_RECORDS=100 real_estate-ingestion data-ingestion nsw_sales
```

---

### Method 2: Deploy via SCP (Manual Transfer)

**Step 1: Prepare files locally**
```bash
# From your local machine, create a deployment package
tar -czf real_estate_deploy.tar.gz \
  backend/ \
  database/ \
  docker-compose.production.yml \
  .env.production \
  scripts/
```

**Step 2: Transfer to server**
```bash
scp real_estate_deploy.tar.gz user@your-server.com:/opt/
```

**Step 3: Extract and deploy on server**
```bash
ssh user@your-server.com
cd /opt
tar -xzf real_estate_deploy.tar.gz
cd real_estate

# Configure environment
nano .env.production  # Update with production values

# Deploy
chmod +x scripts/deploy.sh
./scripts/deploy.sh
```

---

## Files to Transfer (SCP Method)

If using SCP, you need these files/directories:

```
real_estate/
├── backend/
│   ├── src/                        # All Rust source code
│   ├── Cargo.toml                  # Rust dependencies
│   ├── Cargo.lock                  # Locked dependencies
│   ├── Dockerfile                  # API server Dockerfile
│   ├── Dockerfile.ingestion        # Ingestion worker Dockerfile
│   └── crontab                     # Cron schedule for ingestion
├── database/
│   └── init/
│       ├── 01_simple_init.sql      # Base schema
│       ├── 02_seed_data.sql        # Sample data (optional)
│       └── 03_ingestion_schema.sql # Multi-source ingestion schema
├── docker-compose.production.yml   # Production compose config
├── .env.production                 # Environment template
└── scripts/
    └── deploy.sh                   # Deployment script
```

**Minimum SCP command:**
```bash
scp -r backend database docker-compose.production.yml .env.production scripts user@your-server.com:/opt/real_estate/
```

---

## Configuration

### Environment Variables (.env.production)

```bash
# PostgreSQL
POSTGRES_PASSWORD=your_strong_password_here

# Logging
RUST_LOG=info

# Ingestion
LIMIT_RECORDS=0  # 0 = no limit (full production ingestion)

# Data Source URLs (update if they change)
NSW_SALES_URL=https://nswpropertysalesdata.com/data/archive.zip
NSW_RENTALS_URL=https://www.nsw.gov.au/sites/default/files/2024-12/rental-bond-data-december-2024.xlsx
```

### Cron Schedule (backend/crontab)

Default schedule:
- **NSW Sales**: Daily at 6am AEST (after NSW publishes at 5am)
- **NSW Rentals**: Monthly on the 5th at 7am AEST

To modify the schedule:
```bash
# Edit backend/crontab before building
nano backend/crontab

# Rebuild ingestion worker
docker-compose -f docker-compose.production.yml build ingestion-worker
docker-compose -f docker-compose.production.yml up -d ingestion-worker
```

---

## Post-Deployment

### 1. Initial Data Load

Run a manual ingestion to populate the database:

```bash
# Test with limited records first
docker exec -e LIMIT_RECORDS=1000 real_estate-ingestion data-ingestion nsw_sales

# Check results
docker exec -it real_estate-postgres psql -U realtor_user -d realtor_db \
  -c "SELECT COUNT(*) FROM properties WHERE data_source = 'nsw_sales';"

# If successful, run full ingestion
docker exec real_estate-ingestion data-ingestion nsw_sales
```

**Note**: Full NSW ingestion (~1.8M properties) may take 30-60 minutes on first run.

### 2. Verify Cron Schedule

Check that supercronic is running:
```bash
docker logs real_estate-ingestion
# Should see: "INFO[...] read crontab: /etc/crontab"
```

### 3. Monitor Logs

**All logs go through Docker's logging system** - use `docker logs` and `docker-compose logs`:

```bash
# All services (follow mode)
docker-compose -f docker-compose.production.yml logs -f

# Specific service
docker-compose -f docker-compose.production.yml logs -f api-server
docker-compose -f docker-compose.production.yml logs -f ingestion-worker
docker-compose -f docker-compose.production.yml logs -f postgres

# Recent logs only
docker logs --tail 100 real_estate-ingestion
docker logs --tail 100 real_estate-api

# Logs with timestamps
docker logs -t real_estate-ingestion

# Ingestion logs (inside container - written by cron jobs)
docker exec real_estate-ingestion tail -f /var/log/ingestion/nsw_sales.log
docker exec real_estate-ingestion tail -f /var/log/ingestion/nsw_rentals.log

# Set RUST_LOG=debug in .env.production for verbose debugging
```

### 4. Database Inspection

```bash
# Connect to database
docker exec -it real_estate-postgres psql -U realtor_user -d realtor_db

# Useful queries:
\dt                                    # List all tables
SELECT COUNT(*) FROM properties;       # Total properties
SELECT data_source, COUNT(*) FROM properties GROUP BY data_source;
SELECT * FROM ingestion_runs ORDER BY started_at DESC LIMIT 5;
```

---

## Maintenance

### Update Code

```bash
cd /opt/real_estate
git pull origin main
./scripts/deploy.sh
```

### View Service Status

```bash
docker-compose -f docker-compose.production.yml ps
```

### Restart Services

```bash
# All services
docker-compose -f docker-compose.production.yml restart

# Specific service
docker-compose -f docker-compose.production.yml restart api-server
docker-compose -f docker-compose.production.yml restart ingestion-worker
```

### Stop Services

```bash
docker-compose -f docker-compose.production.yml down
```

### Backup Database

```bash
# Create backup
docker exec real_estate-postgres pg_dump -U realtor_user realtor_db > backup_$(date +%Y%m%d).sql

# Restore from backup
docker exec -i real_estate-postgres psql -U realtor_user -d realtor_db < backup_20251004.sql
```

### Connect to Production DB from Local Machine (SSH Tunnel)

```bash
# Create SSH tunnel (from your local machine)
./scripts/local-db-tunnel.sh

# In another terminal, connect with psql
psql postgresql://realtor_user:PASSWORD@localhost:5433/realtor_db

# Or create local backup of production data
pg_dump -h localhost -p 5433 -U realtor_user realtor_db > prod_backup_$(date +%Y%m%d).sql
```

**Configure the tunnel script:**
```bash
# Edit connection details
export REMOTE_HOST=your-server.com
export REMOTE_USER=your-user
export LOCAL_PORT=5433  # Use 5433 to avoid conflict with local postgres

./scripts/local-db-tunnel.sh
```

### Manual Ingestion

```bash
# Run specific source
docker exec real_estate-ingestion data-ingestion nsw_sales
docker exec real_estate-ingestion data-ingestion nsw_rentals

# With limited records (testing)
docker exec -e LIMIT_RECORDS=100 real_estate-ingestion data-ingestion nsw_sales
```

---

## Troubleshooting

### Services not starting

```bash
# Check logs
docker-compose -f docker-compose.production.yml logs

# Check if ports are available
sudo netstat -tulpn | grep -E '3001|5432'
```

### Database connection errors

```bash
# Check database is running
docker exec real_estate-postgres pg_isready -U realtor_user -d realtor_db

# Check environment variables
docker exec real_estate-api env | grep DATABASE_URL
```

### Ingestion failures

```bash
# Check ingestion logs
docker exec real_estate-ingestion cat /var/log/ingestion/nsw_sales.log

# Check disk space
df -h

# Test URL accessibility
docker exec real_estate-ingestion curl -I https://nswpropertysalesdata.com/data/archive.zip
```

### API not responding

```bash
# Check API logs
docker logs real_estate-api

# Test health endpoint
curl -v http://localhost:3001/api/health

# Check if container is running
docker ps | grep api
```

---

## Security Considerations

1. **Change default passwords**: Update `POSTGRES_PASSWORD` in `.env.production`
2. **Firewall**: Only expose necessary ports (consider using nginx reverse proxy)
3. **HTTPS**: Set up SSL certificates (see nginx configuration below)
4. **Regular updates**: Keep Docker images and dependencies updated
5. **Backups**: Schedule regular database backups

---

## Optional: Nginx Reverse Proxy

To serve the API over HTTPS with a domain name:

**1. Uncomment nginx service in docker-compose.production.yml**

**2. Create nginx config:**
```bash
mkdir -p nginx
nano nginx/nginx.conf
```

```nginx
events {
    worker_connections 1024;
}

http {
    upstream api {
        server api-server:3001;
    }

    server {
        listen 80;
        server_name your-domain.com;

        location /api/ {
            proxy_pass http://api;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
        }

        location / {
            return 301 https://$server_name$request_uri;
        }
    }
}
```

**3. Set up SSL certificates** (using Let's Encrypt):
```bash
# Install certbot on server
sudo apt install certbot

# Generate certificates
sudo certbot certonly --standalone -d your-domain.com

# Copy to nginx directory
sudo cp /etc/letsencrypt/live/your-domain.com/*.pem nginx/ssl/
```

---

## Monitoring & Alerts

### Check Ingestion Status

```sql
-- Connect to database and query ingestion_runs table
SELECT
    source,
    status,
    records_processed,
    started_at,
    completed_at,
    completed_at - started_at as duration,
    error_message
FROM ingestion_runs
ORDER BY started_at DESC
LIMIT 10;
```

### Set up Alerts (Optional)

Consider adding:
- Email/Slack notifications for failed ingestions
- Grafana/Prometheus for metrics monitoring
- Log aggregation (ELK stack or similar)

---

## Performance Tuning

### PostgreSQL

For large datasets, consider tuning PostgreSQL:

```bash
# Edit postgresql.conf in container
docker exec -it real_estate-postgres bash
nano /var/lib/postgresql/data/postgresql.conf

# Increase shared_buffers, work_mem, maintenance_work_mem
# Restart database after changes
```

### Ingestion Performance

- **Batch size**: Modify `write.rs` batch sizes for your server specs
- **Parallel processing**: Consider adding parallel CSV parsing
- **Network**: Ensure good bandwidth for downloading large data files

---

## Next Steps

After successful deployment:

1. **Add WA Sales Data**: Implement Phase 2 (Western Australia)
2. **Domain API Integration**: Add Phase 3 (rental data for smaller states)
3. **Frontend Deployment**: Deploy SvelteKit frontend
4. **API Authentication**: Add auth if exposing publicly
5. **Rate Limiting**: Implement rate limiting on API endpoints

---

## Support

For issues or questions:
- Check logs first: `docker-compose -f docker-compose.production.yml logs`
- Review `PROJECT_CONTEXT.md` for architecture details
- Review `MULTI_STATE_DATA_STRATEGY.md` for data source info

---

## Quick Reference

```bash
# Deploy/Update
./scripts/deploy.sh

# View logs
docker-compose -f docker-compose.production.yml logs -f

# Restart all
docker-compose -f docker-compose.production.yml restart

# Manual ingestion
docker exec real_estate-ingestion data-ingestion nsw_sales

# Database access
docker exec -it real_estate-postgres psql -U realtor_user -d realtor_db

# Stop all
docker-compose -f docker-compose.production.yml down

# Backup database
docker exec real_estate-postgres pg_dump -U realtor_user realtor_db > backup.sql
```

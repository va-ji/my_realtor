# Scripts Directory

Automation and deployment scripts for the Real Estate Analysis Tool.

## Available Scripts

### 1. `deploy.sh` - Production Deployment
Automated production deployment with health checks.

```bash
./scripts/deploy.sh
```

**What it does:**
- Pulls latest code from git
- Applies database schema migrations
- Builds Docker images
- Starts all services (postgres, api, ingestion)
- Runs health checks on all services
- Shows service status and useful commands

**Requirements:**
- Must be run from project root
- Requires `.env.production` file configured

---

### 2. `test-local-deployment.sh` - Local Testing
Test the production stack locally before deploying to server.

```bash
./scripts/test-local-deployment.sh
```

**What it does:**
- Builds production Docker images locally
- Starts all services in production mode
- Runs comprehensive health checks
- Tests ingestion with 100 sample records
- Verifies database has data
- Tests API responses
- Shows next steps if successful

**Use this before deploying to production!**

---

### 3. `local-db-tunnel.sh` - SSH Database Tunnel
Connect to production database from your local machine.

```bash
# Configure connection
export REMOTE_HOST=your-server.com
export REMOTE_USER=your-user
export LOCAL_PORT=5433

# Start tunnel (runs in foreground)
./scripts/local-db-tunnel.sh
```

**Use cases:**
- Query production data locally
- Create backups of production database
- Run analytics on production data
- Debug production issues

**Example usage:**
```bash
# Terminal 1: Start tunnel
./scripts/local-db-tunnel.sh

# Terminal 2: Connect with psql
psql postgresql://realtor_user:PASSWORD@localhost:5433/realtor_db

# Terminal 2: Create backup
pg_dump -h localhost -p 5433 -U realtor_user realtor_db > prod_backup.sql
```

---

## Typical Workflow

### First Deployment

1. **Test locally:**
   ```bash
   ./scripts/test-local-deployment.sh
   ```

2. **If tests pass, deploy to server:**
   ```bash
   # Transfer files
   scp -r backend database docker-compose.production.yml .env.production scripts user@server:/opt/real_estate/

   # Deploy on server
   ssh user@server
   cd /opt/real_estate
   nano .env.production  # Update password
   ./scripts/deploy.sh
   ```

### Updates

1. **Test changes locally:**
   ```bash
   ./scripts/test-local-deployment.sh
   ```

2. **Deploy to server:**
   ```bash
   ssh user@server
   cd /opt/real_estate
   git pull origin main
   ./scripts/deploy.sh
   ```

### Debugging Production

1. **Check logs:**
   ```bash
   ssh user@server
   docker-compose -f docker-compose.production.yml logs -f
   ```

2. **Connect to production database:**
   ```bash
   # From local machine
   ./scripts/local-db-tunnel.sh

   # In another terminal
   psql postgresql://realtor_user:PASSWORD@localhost:5433/realtor_db
   ```

---

## Tips

- Always test locally before deploying to production
- Keep `.env.production` out of git (already in .gitignore)
- Use SSH tunnels instead of exposing database port publicly
- Set `RUST_LOG=debug` for verbose logging when debugging
- Use `LIMIT_RECORDS=100` when testing ingestion

---

## See Also

- `DEPLOYMENT.md` - Full deployment documentation
- `PROJECT_CONTEXT.md` - Project overview and architecture
- `MULTI_STATE_DATA_STRATEGY.md` - Data ingestion strategy

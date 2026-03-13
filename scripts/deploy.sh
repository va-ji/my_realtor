#!/bin/bash
set -e

echo "=================================================="
echo "Real Estate Analysis Tool - Production Deployment"
echo "=================================================="
echo ""

# Check if we're in the right directory
if [ ! -f "docker-compose.production.yml" ]; then
    echo "❌ Error: docker-compose.production.yml not found"
    echo "Please run this script from the project root directory"
    exit 1
fi

# Check if .env.production exists
if [ ! -f ".env.production" ]; then
    echo "⚠️  Warning: .env.production not found"
    echo "Creating from .env.example..."
    cp .env.example .env.production
    echo "⚠️  IMPORTANT: Edit .env.production with production values before continuing!"
    exit 1
fi

# Load environment variables
export $(cat .env.production | grep -v '^#' | xargs)

echo "📦 Step 1: Pulling latest code..."
git pull origin main || echo "⚠️  Warning: Could not pull from git (may be OK if deploying manually)"

echo ""
echo "🗄️  Step 2: Applying database schema..."
if docker ps | grep -q real_estate-postgres; then
    echo "Applying schema to existing database..."
    docker exec -i real_estate-postgres psql -U realtor_user -d realtor_db < database/init/03_ingestion_schema.sql
else
    echo "Database container not running. Schema will be applied on first startup."
fi

echo ""
echo "🔨 Step 3: Building Docker images..."
docker-compose -f docker-compose.production.yml build

echo ""
echo "🚀 Step 4: Starting services..."
docker-compose -f docker-compose.production.yml up -d

echo ""
echo "⏳ Waiting for services to be healthy..."
sleep 10

echo ""
echo "🏥 Step 5: Running health checks..."

# Check PostgreSQL
if docker exec real_estate-postgres pg_isready -U realtor_user -d realtor_db > /dev/null 2>&1; then
    echo "✅ PostgreSQL: healthy"
else
    echo "❌ PostgreSQL: not healthy"
fi

# Check API server
if curl -f http://localhost:3001/api/health > /dev/null 2>&1; then
    echo "✅ API Server: healthy"
else
    echo "⚠️  API Server: not responding yet (may still be starting up)"
fi

# Check ingestion worker
if docker ps | grep -q real_estate-ingestion; then
    echo "✅ Ingestion Worker: running"
else
    echo "❌ Ingestion Worker: not running"
fi

echo ""
echo "📊 Step 6: Checking service status..."
docker-compose -f docker-compose.production.yml ps

echo ""
echo "=================================================="
echo "✅ Deployment Complete!"
echo "=================================================="
echo ""
echo "Service URLs:"
echo "  API Server: http://localhost:3001/api/health"
echo "  Database: postgresql://realtor_user:***@localhost:5432/realtor_db"
echo ""
echo "View logs:"
echo "  All services:     docker-compose -f docker-compose.production.yml logs -f"
echo "  API server:       docker-compose -f docker-compose.production.yml logs -f api-server"
echo "  Ingestion worker: docker-compose -f docker-compose.production.yml logs -f ingestion-worker"
echo "  Ingestion logs:   docker exec real_estate-ingestion cat /var/log/ingestion/nsw_sales.log"
echo ""
echo "Manual ingestion test:"
echo "  docker exec real_estate-ingestion data-ingestion nsw_sales"
echo ""
echo "Next steps:"
echo "  1. Test manual ingestion with limited records:"
echo "     docker exec -e LIMIT_RECORDS=100 real_estate-ingestion data-ingestion nsw_sales"
echo "  2. Check database: docker exec -it real_estate-postgres psql -U realtor_user -d realtor_db"
echo "  3. Monitor cron logs: docker exec real_estate-ingestion tail -f /var/log/ingestion/nsw_sales.log"
echo ""

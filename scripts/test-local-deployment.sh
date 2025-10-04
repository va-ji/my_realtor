#!/bin/bash
# Test deployment locally before pushing to production
set -e

echo "=================================================="
echo "Local Production-Mode Testing"
echo "=================================================="
echo ""

# Check if docker-compose.production.yml exists
if [ ! -f "docker-compose.production.yml" ]; then
    echo "❌ Error: docker-compose.production.yml not found"
    echo "Run this script from project root"
    exit 1
fi

# Check if .env.production exists
if [ ! -f ".env.production" ]; then
    echo "⚠️  Creating .env.production from template..."
    cp .env.example .env.production
fi

echo "🧹 Cleaning up any existing local test deployment..."
docker-compose -f docker-compose.production.yml down -v 2>/dev/null || true

echo ""
echo "🏗️  Building images locally..."
docker-compose -f docker-compose.production.yml build

echo ""
echo "🚀 Starting services..."
docker-compose -f docker-compose.production.yml up -d

echo ""
echo "⏳ Waiting for services to be healthy..."
sleep 15

echo ""
echo "🏥 Health checks..."

# Check PostgreSQL
if docker exec real_estate-postgres pg_isready -U realtor_user -d realtor_db > /dev/null 2>&1; then
    echo "✅ PostgreSQL: healthy"
else
    echo "❌ PostgreSQL: not healthy"
    docker logs real_estate-postgres
    exit 1
fi

# Check API server
MAX_RETRIES=10
RETRY_COUNT=0
while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    if curl -f http://localhost:3001/api/health > /dev/null 2>&1; then
        echo "✅ API Server: healthy"
        break
    else
        RETRY_COUNT=$((RETRY_COUNT + 1))
        if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
            echo "❌ API Server: not responding after $MAX_RETRIES attempts"
            docker logs real_estate-api
            exit 1
        fi
        echo "⏳ API Server: waiting... (attempt $RETRY_COUNT/$MAX_RETRIES)"
        sleep 2
    fi
done

# Check ingestion worker
if docker ps | grep -q real_estate-ingestion; then
    echo "✅ Ingestion Worker: running"
else
    echo "❌ Ingestion Worker: not running"
    docker logs real_estate-ingestion
    exit 1
fi

echo ""
echo "🧪 Running test ingestion (100 records)..."
docker exec -e LIMIT_RECORDS=100 real_estate-ingestion data-ingestion nsw_sales

echo ""
echo "📊 Checking database records..."
RECORD_COUNT=$(docker exec real_estate-postgres psql -U realtor_user -d realtor_db -t -c "SELECT COUNT(*) FROM properties WHERE data_source = 'nsw_sales';")
echo "Properties ingested: $RECORD_COUNT"

if [ "$RECORD_COUNT" -gt 0 ]; then
    echo "✅ Ingestion test successful!"
else
    echo "❌ Ingestion test failed - no records found"
    exit 1
fi

echo ""
echo "📋 Testing API response..."
API_RESPONSE=$(curl -s http://localhost:3001/api/properties | jq -r 'length')
echo "API returned $API_RESPONSE properties"

if [ "$API_RESPONSE" -gt 0 ]; then
    echo "✅ API test successful!"
else
    echo "❌ API test failed - no properties returned"
    exit 1
fi

echo ""
echo "=================================================="
echo "✅ All Tests Passed!"
echo "=================================================="
echo ""
echo "Services running:"
docker-compose -f docker-compose.production.yml ps
echo ""
echo "View logs:"
echo "  docker-compose -f docker-compose.production.yml logs -f"
echo ""
echo "Stop and cleanup:"
echo "  docker-compose -f docker-compose.production.yml down -v"
echo ""
echo "Deploy to production:"
echo "  scp -r backend database docker-compose.production.yml .env.production scripts user@server:/opt/real_estate/"
echo "  ssh user@server 'cd /opt/real_estate && ./scripts/deploy.sh'"
echo ""

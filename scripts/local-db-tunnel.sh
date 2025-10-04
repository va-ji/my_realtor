#!/bin/bash
# SSH Tunnel to Production Database
# Allows connecting to production DB from local machine on localhost:5433

set -e

echo "=================================================="
echo "SSH Tunnel to Production Database"
echo "=================================================="
echo ""

# Configuration (edit these)
REMOTE_HOST="${REMOTE_HOST:-your-server.com}"
REMOTE_USER="${REMOTE_USER:-your-user}"
REMOTE_DB_PORT="${REMOTE_DB_PORT:-5432}"
LOCAL_PORT="${LOCAL_PORT:-5433}"

echo "Configuration:"
echo "  Remote: $REMOTE_USER@$REMOTE_HOST:$REMOTE_DB_PORT"
echo "  Local:  localhost:$LOCAL_PORT"
echo ""

# Check if port is already in use
if lsof -Pi :$LOCAL_PORT -sTCP:LISTEN -t >/dev/null 2>&1 ; then
    echo "⚠️  Warning: Port $LOCAL_PORT is already in use"
    echo "Kill existing process? (y/n)"
    read -r response
    if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
        lsof -ti:$LOCAL_PORT | xargs kill -9
        echo "✅ Killed existing process"
    else
        echo "❌ Aborting. Use LOCAL_PORT=5434 ./scripts/local-db-tunnel.sh to use different port"
        exit 1
    fi
fi

echo "🔗 Creating SSH tunnel..."
echo ""
echo "Connection string for local tools:"
echo "  postgresql://realtor_user:PASSWORD@localhost:$LOCAL_PORT/realtor_db"
echo ""
echo "Example usage:"
echo "  psql postgresql://realtor_user:PASSWORD@localhost:$LOCAL_PORT/realtor_db"
echo "  pg_dump -h localhost -p $LOCAL_PORT -U realtor_user realtor_db > prod_backup.sql"
echo ""
echo "Press Ctrl+C to stop tunnel"
echo ""

# Create tunnel (runs in foreground)
ssh -N -L $LOCAL_PORT:localhost:$REMOTE_DB_PORT $REMOTE_USER@$REMOTE_HOST

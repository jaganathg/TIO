#!/bin/bash
set -e

echo "🚀 Setting up Trading Intelligence Orchestrator development environment..."

# Function to get docker compose command
get_docker_compose_cmd() {
    if docker compose version >/dev/null 2>&1; then
        echo "docker compose"
    elif command -v docker-compose >/dev/null 2>&1; then
        echo "docker-compose"
    else
        echo "❌ Docker Compose is required but not installed. Aborting." >&2
        exit 1
    fi
}

# Check if required tools are installed
command -v docker >/dev/null 2>&1 || { echo "❌ Docker is required but not installed. Aborting." >&2; exit 1; }
#command -v docker-compose >/dev/null 2>&1 || { echo "❌ Docker Compose is required but not installed. Aborting." >&2; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "❌ Rust/Cargo is required but not installed. Aborting." >&2; exit 1; }
command -v uv >/dev/null 2>&1 || { echo "❌ uv is required but not installed. Aborting." >&2; exit 1; }

# Set docker compose command
COMPOSE_CMD=$(get_docker_compose_cmd)

# Create data directory for SQLite
echo "📁 Creating data directories..."
mkdir -p data

# Start Docker services
echo "🐳 Starting Docker services..."
cd docker
$COMPOSE_CMD up -d
cd ..

# Wait for services to be ready
echo "⏳ Waiting for services to start..."
sleep 15

# Check service health
echo "🏥 Checking service health..."
curl -f http://localhost:8086/ping > /dev/null 2>&1 && echo "✅ InfluxDB is ready" || echo "❌ InfluxDB failed to start"
curl -f http://localhost:8000/api/v1/heartbeat > /dev/null 2>&1 && echo "✅ ChromaDB is ready" || echo "❌ ChromaDB failed to start"
docker exec tio-redis redis-cli -a redispassword ping > /dev/null 2>&1 && echo "✅ Redis is ready" || echo "❌ Redis health check failed"

# Check if uv virtual environment exists
if [ ! -d ".venv" ]; then
    echo "🐍 Creating Python virtual environment with uv..."
    uv venv
fi

# Install dependencies with uv
echo "📦 Installing Python dependencies with uv..."
source .venv/bin/activate
uv pip install -r python/requirements.txt

# Build Rust workspace to check everything compiles
echo "🦀 Building Rust workspace..."
cargo check --workspace

echo "✅ Setup complete!"
echo ""
echo "🎯 Next steps:"
echo "  1. Get API keys and update config/development.toml"
echo "  2. Run: ./scripts/run-dev.sh to start development environment"
echo "  3. Access services at:"
echo "     - API Gateway: http://localhost:3000"
echo "     - Python Orchestrator: http://localhost:8001"
echo "     - Client UI: http://localhost:8080"
echo "     - InfluxDB UI: http://localhost:8086"
echo "     - ChromaDB: http://localhost:8000"
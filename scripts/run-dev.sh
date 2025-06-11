#!/bin/bash
set -e

echo "🚀 Starting Trading Intelligence Orchestrator development environment..."

# Ensure Docker services are running
echo "🐳 Starting Docker services..."
cd docker
docker-compose up -d
cd ..

# Wait a moment for services to be ready
sleep 5

# Activate uv Python virtual environment
echo "🐍 Activating Python environment with uv..."
source .venv/bin/activate

echo "🎯 Starting all services..."

# Start Python orchestrator in background
echo "  📡 Starting Python Orchestrator..."
cd python/orchestrator
python -m uvicorn main:app --reload --port 8001 &
PYTHON_PID=$!
cd ../..

# Start Rust API Gateway in background
echo "  🦀 Starting API Gateway..."
cd crates/api-gateway
cargo run &
RUST_PID=$!
cd ../..

# Start Dioxus client
echo "  🎨 Starting Client UI..."
cd crates/client
dx serve --hot-reload --port 8080 &
CLIENT_PID=$!
cd ../..

echo ""
echo "✅ All services started successfully!"
echo ""
echo "🌐 Access your services:"
echo "  📊 Client UI:           http://localhost:8080"
echo "  🔌 API Gateway:         http://localhost:3000"
echo "  🧠 Python Orchestrator: http://localhost:8001"
echo "  📈 InfluxDB UI:         http://localhost:8086"
echo "  🎯 ChromaDB:            http://localhost:8000"
echo ""
echo "📋 Logs:"
echo "  To see Python logs: tail -f python/orchestrator/app.log"
echo "  To see API Gateway logs: Check terminal output"
echo ""
echo "🛑 To stop all services: Press Ctrl+C"

# Function to cleanup processes on exit
cleanup() {
    echo "🛑 Stopping all services..."
    kill $PYTHON_PID $RUST_PID $CLIENT_PID 2>/dev/null || true
    cd docker && docker-compose stop && cd ..
    echo "✅ All services stopped"
    exit 0
}

# Trap Ctrl+C
trap cleanup INT

# Wait for user to stop
wait
#!/bin/bash
set -e

# Kill any existing processes
echo "🧹 Cleaning up existing processes..."
pkill -f 'target/debug/signaling_server' 2>/dev/null || true
pkill -f 'target/debug/desktop' 2>/dev/null || true
sleep 0.5

# Build
echo "🔨 Building binaries..."
cd "$(dirname "$0")/../../"
cargo build -p p2p_webrtc --bin signaling_server
cargo build -p app --features desktop --bin desktop

# Start signaling server in background
echo "🚀 Starting signaling server on port 3000..."
./target/debug/signaling_server 3000 &
SIGNALING_PID=$!
sleep 1

# Verify server is running
if ! kill -0 $SIGNALING_PID 2>/dev/null; then
    echo "❌ Signaling server failed to start"
    exit 1
fi

echo "✅ Signaling server started (PID: $SIGNALING_PID)"

# Start desktop instances
echo "🚀 Starting desktop instance 1..."
RUST_LOG=info ./target/debug/desktop &
DESKTOP_PID1=$!

echo "⏳ Waiting 1 second before starting desktop instance 2..."
sleep 1

echo "🚀 Starting desktop instance 2..."
RUST_LOG=info ./target/debug/desktop &
DESKTOP_PID2=$!

echo ""
echo "✅ P2P Test Environment Started!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Signaling Server PID: $SIGNALING_PID"
echo "Desktop Instance 1 PID: $DESKTOP_PID1"
echo "Desktop Instance 2 PID: $DESKTOP_PID2"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "To stop: Ctrl+C or run: pkill -f target/debug"
echo ""

# Wait for all processes
wait

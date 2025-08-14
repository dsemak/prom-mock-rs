#!/bin/bash
# Example usage of prom-mock-rs
# This script demonstrates how to test various mock operation modes

set -euo pipefail

echo "=== Starting prom-mock-rs ==="
cargo run -- --fixtures tests/fixtures.yaml &
MOCK_PID=$!
sleep 2

echo ""
echo "=== Test 1: Basic query from fixtures ==="
curl -s "http://127.0.0.1:19090/api/v1/query?query=up" | jq '.'

echo ""
echo "=== Test 2: Query range from fixtures ==="
curl -s "http://127.0.0.1:19090/api/v1/query_range?query=rate(http_requests_total{job=\"api\"}[5m])&start=now-15m&end=now&step=60s" | jq '.'

echo ""
echo "=== Test 3: Send data via Remote Write ==="
# Creating a simple remote write message in protobuf format
# For example, we'll use curl with binary data
echo "Sending remote write data..."

# This is an example - in reality you need proper protobuf
# curl -H "Content-Type: application/x-protobuf" -X POST http://127.0.0.1:19090/api/v1/write --data-binary "@test_data.pb"

echo ""
echo "=== Test 4: Get labels ==="
curl -s "http://127.0.0.1:19090/api/v1/labels" | jq '.'

echo ""
echo "=== Test 5: Get label values ==="
curl -s "http://127.0.0.1:19090/api/v1/label/__name__/values" | jq '.'

echo ""
echo "=== Test 6: Series ==="
curl -s "http://127.0.0.1:19090/api/v1/series" | jq '.'

echo ""
echo "=== Test 7: Simple query (in-memory) ==="
curl -s "http://127.0.0.1:19090/api/v1/query_simple?query=test_metric" | jq '.'

echo ""
echo "=== Cleanup ==="
kill $MOCK_PID
wait $MOCK_PID 2>/dev/null
echo "Mock stopped"
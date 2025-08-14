#!/bin/bash
# Simple API test script
# Basic health and functionality checks

set -euo pipefail

echo "Testing Prometheus Mock API..."

# Test healthcheck
echo "=== Health Check ==="
curl -s http://127.0.0.1:19090/healthz
echo ""

# Test query from fixtures  
echo "=== Query from fixtures ==="
curl -s "http://127.0.0.1:19090/api/v1/query?query=up"
echo ""

# Test labels (empty response expected for new in-memory storage)
echo "=== Labels API ==="
curl -s "http://127.0.0.1:19090/api/v1/labels"
echo ""

# Test query_simple (may return empty result)
echo "=== Simple Query ==="
curl -s "http://127.0.0.1:19090/api/v1/query_simple?query=test_metric"
echo ""
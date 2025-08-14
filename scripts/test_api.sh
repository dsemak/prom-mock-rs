#!/bin/bash
# API testing script for prom-mock-rs
# Tests various API endpoints

set -euo pipefail

echo "=== Testing prom-mock-rs API ==="
echo ""

echo "1. Health check:"
curl -s http://127.0.0.1:19090/healthz
echo ""
echo ""

echo "2. Query from fixtures (up):"
curl -s "http://127.0.0.1:19090/api/v1/query?query=up" | jq '.'
echo ""

echo "3. Query range from fixtures:"
curl -s "http://127.0.0.1:19090/api/v1/query_range?query=rate(http_requests_total{job=\"api\"}[5m])&start=now-15m&end=now&step=60s" | jq '.'
echo ""

echo "4. Labels API (empty in-memory storage):"
curl -s "http://127.0.0.1:19090/api/v1/labels" | jq '.'
echo ""

echo "5. Simple query (in-memory, empty result expected):"
curl -s "http://127.0.0.1:19090/api/v1/query_simple?query=test_metric" | jq '.'
echo ""

echo "6. Series API:"
curl -s "http://127.0.0.1:19090/api/v1/series" | jq '.'
echo ""

echo "=== Testing completed ==="
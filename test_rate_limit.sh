#!/bin/bash

# Test IP-based rate limiting on public endpoints
# This script tests the soft limit (10 req) and hard limit (30 req)

BASE_URL="http://localhost:3000"

echo "🧪 Testing IP-based Rate Limiting on Public Endpoints"
echo "========================================================"
echo ""

# Test 1: Health endpoint - should work fine under soft limit
echo "📊 Test 1: Sending 5 requests to /health (under soft limit)"
for i in {1..5}; do
  response=$(curl -s -w "\n%{http_code}" "$BASE_URL/health")
  http_code=$(echo "$response" | tail -n1)
  echo "Request $i: HTTP $http_code"
done
echo ""

# Test 2: Health endpoint - should start seeing backoff delays
echo "📊 Test 2: Sending 10 more requests to /health (over soft limit, backoff should apply)"
for i in {6..15}; do
  start_time=$(date +%s%N)
  response=$(curl -s -w "\n%{http_code}" "$BASE_URL/health")
  end_time=$(date +%s%N)
  http_code=$(echo "$response" | tail -n1)
  duration_ms=$(( (end_time - start_time) / 1000000 ))
  echo "Request $i: HTTP $http_code (took ${duration_ms}ms)"
done
echo ""

# Test 3: Health endpoint - should hit hard limit
echo "📊 Test 3: Sending 20 more requests to /health (should hit hard limit at 30)"
for i in {16..35}; do
  response=$(curl -s -w "\n%{http_code}" "$BASE_URL/health")
  http_code=$(echo "$response" | tail -n1)
  body=$(echo "$response" | head -n-1)
  
  if [ "$http_code" = "429" ]; then
    echo "Request $i: HTTP $http_code - RATE LIMITED! ✅"
    echo "Response: $body" | jq -r '.error.message' 2>/dev/null || echo "$body"
    break
  else
    echo "Request $i: HTTP $http_code"
  fi
done
echo ""

# Wait for rate limit to reset
echo "⏳ Waiting 65 seconds for rate limit to reset..."
sleep 65
echo ""

# Test 4: Verify rate limit has reset
echo "📊 Test 4: After reset - should work again"
response=$(curl -s -w "\n%{http_code}" "$BASE_URL/health")
http_code=$(echo "$response" | tail -n1)
echo "Request after reset: HTTP $http_code"
echo ""

echo "✅ Rate limiting test complete!"

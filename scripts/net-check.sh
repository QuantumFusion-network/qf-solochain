#!/bin/zsh

echo "=== Checking Relay Chain ==="
curl -s -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "system_chain"}' http://localhost:9950 | jq

echo "=== Checking Parachain ==="
curl -s -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "system_chain"}' http://localhost:9947 | jq

echo "=== Checking if blocks are being produced ==="
for i in {1..5}; do
  height=$(curl -s -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getHeader"}' http://localhost:9947 | jq -r '.result.number')
  echo "Block height: $height"
  sleep 2
done

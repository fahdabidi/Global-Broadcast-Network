#!/bin/bash
# entrypoint.sh — ECS container entrypoint wrapper.
# Injects GBN_INSTANCE_IPV4 from the ECS task metadata endpoint (awsvpc mode)
# before exec'ing the main binary. Falls back gracefully outside ECS.
set -e

if [ -n "${ECS_CONTAINER_METADATA_URI_V4:-}" ]; then
  _meta="$(curl -sf "${ECS_CONTAINER_METADATA_URI_V4}")"
  export GBN_INSTANCE_IPV4
  GBN_INSTANCE_IPV4="$(echo "$_meta" | python3 -c \
    "import sys,json; d=json.load(sys.stdin); print(d['Networks'][0]['IPv4Addresses'][0])")"
fi

exec "$@"

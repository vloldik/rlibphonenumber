#!/bin/bash
set -e

CRATE_NAME=$1

if [ -z "$CRATE_NAME" ]; then
  echo "Usage: $0 <crate_name>"
  exit 1
fi
VERSION=$(cargo metadata --format-version 1 --no-deps | jq -r ".packages[] | select(.name == \"$CRATE_NAME\") | .version")

if [ -z "$VERSION" ]; then
  echo "Error: Could not find version for crate $CRATE_NAME in workspace."
  exit 1
fi
echo "version=$VERSION" >> "$GITHUB_OUTPUT"

echo "Checking if $CRATE_NAME v$VERSION is already published on crates.io..."

HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "User-Agent: rlibphonenumber-ci (githubactions)" \
  "https://crates.io/api/v1/crates/$CRATE_NAME/$VERSION")

if[ "$HTTP_STATUS" -eq 200 ]; then
  echo "$CRATE_NAME v$VERSION is already published. Skipping."
  echo "published=false" >> "$GITHUB_OUTPUT"
else
  echo "$CRATE_NAME v$VERSION is not published yet. Publishing now..."
  cargo publish --package "$CRATE_NAME"
  echo "published=true" >> "$GITHUB_OUTPUT"

  echo "Waiting for crates.io index to update..."
  sleep 10
fi
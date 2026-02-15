#!/usr/bin/env bash

declare -A PROPERTIES
PROPERTIES[packageName]="artifactsmmo-openapi"
PROPERTIES[supportAsync]="false"
PROPERTIES[preferUnsignedInt]="true"

ADDITIONAL_PROPERTIES=""
for key in "${!PROPERTIES[@]}"; do
	ADDITIONAL_PROPERTIES+="${key}=${PROPERTIES[$key]},"
done

rm -rf src/ docs/ Cargo.toml git_push.sh openapitools.json README.md
npx @openapitools/openapi-generator-cli generate -i openapi.json -g rust -o . --additional-properties="$ADDITIONAL_PROPERTIES"

# fix errors in generation
find . -type f -name "*.rs" -exec sed -i 's/models::models::/models::/g' {} +

#!/usr/bin/env bash
set -euo pipefail

rm -rf src/
npx @openapitools/openapi-generator-cli generate -c ./generator-config.yaml -i https://api.artifactsmmo.com/openapi.json -o . 

# fix errors in generation
find . -type f -name "*.rs" -exec sed -i 's/models::models::/models::/g' {} +

# patch Cargo.toml for workspace
sed -i 's/^edition = .*/edition.workspace = true/' Cargo.toml
sed -i '/^edition.workspace/a publish.workspace = true' Cargo.toml
sed -i '/license/d' Cargo.toml

for dep in chrono reqwest-middleware reqwest serde serde_json serde_repr serde_with url; do
  sed -i "/^${dep} = /{ s/^${dep} = .*/${dep}.workspace = true/ }" Cargo.toml
done

cat >> Cargo.toml << 'TOML'

[lints.clippy]
needless_return = { level = "allow" }
empty_docs = { level = "allow" }
to_string_in_format_args = { level = "allow" }
derivable_impls = { level = "allow" }
TOML

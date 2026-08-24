#!/bin/bash
# Builds the browser (wasm) version in Docker and stages a playable page into
# dist/browser. SERVE=1 serves it on http://localhost:8080 afterwards.
set -euo pipefail
cd "$(dirname "$0")"

NAME=dr-rustario-vs-rustris
IMAGE=$NAME-browser

docker build . -t "$IMAGE" -f Dockerfile.browser \
  --build-arg "EMSDK_VERSION=${EMSDK_VERSION:-6.0.7}"
CONTAINER=$(docker create "$IMAGE")
trap 'docker rm -f "$CONTAINER" > /dev/null' EXIT

rm -rf dist/browser
mkdir -p dist/browser
# emcc keeps dashes in the .js but uses underscores for the .wasm it references
for f in "$NAME.js" "${NAME//-/_}.wasm"; do
  docker cp "$CONTAINER:/app/target/wasm32-unknown-emscripten/release/$f" dist/browser/
done
cp web/index.html dist/browser/

echo
ls -lh dist/browser
echo
echo "note: serve the wasm with gzip/brotli in production; it embeds all game assets"
if [ "${SERVE:-0}" = "1" ]; then
  exec ./serve-browser.sh
fi

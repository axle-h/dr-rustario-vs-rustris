#!/bin/bash
# Serves the staged browser build (dist/browser, see build-browser.sh) on localhost.
#
#   ./serve-browser.sh          -> http://localhost:8080
#   PORT=9000 ./serve-browser.sh
set -euo pipefail
cd "$(dirname "$0")"

if [ ! -f dist/browser/index.html ]; then
  echo "dist/browser is not staged: run ./build-browser.sh first" >&2
  exit 1
fi
PORT="${PORT:-8080}"
echo "serving dist/browser on http://localhost:$PORT (ctrl-c to stop)"
python3 -m http.server -d dist/browser "$PORT"

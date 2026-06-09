#!/bin/sh
# Re-pack and sign extensions/ab-connect into extensions/ab-connect.crx using the
# stable signing key, then print the extension id. Keeps the crx id (and thus the
# native-messaging allowed_origins + force-install policy) constant across versions.
#
# The private key lives at .secrets/ab-connect.pem and is git-ignored. To re-pack
# on another machine / in CI, restore it from a secret first (see RELEASING).
#
# After changing the extension:
#   1. bump "version" in extensions/ab-connect/manifest.json
#   2. bump <updatecheck version=...> in extensions/updates.xml to match
#   3. run this script
#   4. commit extensions/ab-connect.crx + updates.xml + manifest.json
set -e
cd "$(dirname "$0")/.."
KEY=.secrets/ab-connect.pem
EXT=extensions/ab-connect
CHROME="${CHROME_BIN:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"

if [ ! -f "$KEY" ]; then
  echo "error: $KEY missing. Restore the signing key (CI secret AB_CONNECT_PEM) before packing." >&2
  exit 1
fi

rm -f extensions/ab-connect.crx
"$CHROME" --pack-extension="$PWD/$EXT" --pack-extension-key="$PWD/$KEY" >/dev/null 2>&1 || true
[ -f extensions/ab-connect.crx ] || { echo "error: pack failed" >&2; exit 1; }

ID=$(openssl rsa -in "$KEY" -pubout -outform DER 2>/dev/null \
     | openssl dgst -sha256 -binary | xxd -p -c256 | head -c32 | tr '0-9a-f' 'a-p')
echo "packed extensions/ab-connect.crx"
echo "extension id: $ID"
echo "manifest version: $(grep -o '"version"[^,]*' "$EXT/manifest.json" | head -1)"
echo "updates.xml  version: $(grep -o "version='[^']*'" extensions/updates.xml | tail -1)"

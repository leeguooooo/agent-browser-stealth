#!/bin/sh
# Build the Chrome Web Store upload package extensions/ab-connect.zip (and a signed
# extensions/ab-connect.crx for reference) from extensions/ab-connect, keeping the
# extension id constant via the stable signing key + manifest "key".
#
# The id MUST stay ciiljdlhdpfckdcfkphgmfalanpdejep so the native-messaging
# allowed_origins and the force-install policy keep matching. The id is pinned by
# the "key" field in manifest.json (kept in the uploaded zip on purpose).
#
# The private key lives at .secrets/ab-connect.pem and is git-ignored.
#
# After changing the extension:
#   1. bump "version" in extensions/ab-connect/manifest.json
#   2. run this script
#   3. commit extensions/ab-connect.zip (+ .crx) + manifest.json
#   4. upload ab-connect.zip to the Web Store (see extensions/store/SUBMISSION.html)
set -e
cd "$(dirname "$0")/.."
KEY=.secrets/ab-connect.pem
EXT=extensions/ab-connect
CHROME="${CHROME_BIN:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"

# Web Store upload package (zip of the unpacked extension, dotfiles excluded).
rm -f extensions/ab-connect.zip
( cd "$EXT" && zip -rq ../ab-connect.zip . -x '.*' )
[ -f extensions/ab-connect.zip ] || { echo "error: zip failed" >&2; exit 1; }

# Signed crx (reference / non-store force-install for managed setups).
if [ -f "$KEY" ]; then
  rm -f extensions/ab-connect.crx
  "$CHROME" --pack-extension="$PWD/$EXT" --pack-extension-key="$PWD/$KEY" >/dev/null 2>&1 || true
  ID=$(openssl rsa -in "$KEY" -pubout -outform DER 2>/dev/null \
       | openssl dgst -sha256 -binary | xxd -p -c256 | head -c32 | tr '0-9a-f' 'a-p')
  echo "extension id: $ID"
else
  echo "note: $KEY missing — built zip only (no crx)."
fi
echo "packed extensions/ab-connect.zip"
echo "manifest version: $(grep -o '"version"[^,]*' "$EXT/manifest.json" | head -1)"

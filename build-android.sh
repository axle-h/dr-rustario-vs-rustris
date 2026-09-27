#!/bin/bash
# Builds the Android APK (arm64-v8a) into dist/.
#
#   ./build-android.sh             -> dist/dr-rustario-vs-rustris.apk
#   DEPLOY=1 ./build-android.sh    ... and installs it with the host's adb, on whichever device
#                                  it sees (USB debugging, or `adb connect` for wireless)
#   ABI=x86_64 ./build-android.sh  -> dist/dr-rustario-vs-rustris-x86_64.apk, for the emulator
#
# The APK is signed with android/release.keystore, which is made on the first build and which
# git ignores. Keep it: Android only installs an update signed by the key already installed,
# and uninstalling to change key deletes the config and high scores with the app. It is a
# sideloading key, so its password is not a secret; ANDROID_KEYSTORE and
# ANDROID_KEYSTORE_PASSWORD point it at another.
set -euo pipefail
cd "$(dirname "$0")"

NAME=dr-rustario-vs-rustris
ABI=${ABI:-arm64-v8a}
case "$ABI" in
  arm64-v8a) TARGET=aarch64-linux-android APK=$NAME ;;
  x86_64) TARGET=x86_64-linux-android APK=$NAME-x86_64 ;;
  *) echo "error: ABI is arm64-v8a or x86_64, not $ABI" >&2; exit 1 ;;
esac
IMAGE=$NAME-android-$ABI
KEYSTORE=${ANDROID_KEYSTORE:-android/release.keystore}
export KEYSTORE_PASSWORD=${ANDROID_KEYSTORE_PASSWORD:-dr-rustario-vs-rustris}
UNSIGNED=/app/android/app/build/outputs/apk/release/app-release-unsigned.apk

# versionName is the launcher's version, and versionCode the same number as MMmmpp
VERSION_NAME=$(sed -n 's/^version = "\(.*\)"/\1/p' launcher/Cargo.toml | head -1)
IFS=. read -r MAJOR MINOR PATCH <<< "$VERSION_NAME"
VERSION_CODE=$((MAJOR * 10000 + MINOR * 100 + PATCH))

KEYSTORE_DIR=$(realpath "$(dirname "$KEYSTORE")")
KEYSTORE_FILE=$(basename "$KEYSTORE")
# the toolchain runs as the host user, so that what it writes here is the host user's
as_host() {
  docker run --rm -u "$(id -u):$(id -g)" -e HOME=/tmp -e KEYSTORE_PASSWORD \
    -v "$KEYSTORE_DIR:/keys" "$@"
}

if [ ! -f "$KEYSTORE" ]; then
  docker build . -t "$IMAGE-toolchain" -f Dockerfile.android --target toolchain \
    --build-arg "ABI=$ABI" --build-arg "TARGET=$TARGET"
  echo "--- making $KEYSTORE"
  as_host "$IMAGE-toolchain" keytool -genkeypair -keystore "/keys/$KEYSTORE_FILE" \
    -alias "$NAME" -keyalg RSA -keysize 4096 -validity 36500 -dname "CN=$NAME" \
    -storepass:env KEYSTORE_PASSWORD -keypass:env KEYSTORE_PASSWORD
fi

docker build . -t "$IMAGE" -f Dockerfile.android \
  --build-arg "ABI=$ABI" --build-arg "TARGET=$TARGET" \
  --build-arg "VERSION_CODE=$VERSION_CODE" --build-arg "VERSION_NAME=$VERSION_NAME"

mkdir -p dist
rm -f "dist/$APK.apk"
as_host -v "$PWD/dist:/dist" "$IMAGE" apksigner sign \
  --ks "/keys/$KEYSTORE_FILE" --ks-pass env:KEYSTORE_PASSWORD \
  --out "/dist/$APK.apk" "$UNSIGNED"
rm -f "dist/$APK.apk.idsig"

echo "--- dist/$APK.apk $VERSION_NAME ($VERSION_CODE, $ABI)"
as_host -v "$PWD/dist:/dist" "$IMAGE" apksigner verify --print-certs "/dist/$APK.apk" \
  | grep -v WARNING
ls -lh "dist/$APK.apk"

if [ -n "${DEPLOY:-}" ]; then
  adb install -r "dist/$APK.apk"
fi

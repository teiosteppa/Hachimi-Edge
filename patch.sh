#!/bin/bash

# release, can be changed by passing as env var in command
BUILD_TYPE=1

APK_EXTRACT_DIR=umav
BASE_APK=umav_hachimi.apk
APK_ARM64_LIB_DIR="umav/lib/arm64-v8a"
APK_ARM_LIB_DIR="umav/lib/armeabi-v7a"
APKSIGNER="$HOME/Android/Sdk/build-tools/35.0.0/apksigner"

KEYSTORE=umav.keystore
APK=umav.apk

echo "-- Cleaning up"
rm -rf umav
rm -f BASE_APK

echo "-- Extracting APK"
rm -rf "$APK_EXTRACT_DIR"
unzip "$APK" -d "$APK_EXTRACT_DIR"

echo "-- [arm64] Copying libmain_orig.so"
cp "$APK_ARM64_LIB_DIR/libmain.so" "$APK_ARM64_LIB_DIR/libmain_orig.so"
echo "-- [arm64] Copying Hachimi"
cp "./build/aarch64-linux-android/$BUILD_TYPE/libhachimi.so" "$APK_ARM64_LIB_DIR/libmain.so"

echo "-- [armv7] Copying libmain_orig.so"
cp "$APK_ARM_LIB_DIR/libmain.so" "$APK_ARM_LIB_DIR/libmain_orig.so"
echo "-- [armv7] Copying Hachimi"
cp "./build/armv7-linux-androideabi/$BUILD_TYPE/libhachimi.so" "$APK_ARM_LIB_DIR/libmain.so"

echo "-- Repacking APK"
pushd "$APK_EXTRACT_DIR"
zip -r6 "$BASE_APK" .
zip -Z store "$BASE_APK" resources.arsc
popd

echo "-- Signing APK"
echo "(Password is securep@ssw0rd816-n if you're using UmaPatcher's keystore)"
printf 'securep@ssw0rd816-n' | "$APKSIGNER" sign --ks $KEYSTORE "BASE_APK"

echo "-- APK is stored at $(realpath $BASE_APK)"
#!/usr/bin/env bash
set -e

case "$OSTYPE" in
    darwin*)  OS="darwin" ;; 
    linux*)   OS="linux" ;;
    *)
        echo "Unknown OSTYPE: $OSTYPE"
        exit 1
        ;;
esac

if [[ -z "$ANDROID_NDK_ROOT" ]]; then
    echo "ANDROID_NDK_ROOT must be set"
    exit 1
fi

if [ "$RELEASE" = "1" ]; then
    CARGOARGS="$CARGOARGS --release"
    BUILD_TYPE="release"
else
    BUILD_TYPE="debug"
fi

TOOLCHAIN_DIR="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/$OS-x86_64"
SYSROOT="$TOOLCHAIN_DIR/sysroot"

export BINDGEN_EXTRA_CLANG_ARGS="--sysroot=$SYSROOT"
export RUSTFLAGS="-C link-args=-static-libstdc++ -C link-args=-lc++abi"

export CC_aarch64_linux_android="$TOOLCHAIN_DIR/bin/aarch64-linux-android24-clang"
export CXX_aarch64_linux_android="$TOOLCHAIN_DIR/bin/aarch64-linux-android24-clang++"
export AR_aarch64_linux_android="$TOOLCHAIN_DIR/bin/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TOOLCHAIN_DIR/bin/aarch64-linux-android24-clang"

export CC_x86_64_linux_android="$TOOLCHAIN_DIR/bin/x86_64-linux-android24-clang"
export CXX_x86_64_linux_android="$TOOLCHAIN_DIR/bin/x86_64-linux-android24-clang++"
export AR_x86_64_linux_android="$TOOLCHAIN_DIR/bin/llvm-ar"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$TOOLCHAIN_DIR/bin/x86_64-linux-android24-clang"

mkdir -p build
cargo build --target=aarch64-linux-android --target-dir=build $CARGOARGS
cargo build --target=x86_64-linux-android --target-dir=build $CARGOARGS

pushd build

cp "aarch64-linux-android/$BUILD_TYPE/libhachimi.so" libmain-arm64-v8a.so
cp "x86_64-linux-android/$BUILD_TYPE/libhachimi.so" libmain-x86_64.so

if [ "$RELEASE" = "1" ]; then
    ARM64_V8A_SHA1=($(sha1sum libmain-arm64-v8a.so))
    X86_64_SHA1=($(sha1sum libmain-x86_64.so))

    cat << EOF > sha1.json
{
    "libmain-arm64-v8a.so": "$ARM64_V8A_SHA1",
    "libmain-armeabi-v7a.so": "null",
    "libmain-x86_64.so": "0",
    "libmain-x86.so": "0"
}
EOF
fi

popd

#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
case "${PLATFORM_NAME:-}" in
  iphoneos) task_architectures="${ARCHS:-arm64}" ;;
  iphonesimulator) task_architectures="${ARCHS:-${NATIVE_ARCH_ACTUAL:-arm64}}" ;;
  *) echo 'Run this script from the Voxy Xcode target (iphoneos/iphonesimulator)' >&2; exit 1 ;;
esac
# Validate the entire architecture list before building any slice.
for task_architecture in $task_architectures; do
  case "$PLATFORM_NAME:$task_architecture" in
    iphoneos:arm64|iphonesimulator:arm64|iphonesimulator:x86_64) ;;
    *) echo "Unsupported iOS architecture: $PLATFORM_NAME:$task_architecture" >&2; exit 1 ;;
  esac
done
# Xcode does not inherit an interactive shell PATH.
export PATH="${VOXY_CARGO_BIN:-$HOME/.cargo/bin}:$PATH"
xcrun --sdk "$PLATFORM_NAME" --show-sdk-path >/dev/null
export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-15.0}"
export CARGO_TARGET_DIR="$PWD/target/ios"
case "${CONFIGURATION:-Debug}" in
  Debug) task_profile=dev; task_directory=debug ;;
  Release) task_profile=release; task_directory=release ;;
  *) echo 'Unsupported Xcode configuration' >&2; exit 1 ;;
esac
set --
for task_architecture in $task_architectures; do
  case "$PLATFORM_NAME:$task_architecture" in
    iphoneos:arm64) task_target=aarch64-apple-ios ;;
    iphonesimulator:arm64) task_target=aarch64-apple-ios-sim ;;
    iphonesimulator:x86_64) task_target=x86_64-apple-ios ;;
  esac
  cargo build --locked -p voxy_mobile --target "$task_target" --profile "$task_profile"
  set -- "$@" "$CARGO_TARGET_DIR/$task_target/$task_directory/libvoxy_mobile.a"
done
mkdir -p "$BUILT_PRODUCTS_DIR"
if [ "$#" -eq 1 ]; then
  cp "$1" "$BUILT_PRODUCTS_DIR/libvoxy_mobile.a"
else
  xcrun lipo -create "$@" -output "$BUILT_PRODUCTS_DIR/libvoxy_mobile.a"
fi

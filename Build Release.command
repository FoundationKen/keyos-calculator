#!/bin/bash
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"

case "$(uname -m)" in
    arm64|aarch64) calculator_sdk_arch="aarch64" ;;
    x86_64) calculator_sdk_arch="x86_64" ;;
    *) printf 'Unsupported Mac architecture.\n' >&2; exit 1 ;;
esac

export FOUNDATION_SDK_ROOT="${HOME}/.foundation/sdk/foundation-sdk-1.1.0-${calculator_sdk_arch}-apple-darwin"
export FOUNDATION_SDK_BIN="${FOUNDATION_SDK_ROOT}/bin"

if [[ ! -x "${FOUNDATION_SDK_BIN}/foundation" ]]; then
    printf 'Install Foundation SDK v1.1.0 at %s first.\n' "$FOUNDATION_SDK_ROOT" >&2
    exit 1
fi

if ! command -v nix >/dev/null 2>&1; then
    printf 'Nix is required to build this application.\n' >&2
    exit 1
fi

exec nix develop "$FOUNDATION_SDK_ROOT" \
    --command "${FOUNDATION_SDK_BIN}/foundation" build --release

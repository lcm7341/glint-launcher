#!/bin/sh
# needs cmake, ninja, clang-cl, lld-link and the xwin sdk in ~/.xwin
set -e
here=$(cd "$(dirname "$0")" && pwd)
cmake -S "$here" -B "$here/build" -G Ninja \
	-DCMAKE_TOOLCHAIN_FILE="$HOME/.xwin/clang-cl-x86.cmake" \
	-DCMAKE_BUILD_TYPE=Release
ninja -C "$here/build"
ls -la "$here/out/XInput9_1_0.dll"

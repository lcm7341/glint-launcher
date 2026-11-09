#!/usr/bin/env bash
# needs mingw-w64 (brew install mingw-w64)
set -euo pipefail
cd "$(dirname "$0")"

x86_64-w64-mingw32-gcc -shared -O2 -s -Wall -Wextra -Werror \
  -static-libgcc -Wl,--no-insert-timestamp \
  -o userenv.dll userenv_proxy.c userenv.def
echo "Built $(pwd)/userenv.dll"

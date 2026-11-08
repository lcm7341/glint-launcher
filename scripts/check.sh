#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

if [ -t 1 ]; then bold=$'\e[1m' red=$'\e[1;31m' green=$'\e[1;32m' reset=$'\e[0m'
else bold='' red='' green='' reset=''; fi

max_file_lines=500

check_file_sizes() {
  local oversized=0 file code_lines
  while IFS= read -r file; do
    code_lines=$(wc -l < "$file" | tr -d ' ')
    if [ "$code_lines" -gt "$max_file_lines" ]; then
      echo "$file: $code_lines lines (limit $max_file_lines)"
      oversized=1
    fi
  done < <(git ls-files --cached --others --exclude-standard '*.rs')
  return "$oversized"
}

# cargo fmt --all would format weft too
check_formatting() {
  git ls-files --cached --others --exclude-standard '*.rs' \
    | xargs rustfmt --edition 2024 --check
}

if [ "${1-}" = "--fix" ]; then
  git ls-files --cached --others --exclude-standard '*.rs' | xargs rustfmt --edition 2024
fi

steps=(
  "check_formatting"
  "check_file_sizes"
  "cargo build --workspace --all-features"
  "cargo clippy --workspace --all-targets --all-features -- -D warnings"
)

for step in "${steps[@]}"; do
  printf '\n%s==> %s%s\n' "$bold" "$step" "$reset"
  if ! $step; then
    printf '\n%sFAILED:%s %s\n' "$red" "$reset" "$step"
    exit 1
  fi
done

printf '\n%sAll checks passed.%s\n' "$green" "$reset"

#!/usr/bin/env bash
# Issue #9 - assert every `Commands::` variant in src/cli.rs is mentioned
# in README.md. Catches the "we shipped a new subcommand but never
# updated docs" docs-drift regression cheaply. No new dependencies; runs
# under bash + grep, ready to be added to .github/workflows/ci.yml.
#
# Exit 0 = every command is documented. Exit 1 = at least one CLI variant
# has no README mention (script prints the missing command names).

set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
README="$ROOT/README.md"
CLI="$ROOT/src/cli.rs"

if [ ! -f "$README" ]; then
  echo "ERROR: README.md not found at $README" >&2
  exit 1
fi

if [ ! -f "$CLI" ]; then
  echo "ERROR: src/cli.rs not found at $CLI" >&2
  exit 1
fi

# Extract every Commands variant name. `pub enum Commands { ... }` spans
# multiple lines, one variant per line (`Profile(ProfileArgs),`,
# `Playbook,`, etc.) - awk walks from the `enum Commands {` line to the
# matching closing `}` and pulls the leading identifier off each variant
# line.
missing=()
while IFS= read -r candidate; do
  if [ -z "$candidate" ]; then
    continue
  fi
  # README documents commands lowercase (`atsassin profile init`), while
  # the Rust enum variant is PascalCase (`Profile`) - match case-
  # insensitively on a word boundary. A substring match is sufficient;
  # could upgrade to a real Markdown parser but the existing toolchain
  # has no such dependency, and this catches "we forgot to document X"
  # without needing the README's exact backticked form.
  if grep -qiE "(^|[^a-z])${candidate}([^a-z]|$)" "$README"; then
    continue
  fi
  missing+=("$candidate")
done < <(awk '
  /pub enum Commands/ { inblock=1; next }
  inblock && /^}/ { inblock=0 }
  inblock {
    line=$0
    sub(/^[[:space:]]+/, "", line)
    if (match(line, /^[A-Z][A-Za-z0-9]*/)) {
      print substr(line, RSTART, RLENGTH)
    }
  }
' "$CLI" | sort -u)

if [ "${#missing[@]}" -eq 0 ]; then
  echo "OK: every Commands:: variant in src/cli.rs has a README mention."
  exit 0
fi

echo "MISSING (these Commands:: variants have no README mention):"
for m in "${missing[@]}"; do
  echo "  - $m"
done
exit 1

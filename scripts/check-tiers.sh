#!/usr/bin/env bash
# check-tiers.sh — CI gate: every crates/*/Cargo.toml must carry a valid
# [package.metadata.rustforge] tier = "<tier>" entry.
#
# Valid tiers: stable | beta | experimental | stub
#
# Exit code: 0 = all crates annotated correctly
#            1 = one or more crates are missing/invalid (offenders printed)
#
# Usage: bash scripts/check-tiers.sh

set -euo pipefail

CRATES_DIR="$(cd "$(dirname "$0")/../crates" && pwd)"
VALID_TIERS="stable|beta|experimental|stub"

errors=()
counts_stable=0
counts_beta=0
counts_experimental=0
counts_stub=0
total=0

for cargo_toml in "$CRATES_DIR"/*/Cargo.toml; do
  crate_dir="$(dirname "$cargo_toml")"
  crate_name="$(basename "$crate_dir")"
  total=$((total + 1))

  # Extract the tier value — look for: tier = "..." on its own line.
  # Use grep -oE to extract only the matched portion, then cut to get the value.
  # Works on both GNU grep (Linux/CI) and BSD grep (macOS).
  tier_value="$(grep -E '^tier[[:space:]]*=[[:space:]]*"[^"]*"' "$cargo_toml" \
    | grep -oE '"[^"]+"' \
    | tr -d '"' \
    | head -1 || true)"

  if [[ -z "$tier_value" ]]; then
    errors+=("MISSING tier annotation: $crate_name  ($cargo_toml)")
    continue
  fi

  case "$tier_value" in
    stable)       counts_stable=$((counts_stable + 1)) ;;
    beta)         counts_beta=$((counts_beta + 1)) ;;
    experimental) counts_experimental=$((counts_experimental + 1)) ;;
    stub)         counts_stub=$((counts_stub + 1)) ;;
    *)
      errors+=("INVALID tier \"$tier_value\" in $crate_name  ($cargo_toml)  [must be one of: $VALID_TIERS]")
      ;;
  esac
done

echo ""
echo "=== RustForge Tier Coverage (crates/*) ==="
printf "  %-16s %d\n" "stable:"       "$counts_stable"
printf "  %-16s %d\n" "beta:"         "$counts_beta"
printf "  %-16s %d\n" "experimental:" "$counts_experimental"
printf "  %-16s %d\n" "stub:"         "$counts_stub"
echo "  ─────────────────────────────────────"
printf "  %-16s %d\n" "Total:"        "$total"
echo ""

if [[ ${#errors[@]} -gt 0 ]]; then
  echo "TIER CHECK FAILED — ${#errors[@]} offender(s):"
  for err in "${errors[@]}"; do
    echo "  ! $err"
  done
  echo ""
  echo "Fix: add the following block to each offending Cargo.toml:"
  echo ""
  echo "  [package.metadata.rustforge]"
  echo "  tier = \"<stable|beta|experimental|stub>\""
  echo ""
  exit 1
fi

echo "Tier check PASSED — all $total crates annotated with a valid tier."
exit 0

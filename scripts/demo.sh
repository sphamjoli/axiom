#!/usr/bin/env bash
#
# End-to-end demo. Alice runs `axiom analyse` on links.axm and retry.axm to
# write certificates; Bob runs `axiom check` on every claim. Negative models,
# negative certificates and certificates mutated from Alice's are rejected.
#
# Prints one row per claim: the source, the claim, its kind, whether it is
# proven or estimated, and whether it is accepted or rejected with its rule
# code. A claim the checker rejects as false has the code `false-claim`; a
# model or certificate refused as a whole has the code `axiom` prints.
#
# Uses the binary in AXIOM_BIN when set, otherwise builds it with cargo.
# Exit status is non-zero if any row differs from the expected table.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

if [ -z "${AXIOM_BIN:-}" ]; then
  cargo build -q -p axiom-cli --locked
  AXIOM_BIN="$root/target/debug/axiom"
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
estimate=(--estimate --seed 7 --epsilon 1/10 --delta 1/100)

# The certainty a claim kind carries when it is accepted.
certainty() {
  if [ "$1" = Estimate ]; then echo estimated; else echo proven; fi
}

# Runs `axiom check` on a certificate and prints a row per claim, or one row
# with the refusal code when the certificate is refused as a whole.
check_rows() {
  local label="$1" cert="$2" out status line claim kind verdict
  status=0
  out="$("$AXIOM_BIN" check "$cert" 2>&1)" || status=$?
  if [ "$status" -gt 1 ]; then
    printf '%s | - | - | - | rejected | %s\n' "$label" \
      "$(printf '%s\n' "$out" | sed -n 's/^axiom: \([a-z0-9-]*\):.*/\1/p' | head -n 1)"
    return
  fi
  while IFS= read -r line; do
    case "$line" in
      "claim "*": accepted"* | "claim "*": rejected") ;;
      *) continue ;;
    esac
    claim="$(printf '%s\n' "$line" | cut -d' ' -f2)"
    kind="$(printf '%s\n' "$line" | cut -d' ' -f3 | tr -d ':')"
    case "$line" in
      *": rejected") verdict="rejected | false-claim" ;;
      *) verdict="accepted | -" ;;
    esac
    printf '%s | %s | %s | %s | %s\n' "$label" "$claim" "$kind" "$(certainty "$kind")" "$verdict"
  done <<<"$out"
}

# Runs `axiom analyse` on a model Alice should not be able to certify.
refuse_model_row() {
  local label="$1" model="$2" out status=0
  out="$("$AXIOM_BIN" analyse "$model" --cert "$work/refused.json" 2>&1)" || status=$?
  if [ "$status" -eq 0 ]; then
    printf '%s | - | - | - | accepted | -\n' "$label"
  else
    printf '%s | - | - | - | rejected | %s\n' "$label" \
      "$(printf '%s\n' "$out" | sed -n 's/^axiom: \([a-z0-9-]*\):.*/\1/p' | head -n 1)"
  fi
}

# Copies a certificate with one Perl substitution applied, failing if the
# substitution matched nothing.
mutate() {
  local from="$1" to="$2" expr="$3"
  perl -0pe "$expr" "$from" >"$to"
  if cmp -s "$from" "$to"; then
    echo "demo: mutation did not apply to $from" >&2
    exit 3
  fi
}

"$AXIOM_BIN" analyse fixtures/models/links.axm --cert "$work/links.json" "${estimate[@]}" >/dev/null
"$AXIOM_BIN" analyse fixtures/models/retry.axm --cert "$work/retry.json" "${estimate[@]}" >/dev/null

mutate "$work/links.json" "$work/links-factor.json" \
  's/("kind": "Contraction",\s*"factor": \{\s*"num": ")3("\s*,\s*"den": ")5"/${1}1${2}2"/'
mutate "$work/links.json" "$work/links-steps.json" 's/"steps": 13/"steps": 12/'
mutate "$work/links.json" "$work/links-samples.json" 's/"samples": 265/"samples": 100/'
mutate "$work/retry.json" "$work/retry-attempts.json" \
  's/("expected_attempts": \{\s*"num": ")10("\s*,\s*"den": ")3"/${1}3${2}1"/'

actual="$work/actual.txt"
{
  check_rows "alice links.axm" "$work/links.json"
  check_rows "alice retry.axm" "$work/retry.json"
  check_rows "mutant links contraction 1/2" "$work/links-factor.json"
  check_rows "mutant links 12 steps" "$work/links-steps.json"
  check_rows "mutant links 100 samples" "$work/links-samples.json"
  check_rows "mutant retry 3 attempts" "$work/retry-attempts.json"
  for name in wrong-fixed-point termination-wrong-probability convergence-bound-on-retry estimate-steady-state-on-retry; do
    check_rows "negative $name.json" "fixtures/certificates/reject/$name.json"
  done
  for name in not-normalised weight-above-one; do
    refuse_model_row "negative $name.axm" "fixtures/models/reject/$name.axm"
  done
} >"$actual"

expected="$work/expected.txt"
cat >"$expected" <<'EOF'
alice links.axm | 0 | Mass | proven | accepted | -
alice links.axm | 1 | Mass | proven | accepted | -
alice links.axm | 2 | FixedPoint | proven | accepted | -
alice links.axm | 3 | Contraction | proven | accepted | -
alice links.axm | 4 | ConvergenceBound | proven | accepted | -
alice links.axm | 5 | Estimate | estimated | accepted | -
alice retry.axm | 0 | Termination | proven | accepted | -
alice retry.axm | 1 | Estimate | estimated | accepted | -
mutant links contraction 1/2 | 0 | Mass | proven | accepted | -
mutant links contraction 1/2 | 1 | Mass | proven | accepted | -
mutant links contraction 1/2 | 2 | FixedPoint | proven | accepted | -
mutant links contraction 1/2 | 3 | Contraction | proven | rejected | false-claim
mutant links contraction 1/2 | 4 | ConvergenceBound | proven | accepted | -
mutant links contraction 1/2 | 5 | Estimate | estimated | accepted | -
mutant links 12 steps | 0 | Mass | proven | accepted | -
mutant links 12 steps | 1 | Mass | proven | accepted | -
mutant links 12 steps | 2 | FixedPoint | proven | accepted | -
mutant links 12 steps | 3 | Contraction | proven | accepted | -
mutant links 12 steps | 4 | ConvergenceBound | proven | rejected | false-claim
mutant links 12 steps | 5 | Estimate | estimated | accepted | -
mutant links 100 samples | 0 | Mass | proven | accepted | -
mutant links 100 samples | 1 | Mass | proven | accepted | -
mutant links 100 samples | 2 | FixedPoint | proven | accepted | -
mutant links 100 samples | 3 | Contraction | proven | accepted | -
mutant links 100 samples | 4 | ConvergenceBound | proven | accepted | -
mutant links 100 samples | 5 | Estimate | estimated | rejected | false-claim
mutant retry 3 attempts | 0 | Termination | proven | rejected | false-claim
mutant retry 3 attempts | 1 | Estimate | estimated | accepted | -
negative wrong-fixed-point.json | 0 | Mass | proven | accepted | -
negative wrong-fixed-point.json | 1 | Mass | proven | accepted | -
negative wrong-fixed-point.json | 2 | FixedPoint | proven | rejected | false-claim
negative termination-wrong-probability.json | 0 | Termination | proven | rejected | false-claim
negative convergence-bound-on-retry.json | - | - | - | rejected | checker-not-chain
negative estimate-steady-state-on-retry.json | - | - | - | rejected | checker-not-chain
negative not-normalised.axm | - | - | - | rejected | row-not-normalised
negative weight-above-one.axm | - | - | - | rejected | weight-above-one
EOF

printf 'source | claim | kind | certainty | verdict | rule\n'
cat "$actual"
if ! diff -u "$expected" "$actual" >&2; then
  echo "demo: outcomes differ from the expected table" >&2
  exit 1
fi
echo "demo: every outcome is as expected"

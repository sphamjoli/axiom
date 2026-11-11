#!/usr/bin/env bash
#
# Fast textual pre-check for Lean constructs that weaken or enlarge the trusted
# computing base: `sorry`, `admit`, `axiom` declarations, `native_decide` and
# its kernel-bypass relatives, `implemented_by`, `extern`, and `unsafe`.
# Comments and string literals are blanked first, keeping line numbers, so
# prose may name these words freely. The authority is the kernel audit in
# scripts/checkLeanAxioms.sh; this check only fails fast without a build.
# With file arguments it checks those files; without, every .lean file under
# lean/ outside the .lake build directory.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

files=()
for arg in "$@"; do
  case "${arg}" in
    *.lean) files+=("${arg}") ;;
  esac
done

if [ "${#files[@]}" -eq 0 ]; then
  while IFS= read -r file; do
    files+=("${file}")
  done < <(find lean -name '*.lean' -not -path '*/.lake/*' 2>/dev/null)
fi

[ "${#files[@]}" -eq 0 ] && exit 0

# Blanks `--` line comments, nested `/- -/` blocks, and string literals, then
# prints `file:line: text` for each remaining line that names a forbidden word.
# `axiom` is matched only as a declaration keyword, after any attributes and
# modifiers. Single quotes keep the perl variables away from bash.
# shellcheck disable=SC2016
strip_and_match='
  my $depth = 0;
  my $in_string = 0;
  my $out = "";
  for (my $i = 0; $i < length; $i++) {
    my $c = substr($_, $i, 1);
    my $pair = substr($_, $i, 2);
    if ($c eq "\n") { $out .= $c; next; }
    if ($depth > 0) {
      if ($pair eq "/-") { $depth++; $i++; }
      elsif ($pair eq "-/") { $depth--; $i++; }
      next;
    }
    if ($in_string) {
      if ($c eq "\\") { $i++; }
      elsif ($c eq "\"") { $in_string = 0; }
      next;
    }
    if ($pair eq "/-") { $depth = 1; $i++; next; }
    if ($pair eq "--") { $i++ while $i + 1 < length && substr($_, $i + 1, 1) ne "\n"; next; }
    if ($c eq "\"") { $in_string = 1; next; }
    $out .= $c;
  }
  my $line_number = 0;
  my $word = qr/(?<![\w\x27])(sorry|sorryAx|admit|native_decide|ofReduceBool|skipKernelTC|implemented_by|extern|unsafe)(?![\w\x27])/;
  my $native = qr/\bdecide\s+\+native\b/;
  my $modifier = qr/(?:\@\[[^\]]*\]\s*|(?:private|protected|noncomputable|partial|nonrec)\s+)*/;
  my $axiom = qr/^\s*$modifier(?:axiom)(?![\w\x27])/;
  for my $line (split /\n/, $out, -1) {
    $line_number++;
    print "$ARGV:$line_number: $line\n" if $line =~ $word || $line =~ $native || $line =~ $axiom;
  }
'

if matches="$(perl -0777 -ne "${strip_and_match}" "${files[@]}")" && [ -n "${matches}" ]; then
  echo "checkLean: forbidden constructs found:" >&2
  echo "${matches}" >&2
  exit 1
fi

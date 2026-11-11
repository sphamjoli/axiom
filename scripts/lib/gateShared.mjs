// Shared plumbing for the diff-scoped source gates (decorative separators,
// trailing comments, Markdown wrap). Each gate supplies its own rule and file
// extensions; file discovery and "only fail on lines this change adds" diff
// scoping live here so a rule change never forks the scoping logic. Runs under
// bun or node.

import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const REPO_ROOT = join(fileURLToPath(import.meta.url), "..", "..", "..");
const EXCLUDED_SEGMENTS = new Set(["node_modules", "target", ".git", ".lake", ".local"]);

function isCheckableFile(path, extensions) {
  return extensions.some((extension) => path.endsWith(extension));
}

function walk(dir, out, extensions) {
  for (const entry of readdirSync(dir)) {
    if (EXCLUDED_SEGMENTS.has(entry)) continue;
    const full = join(dir, entry);
    const stats = statSync(full);
    if (stats.isDirectory()) walk(full, out, extensions);
    else if (isCheckableFile(full, extensions)) out.push(full);
  }
}

function collectDefaultFiles(extensions) {
  const files = [];
  walk(REPO_ROOT, files, extensions);
  return files;
}

function git(args) {
  return execFileSync("git", args, { encoding: "utf8" });
}

// Line numbers added or changed on the new side of a diff, so a diff-scoped
// gate blocks only violations being introduced now, never pre-existing debt on
// untouched lines. `range` selects the diff: ["--cached"] for the staged index,
// ["<base>...HEAD"] for a branch comparison. Empty set on any git error
// (treated as "nothing newly added", so the file cannot block).
function diffAddedLines(file, range) {
  const added = new Set();
  let diff;
  try {
    diff = git(["diff", ...range, "--unified=0", "--", file]);
  } catch {
    return added;
  }
  const hunkHeader = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@/;
  for (const line of diff.split("\n")) {
    const match = hunkHeader.exec(line);
    if (!match) continue;
    const start = Number(match[1]);
    const count = match[2] === undefined ? 1 : Number(match[2]);
    for (let lineNumber = start; lineNumber < start + count; lineNumber += 1) added.add(lineNumber);
  }
  return added;
}

function changedFiles(range, extensions) {
  try {
    return git(["diff", ...range, "--name-only", "--diff-filter=ACM"])
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => isCheckableFile(line, extensions));
  } catch {
    return [];
  }
}

// Resolve the value of a `--base <ref>` / `--base=<ref>` flag, or null.
function baseRef(argv) {
  const eq = argv.find((arg) => arg.startsWith("--base="));
  if (eq) return eq.slice("--base=".length);
  const idx = argv.indexOf("--base");
  if (idx !== -1 && argv[idx + 1]) return argv[idx + 1];
  return null;
}

// Turn a gate's CLI arguments into the set of files to scan and the git range
// (or null) whose newly added lines a finding must fall on to count. Explicit
// checkable file arguments win; otherwise a diff mode lists its own changed
// files and a full run walks the workspace. `extensions` scopes which paths
// count as checkable.
function resolveScope(argv, extensions) {
  const staged = argv.includes("--staged");
  const base = baseRef(argv);
  const range = staged ? ["--cached"] : base ? [`${base}...HEAD`] : null;
  const explicit = argv.filter((arg) => isCheckableFile(arg, extensions));
  const files =
    explicit.length > 0
      ? explicit
      : range
        ? changedFiles(range, extensions)
        : collectDefaultFiles(extensions);
  return { range, files };
}

// Keep only findings on lines the change adds or modifies (diff modes); a full
// run has no range and keeps everything. Findings must carry `path` and `line`.
function filterToDiff(findings, range) {
  if (!range) return findings;
  const addedByFile = new Map();
  return findings.filter((finding) => {
    let added = addedByFile.get(finding.path);
    if (!added) {
      added = diffAddedLines(finding.path, range);
      addedByFile.set(finding.path, added);
    }
    return added.has(finding.line);
  });
}

// Run one gate end to end: resolve the scope from argv, scan each file with the
// gate's `checkSource(text, path, findings)` visitor,
// keep only diff-scoped findings, and either exit 0 (clean) or print the gate's
// report and exit 1. `report.header(count)` and `report.footer` frame the list;
// `report.line(finding)` renders one finding (findings carry at least path/line).
// `gate.extensions` lists the file extensions the gate scans.
function runGate(argv, gate) {
  const { range, files } = resolveScope(argv, gate.extensions);
  const findings = [];
  for (const file of files) {
    let source;
    try {
      source = readFileSync(file, "utf8");
    } catch {
      continue;
    }
    gate.checkSource(source, file, findings);
  }

  const gated = filterToDiff(findings, range);
  if (gated.length === 0) process.exit(0);

  gated.sort((left, right) => left.path.localeCompare(right.path) || left.line - right.line);
  console.error(gate.report.header(gated.length));
  for (const finding of gated) console.error(gate.report.line(finding));
  console.error(gate.report.footer);
  process.exit(1);
}

export { REPO_ROOT, relative, isCheckableFile, resolveScope, filterToDiff, runGate };

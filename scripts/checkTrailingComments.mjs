// Fails when a line comment trails code on the same line. A comment belongs on
// its own line above what it describes, so the intent reads before the code
// rather than crowding its right margin.
//
// JavaScript comments come from the TypeScript parser, so a `//` inside a
// string, template, or regex literal is never mistaken for a comment. Rust and
// Lean lines are matched with regexes, which accept a false positive on a
// comment marker inside a string literal. Diff-scoped by the shared runner:
// only lines this change adds can fail. Runs under bun or node. Pass file paths
// to check a subset (the pre-commit hook passes staged files); pass none to
// walk the whole workspace.

import { pathToFileURL } from "node:url";
import ts from "typescript";
import { relative, REPO_ROOT, runGate } from "./lib/gateShared.mjs";

const CHECKABLE_EXTENSIONS = [".rs", ".lean", ".mjs"];

// Zero-based line index of a character offset within some text.
function lineIndexAt(text, offset) {
  return text.slice(0, offset).split("\n").length - 1;
}

// Single-line comments with their raw text and start offset. Comments are read
// from the parsed source, not a bare scanner: only the parser tracks when a `/`
// opens a regex literal or a `}` resumes a template, so a `//` inside a regex
// (`/\//g`) or an interpolated template (`` `${x} // y` ``) is never mistaken
// for a comment. Block comments are excluded: this gate targets the `//`
// trailing comment. Ranges are deduped because one comment can be both the
// trailing trivia of one node and the leading trivia of the next.
function scanLineComments(text) {
  const sourceFile = ts.createSourceFile("scan.mjs", text, ts.ScriptTarget.Latest, true);
  const comments = [];
  const seen = new Set();

  const collect = (ranges) => {
    if (!ranges) return;
    for (const range of ranges) {
      if (range.kind !== ts.SyntaxKind.SingleLineCommentTrivia) continue;
      if (seen.has(range.pos)) continue;
      seen.add(range.pos);
      comments.push({ text: text.slice(range.pos, range.end), start: range.pos });
    }
  };

  const visit = (node) => {
    collect(ts.getLeadingCommentRanges(text, node.getFullStart()));
    collect(ts.getTrailingCommentRanges(text, node.getEnd()));
    node.forEachChild(visit);
  };
  visit(sourceFile);

  return comments;
}

// True when non-whitespace code precedes the comment on its own line, i.e. the
// comment trails code rather than sitting on its own line.
function trailsCode(text, start) {
  const lineStart = text.lastIndexOf("\n", start - 1) + 1;
  return text.slice(lineStart, start).trim().length > 0;
}

// A comment marker preceded by code on the same line. The Rust pattern skips a
// URL's `://`; the Lean pattern skips lines that open with a comment.
const TRAILING_LINE_COMMENT = {
  ".rs": /^\s*[^/*\s].*[^:/](\/\/.*)$/,
  ".lean": /^\s*(?!--|\/-)\S.*?\s(--.*)$/,
};

function scanLineRegex(text, pattern) {
  const comments = [];
  let offset = 0;
  for (const line of text.split("\n")) {
    const match = pattern.exec(line);
    if (match) {
      const commentStart = line.length - match[1].length;
      comments.push({ text: match[1], start: offset + commentStart });
    }
    offset += line.length + 1;
  }
  return comments;
}

function checkSource(text, path, findings) {
  const pattern = Object.entries(TRAILING_LINE_COMMENT).find(([extension]) =>
    path.endsWith(extension),
  )?.[1];
  const comments = pattern
    ? scanLineRegex(text, pattern)
    : scanLineComments(text).filter((comment) => trailsCode(text, comment.start));
  for (const comment of comments) {
    findings.push({
      path,
      line: lineIndexAt(text, comment.start) + 1,
      snippet: comment.text.trim().slice(0, 72),
    });
  }
}

const gate = {
  extensions: CHECKABLE_EXTENSIONS,
  checkSource,
  report: {
    header: (count) =>
      `Trailing inline comments found (${count}). Move each onto its own line above the code:\n`,
    line: (finding) => `  ${relative(REPO_ROOT, finding.path)}:${finding.line}  ${finding.snippet}`,
    footer: "\nA comment on its own line above the code reads before the code it explains.",
  },
};

const invokedDirectly =
  Boolean(process.argv[1]) && import.meta.url === pathToFileURL(process.argv[1]).href;
if (invokedDirectly) {
  runGate(process.argv.slice(2), gate);
}

export { checkSource };

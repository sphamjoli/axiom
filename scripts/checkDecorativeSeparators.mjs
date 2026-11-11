// Fails when a comment line is a decorative separator, divider, or banner: a
// visual rule made from repeated separator characters rather than a helpful
// sentence. It flags `// ======`, a `/* ---- */` block, a `* ----` doc-comment
// continuation, `-- ----` and `/- ---- -/` in Lean, and `# ----` in shell, so
// comments carry information and not ornament.
//
// Only comment content is inspected. JavaScript comments come from the
// TypeScript scanner, so a `-----` inside a string or regex literal is never
// seen. Rust, Lean, and shell comments are matched with regexes, which accept a
// false positive on a comment marker inside a string literal.
//
// Detection is deliberately conservative: a line is flagged only when it holds
// a run of at least four identical separator characters and those separators
// dominate the visible content, so ordinary prose, a stray hyphen, or a URL
// passes untouched. Runs under bun or node. Pass file paths to check a subset
// (the pre-commit hook passes staged files); pass none to walk the whole
// workspace.

import { pathToFileURL } from "node:url";
import ts from "typescript";
import { relative, REPO_ROOT, runGate } from "./lib/gateShared.mjs";

const CHECKABLE_EXTENSIONS = [".rs", ".lean", ".mjs", ".sh"];

// Characters used to draw a horizontal rule: the ASCII punctuation runs plus
// the box-drawing line glyphs a divider is sometimes made of. A plain hyphen or
// slash in prose is only a concern once it repeats into a run, which the run
// threshold below enforces.
const SEPARATOR_CHARACTERS = "=-*#~_/─━═╍┅┄┈╌⎯";

// The shortest run of one separator character that reads as a drawn rule rather
// than punctuation. Three or fewer (`---`, `***`) stays within ordinary prose.
const MINIMUM_RUN_LENGTH = 4;

// The share of visible characters that must be separators for a line to count
// as decorative. Keeping it at half means a banner framing a short label is
// caught while a real sentence that merely contains a run is not.
const SEPARATOR_DOMINANCE = 0.5;

// Characters that fence a run off as quoted content rather than a drawn rule: a
// table cell (`|----|`), a regex or path shown in prose (`/====/`), or a code
// literal in backticks or quotes. A genuine divider is never wrapped this way,
// so a run bounded by these on both sides is content the reader is meant to see.
const RUN_DELIMITERS = "`'\"/|";

// A line of comment content is decorative when a single separator character
// repeats into a run of at least the threshold and separators make up at least
// half of what is visible. Spaces break a run, so spaced punctuation such as
// `- - - -` reads as prose and is left alone, and a run fenced by RUN_DELIMITERS
// (a table cell or a regex or literal quoted in prose) is treated as content.
function isDecorativeSeparator(content) {
  const trimmed = content.trim();
  if (trimmed.length === 0) return false;

  const characters = [...trimmed];
  let longestRun = 0;
  let longestRunEndIndex = -1;
  let currentRun = 0;
  let previousCharacter = "";
  let separatorCount = 0;
  let visibleCount = 0;

  characters.forEach((character, index) => {
    const isWhitespace = character === " " || character === "\t";
    if (!isWhitespace) visibleCount += 1;

    if (SEPARATOR_CHARACTERS.includes(character)) {
      separatorCount += 1;
      currentRun = character === previousCharacter ? currentRun + 1 : 1;
      if (currentRun > longestRun) {
        longestRun = currentRun;
        longestRunEndIndex = index;
      }
      previousCharacter = character;
    } else {
      currentRun = 0;
      previousCharacter = "";
    }
  });

  if (longestRun < MINIMUM_RUN_LENGTH) return false;
  if (visibleCount === 0) return false;
  if (separatorCount / visibleCount < SEPARATOR_DOMINANCE) return false;

  const beforeRun = characters[longestRunEndIndex - longestRun];
  const afterRun = characters[longestRunEndIndex + 1];
  const isFenced =
    beforeRun !== undefined &&
    afterRun !== undefined &&
    RUN_DELIMITERS.includes(beforeRun) &&
    RUN_DELIMITERS.includes(afterRun);
  return !isFenced;
}

// Reduce a raw comment token to the content lines a reader sees, with the
// comment markers removed. Each physical line of a block comment drops one
// leading `*` continuation marker so a `* ----` line is judged on `----`.
function commentContentLines(rawText) {
  if (rawText.startsWith("#")) return [rawText.slice(1)];
  if (rawText.startsWith("--")) return [rawText.slice(2)];
  if (rawText.startsWith("//")) return [rawText.slice(2)];
  if (rawText.startsWith("/-")) {
    return rawText.replace(/^\/-[-!]?/, "").replace(/-\/$/, "").split("\n");
  }
  const inner = rawText.replace(/^\/\*/, "").replace(/\*\/$/, "");
  return inner.split("\n").map((physicalLine) => physicalLine.replace(/^\s*\*/, ""));
}

// Zero-based line index of a character offset within some text.
function lineIndexAt(text, offset) {
  return text.slice(0, offset).split("\n").length - 1;
}

// Comment tokens from a JavaScript module via the TypeScript scanner, which
// tracks regex and template literals, so a `//` inside one is never a comment.
// Kept as the reason for the typescript dev dependency.
function scanScriptComments(text) {
  const comments = [];
  const scanner = ts.createScanner(ts.ScriptTarget.Latest, false, ts.LanguageVariant.Standard, text);
  let token = scanner.scan();
  while (token !== ts.SyntaxKind.EndOfFileToken) {
    if (
      token === ts.SyntaxKind.SingleLineCommentTrivia ||
      token === ts.SyntaxKind.MultiLineCommentTrivia
    ) {
      comments.push({ text: scanner.getTokenText(), start: scanner.getTokenPos() });
    }
    token = scanner.scan();
  }
  return comments;
}

const REGEX_COMMENT_PATTERNS = {
  ".rs": [/\/\/.*$/gm, /\/\*[\s\S]*?\*\//g],
  ".lean": [/--.*$/gm, /\/-[\s\S]*?-\//g],
  ".sh": [/^\s*#(?!!).*$/gm],
};

function scanRegexComments(text, patterns) {
  const comments = [];
  for (const pattern of patterns) {
    for (const match of text.matchAll(pattern)) {
      const raw = match[0].trimStart();
      comments.push({ text: raw, start: match.index + match[0].length - raw.length });
    }
  }
  return comments;
}

function extensionOf(path) {
  return CHECKABLE_EXTENSIONS.find((extension) => path.endsWith(extension));
}

function checkSource(text, path, findings) {
  const extension = extensionOf(path);
  const comments =
    extension === ".mjs"
      ? scanScriptComments(text)
      : scanRegexComments(text, REGEX_COMMENT_PATTERNS[extension]);
  for (const comment of comments) {
    const baseLineIndex = lineIndexAt(text, comment.start);
    commentContentLines(comment.text).forEach((content, lineWithinComment) => {
      if (!isDecorativeSeparator(content)) return;
      findings.push({
        path,
        line: baseLineIndex + lineWithinComment + 1,
        snippet: content.trim().slice(0, 72),
      });
    });
  }
}

const gate = {
  extensions: CHECKABLE_EXTENSIONS,
  checkSource,
  report: {
    header: (count) =>
      `Decorative separator comments found (${count}). Delete each; a comment must inform:\n`,
    line: (finding) => `  ${relative(REPO_ROOT, finding.path)}:${finding.line}  ${finding.snippet}`,
    footer: "\nDividers and banners add noise, not information; let the code structure speak.",
  },
};

const invokedDirectly =
  Boolean(process.argv[1]) && import.meta.url === pathToFileURL(process.argv[1]).href;
if (invokedDirectly) {
  runGate(process.argv.slice(2), gate);
}

export { checkSource, isDecorativeSeparator };

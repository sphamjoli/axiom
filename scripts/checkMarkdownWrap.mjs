// Fails when a Markdown paragraph is hard-wrapped across multiple source
// lines instead of written as one line per paragraph. This repository's own
// convention (used throughout `README.md`, `docs/decision-log.md`, and every
// crate README) lets the renderer or editor wrap prose; a paragraph broken by
// real newlines in the source renders as a narrow column in any viewer that
// does not collapse single newlines into a space, and looks inconsistent
// next to every other document in the repository regardless.
//
// Only plain prose paragraphs are inspected. Fenced code blocks, indented
// code blocks, headings, list items (including a wrapped list item's own
// continuation lines), table rows, and blank lines are exempt: a list item
// or a code block legitimately spans several source lines. Runs under bun or
// node. Pass file paths to check a subset (the pre-commit hook passes staged
// files); pass none to walk the whole workspace.

import { pathToFileURL } from "node:url";
import { REPO_ROOT, relative, runGate } from "./lib/gateShared.mjs";

const CHECKABLE_EXTENSIONS = [".md"];
const LIST_MARKER = /^(-|\*|\+|\d+\.)\s/;

function checkSource(text, path, findings) {
  const lines = text.split("\n");
  let inFence = false;
  let paragraphStartLine = null;
  let paragraphLineCount = 0;

  const closeParagraph = () => {
    if (paragraphLineCount > 1) {
      findings.push({
        path,
        line: paragraphStartLine,
        snippet: lines[paragraphStartLine - 1].trim().slice(0, 72),
      });
    }
    paragraphStartLine = null;
    paragraphLineCount = 0;
  };

  lines.forEach((line, index) => {
    const trimmed = line.trim();
    if (trimmed.startsWith("```")) {
      closeParagraph();
      inFence = !inFence;
      return;
    }
    if (inFence) return;

    const isCodeIndent = line.startsWith("    ") || line.startsWith("\t");
    const isHeading = trimmed.startsWith("#");
    const isTableRow = trimmed.startsWith("|");
    const isListItem = LIST_MARKER.test(trimmed);

    if (trimmed === "" || isCodeIndent || isHeading || isTableRow || isListItem) {
      closeParagraph();
      return;
    }

    if (paragraphStartLine === null) paragraphStartLine = index + 1;
    paragraphLineCount += 1;
  });

  closeParagraph();
}

const gate = {
  extensions: CHECKABLE_EXTENSIONS,
  checkSource,
  report: {
    header: (count) =>
      `Hard-wrapped Markdown paragraphs found (${count}). Join each onto one line:\n`,
    line: (finding) => `  ${relative(REPO_ROOT, finding.path)}:${finding.line}  ${finding.snippet}`,
    footer:
      "\nOne line per paragraph, wrapped by the renderer, matches the rest of this repository's documents.",
  },
};

const invokedDirectly =
  Boolean(process.argv[1]) && import.meta.url === pathToFileURL(process.argv[1]).href;
if (invokedDirectly) {
  runGate(process.argv.slice(2), gate);
}

export { checkSource };

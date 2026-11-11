// Fails a message whose header does not follow this repository's Conventional
// Commits convention. The single source of truth for both the commit-msg hook
// and the pull-request title check in `.github/workflows/pr-checks.yml`. Runs
// under bun or node. Reads the message from the file path given as the one
// argument.

import { readFileSync } from "node:fs";

const TYPES = [
  "feat",
  "fix",
  "docs",
  "style",
  "refactor",
  "test",
  "chore",
  "ci",
  "perf",
  "revert",
];
const SCOPES = ["core", "infer", "circuit", "cert", "cli", "lean", "examples", "docs", "ci", "deps"];
const SUBJECT_PATTERN = /^[a-zA-Z][-a-zA-Z0-9 .,():+/]+$/;

const HEADER_PATTERN = new RegExp(
  `^(?<type>${TYPES.join("|")})(\\((?<scope>[^)]+)\\))?: (?<subject>.+)$`,
);

function firstNonCommentLine(message) {
  for (const line of message.split("\n")) {
    if (line.trim() === "" || line.startsWith("#")) continue;
    return line;
  }
  return "";
}

function validate(message) {
  const header = firstNonCommentLine(message);
  const match = HEADER_PATTERN.exec(header);
  if (!match || !match.groups) {
    return `Commit message must read "type(scope): subject" or "type: subject".\nTypes: ${TYPES.join(", ")}\nScopes: ${SCOPES.join(", ")} (optional)\nGot: "${header}"`;
  }

  const { scope, subject } = match.groups;
  if (scope !== undefined && !SCOPES.includes(scope)) {
    return `Unknown scope "${scope}". Scopes: ${SCOPES.join(", ")}\nGot: "${header}"`;
  }
  if (!SUBJECT_PATTERN.test(subject)) {
    return `Subject must start with a letter and contain only letters, digits, spaces, and -.,():+/\nGot: "${subject}"`;
  }

  return null;
}

function main() {
  const path = process.argv[2];
  if (!path) {
    console.error("checkCommitMsg: no commit-message file path given");
    process.exit(1);
  }
  const message = readFileSync(path, "utf8");
  const error = validate(message);
  if (error) {
    console.error(`checkCommitMsg: ${error}`);
    console.error(
      '\nExample: "fix(cert): reject certificates whose weights do not sum to one"',
    );
    process.exit(1);
  }
}

main();

export { validate };

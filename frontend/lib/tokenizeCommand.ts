export type CommandTokenKind =
  | "runner"
  | "package"
  | "url"
  | "flag"
  | "operator"
  | "arg";

export interface CommandToken {
  text: string;
  kind: CommandTokenKind;
}

const RUNNERS = new Set([
  "npx",
  "npm",
  "pnpm",
  "yarn",
  "pip",
  "pip3",
  "cargo",
  "curl",
  "wget",
  "bun",
  "go",
]);

const OPERATORS = new Set(["|", "&&", ";", "||", ">", "<", ">>"]);

export function tokenKind(
  word: string,
  prevRunner: boolean,
): CommandTokenKind {
  const lower = word.toLowerCase();
  if (RUNNERS.has(lower)) return "runner";
  if (word.startsWith("-")) return "flag";
  if (OPERATORS.has(word)) return "operator";
  if (
    word.startsWith("http://") ||
    word.startsWith("https://") ||
    (word.includes(".") &&
      !word.startsWith("./") &&
      !word.startsWith("../"))
  ) {
    return "url";
  }
  if (
    prevRunner ||
    word.startsWith("@") ||
    word.includes("/") ||
    word.includes(":")
  ) {
    return "package";
  }
  return "arg";
}

export function tokenClass(kind: CommandTokenKind): string {
  switch (kind) {
    case "runner":
      return "cmd-token-runner";
    case "package":
      return "cmd-token-package";
    case "url":
      return "cmd-token-url";
    case "flag":
      return "cmd-token-flag";
    case "operator":
      return "cmd-token-op";
    case "arg":
      return "cmd-token-arg";
  }
}

export function tokenizeCommand(command: string): CommandToken[] {
  const tokens: CommandToken[] = [];
  let prevRunner = false;
  for (const word of command.split(/\s+/).filter(Boolean)) {
    const kind = tokenKind(word, prevRunner);
    prevRunner = kind === "runner";
    tokens.push({ text: word, kind });
  }
  return tokens;
}
import { parseArgs as parseStdArgs } from "@std/cli/parse-args";
import type { Args, ParseOptions } from "@std/cli/parse-args";

import { io } from "@/lib/std/io.ts";

type CliParseOptions = Omit<ParseOptions, "unknown" | "--"> & {
  unknownOptionMessage?: (arg: string) => string;
};

export function parseArgs(args: string[], options: CliParseOptions = {}): Args {
  const { unknownOptionMessage, ...parseOptions } = options;
  requireStringValues(args, Array.isArray(parseOptions.string) ? parseOptions.string : []);
  return parseStdArgs(args, {
    "--": true,
    ...parseOptions,
    unknown: (arg) => {
      if (arg.startsWith("-")) {
        io.fail(unknownOptionMessage?.(arg) ?? `unknown option: ${arg}`);
      }
      return true;
    },
  });
}

function requireStringValues(args: string[], names: string[]): void {
  const stringOptions = new Set(names);
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--") {
      return;
    }
    if (!arg.startsWith("--")) {
      continue;
    }
    const [name, value] = arg.slice(2).split("=", 2);
    if (!stringOptions.has(name) || value !== undefined) {
      continue;
    }
    const next = args[index + 1];
    if (next === undefined || next.startsWith("-")) {
      io.fail(`missing value for --${name}`);
    }
  }
}

export function helpRequested(args: Args): boolean {
  return args.help === true || args.h === true || args._.includes("help");
}

export function requireNoPositionals(
  args: Args,
  context: string,
  options: { allowHelp?: boolean } = {},
): void {
  const extra = args._.find((arg) => !(options.allowHelp === true && arg === "help"));
  if (extra !== undefined) {
    io.fail(`${context}: unexpected argument: ${extra}`);
  }
}

export function stringOption(args: Args, name: string, fallback = ""): string {
  const value = args[name];
  return typeof value === "string" ? value : fallback;
}

export function booleanOption(args: Args, name: string): boolean {
  return args[name] === true;
}

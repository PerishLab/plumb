const decoder = new TextDecoder();
const encoder = new TextEncoder();

export type CommandOptions = {
  cwd?: string;
  env?: Record<string, string>;
  stdin?: "inherit" | "null" | "piped";
  stdout?: "inherit" | "null" | "piped";
  stderr?: "inherit" | "null" | "piped";
};

const blockedInheritedEnv = new Set([
  "DYLD_FALLBACK_LIBRARY_PATH",
  "DYLD_INSERT_LIBRARIES",
  "DYLD_LIBRARY_PATH",
  "LD_PRELOAD",
  "LD_LIBRARY_PATH",
]);

function hasBlockedInheritedEnv(): boolean {
  for (const key of blockedInheritedEnv) {
    if (Deno.env.get(key) !== undefined) {
      return true;
    }
  }
  return false;
}

function sanitizedEnv(extra: Record<string, string> | undefined): Record<string, string> {
  const env = Deno.env.toObject();
  for (const key of blockedInheritedEnv) {
    delete env[key];
  }
  return { ...env, ...(extra ?? {}) };
}

function envOptions(
  extra: Record<string, string> | undefined,
): Pick<Deno.CommandOptions, "clearEnv" | "env"> {
  if (hasBlockedInheritedEnv()) {
    return { clearEnv: true, env: sanitizedEnv(extra) };
  }
  return extra === undefined ? {} : { env: extra };
}

async function run(command: string, args: string[] = [], options: CommandOptions = {}) {
  const code = await status(command, args, options);
  if (code !== 0) {
    Deno.exit(code);
  }
}

async function status(command: string, args: string[] = [], options: CommandOptions = {}) {
  const status = await new Deno.Command(command, {
    args,
    cwd: options.cwd,
    ...envOptions(options.env),
    stdin: options.stdin ?? "inherit",
    stdout: options.stdout ?? "inherit",
    stderr: options.stderr ?? "inherit",
  }).spawn().status;
  return status.code;
}

async function text(
  command: string,
  args: string[] = [],
  options: Omit<CommandOptions, "stdout"> = {},
): Promise<string> {
  const output = await new Deno.Command(command, {
    args,
    cwd: options.cwd,
    ...envOptions(options.env),
    stdin: options.stdin ?? "null",
    stdout: "piped",
    stderr: options.stderr ?? "inherit",
  }).output();
  if (!output.success) {
    Deno.exit(output.code);
  }
  return decoder.decode(output.stdout).trimEnd();
}

async function input(
  command: string,
  args: string[],
  input: string,
  options: Omit<CommandOptions, "stdin"> = {},
): Promise<string> {
  const child = new Deno.Command(command, {
    args,
    cwd: options.cwd,
    ...envOptions(options.env),
    stdin: "piped",
    stdout: options.stdout ?? "piped",
    stderr: options.stderr ?? "inherit",
  }).spawn();
  const writer = child.stdin.getWriter();
  await writer.write(encoder.encode(input));
  await writer.close();
  const output = await child.output();
  if (!output.success) {
    Deno.exit(output.code);
  }
  return decoder.decode(output.stdout).trimEnd();
}

async function exists(name: string): Promise<boolean> {
  try {
    await new Deno.Command(name, {
      args: ["--version"],
      ...envOptions(undefined),
      stdin: "null",
      stdout: "null",
      stderr: "null",
    }).output();
    return true;
  } catch (err) {
    if (err instanceof Deno.errors.NotFound) {
      return false;
    }
    throw err;
  }
}

export const cmd = {
  run,
  status,
  text,
  input,
  exists,
};

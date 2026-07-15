import { path as stdPath } from "@/lib/std/path.ts";

async function fileExists(path: string): Promise<boolean> {
  try {
    const stat = await Deno.stat(path);
    return stat.isFile;
  } catch (err) {
    if (err instanceof Deno.errors.NotFound) {
      return false;
    }
    throw err;
  }
}

async function dirExists(path: string): Promise<boolean> {
  try {
    const stat = await Deno.stat(path);
    return stat.isDirectory;
  } catch (err) {
    if (err instanceof Deno.errors.NotFound) {
      return false;
    }
    throw err;
  }
}

async function dirEnsure(path: string, mode?: string): Promise<void> {
  await Deno.mkdir(path, { recursive: true });
  await chmodIfUnix(path, mode);
}

async function chmodIfUnix(path: string, mode?: string): Promise<void> {
  if (mode === undefined || Deno.build.os === "windows") {
    return;
  }
  const parsed = Number.parseInt(mode.replace(/^0o/, ""), 8);
  if (!Number.isInteger(parsed) || parsed < 0) {
    throw new Error(`invalid file mode: ${mode}`);
  }
  await Deno.chmod(path, parsed);
}

async function writeText(path: string, text: string, mode?: string): Promise<void> {
  const parent = stdPath.dirname(path);
  if (parent !== "") {
    await Deno.mkdir(parent, { recursive: true });
  }
  await Deno.writeTextFile(path, text);
  await chmodIfUnix(path, mode);
}

async function containsAny(path: string, needles: string[]): Promise<boolean> {
  const text = await readTextIfExists(path);
  return needles.some((needle) => text.includes(needle));
}

async function backupNumbered(path: string): Promise<string> {
  const backup = await nextBackupPath(path);
  await Deno.rename(path, backup);
  return backup;
}

async function readTextIfExists(path: string): Promise<string> {
  try {
    return await Deno.readTextFile(path);
  } catch (err) {
    if (err instanceof Deno.errors.NotFound) {
      return "";
    }
    throw err;
  }
}

async function nextBackupPath(path: string): Promise<string> {
  const { dir, name } = splitPath(path);
  const first = pathWithFileName(dir, `${name}.bak`);
  if (!(await pathExists(first))) {
    return first;
  }
  for (let index = 1; index < 1000; index += 1) {
    const candidate = pathWithFileName(dir, `${name}.bak.${index}`);
    if (!(await pathExists(candidate))) {
      return candidate;
    }
  }
  throw new Error(`too many existing backups for ${path}`);
}

function splitPath(path: string): { dir: string; name: string } {
  const trimmed = path.replace(/[\\/]+$/g, "");
  const slash = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  const dir = slash < 0 ? "" : trimmed.slice(0, slash);
  const name = slash < 0 ? trimmed : trimmed.slice(slash + 1);
  if (name === "") {
    throw new Error(`invalid path: ${path}`);
  }
  return { dir, name };
}

function pathWithFileName(dir: string, name: string): string {
  return dir === "" ? name : stdPath.join(dir, name);
}

async function pathExists(path: string): Promise<boolean> {
  try {
    await Deno.stat(path);
    return true;
  } catch (err) {
    if (err instanceof Deno.errors.NotFound) {
      return false;
    }
    throw err;
  }
}

export const fs = {
  file: {
    exists: fileExists,
    writeText,
    readTextIfExists,
    containsAny,
    chmodIfUnix,
    backup: {
      numbered: backupNumbered,
    },
  },
  dir: {
    exists: dirExists,
    ensure: dirEnsure,
  },
};

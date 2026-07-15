const encoder = new TextEncoder();

type TreeEntry = {
  label: string;
  file: string;
};

export async function treeHash(paths: string[]): Promise<string> {
  if (paths.length === 0) {
    throw new Error("treeHash requires at least one path");
  }
  const entries: TreeEntry[] = [];
  for (const path of paths) {
    await collectTree(path, path, entries);
  }
  entries.sort((left, right) => left.label.localeCompare(right.label));

  const zero = new Uint8Array([0]);
  const parts: Uint8Array[] = [];
  for (const entry of entries) {
    parts.push(
      encoder.encode(normalizePath(entry.label)),
      zero,
      await Deno.readFile(entry.file),
      zero,
    );
  }
  const payload = concatBytes(parts);
  const digestInput = new ArrayBuffer(payload.byteLength);
  new Uint8Array(digestInput).set(payload);
  const digest = await crypto.subtle.digest("SHA-256", digestInput);
  return Array.from(new Uint8Array(digest))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

async function collectTree(path: string, label: string, entries: TreeEntry[]): Promise<void> {
  const stat = await Deno.stat(path);
  if (stat.isFile) {
    entries.push({ label, file: path });
    return;
  }
  if (stat.isDirectory) {
    for await (const entry of Deno.readDir(path)) {
      await collectTree(pathJoin(path, entry.name), pathJoin(label, entry.name), entries);
    }
    return;
  }
  throw new Error(`unsupported path for treeHash: ${path}`);
}

function pathJoin(...parts: string[]): string {
  const separator = Deno.build.os === "windows" ? "\\" : "/";
  const joined = parts
    .filter((part) => part !== "")
    .map((part, index) =>
      index === 0 ? part.replace(/[\\/]+$/g, "") : part.replace(/^[\\/]+|[\\/]+$/g, "")
    )
    .filter((part) => part !== "")
    .join(separator);
  return joined === "" ? "." : joined;
}

function normalizePath(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+/g, "/");
}

function concatBytes(parts: Uint8Array[]): Uint8Array {
  const total = parts.reduce((sum, part) => sum + part.length, 0);
  const output = new Uint8Array(total);
  let offset = 0;
  for (const part of parts) {
    output.set(part, offset);
    offset += part.length;
  }
  return output;
}

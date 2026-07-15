export type StableVersion = {
  major: number;
  minor: number;
  patch: number;
};

export function parseStableVersion(version: string): StableVersion {
  const value = version.startsWith("v") ? version.slice(1) : version;
  const parts = value.split(".");
  if (parts.length !== 3) {
    throw new Error(`expected stable semantic version, got ${version}`);
  }
  const [major, minor, patch] = parts.map((part) => {
    if (!/^[0-9]+$/.test(part)) {
      throw new Error(`invalid stable semantic version, got ${version}`);
    }
    return Number(part);
  });
  return { major, minor, patch };
}

export function compareStableVersion(left: string, right: string): "lt" | "eq" | "gt" {
  const leftParsed = parseStableVersion(left);
  const rightParsed = parseStableVersion(right);
  for (const key of ["major", "minor", "patch"] as const) {
    if (leftParsed[key] < rightParsed[key]) {
      return "lt";
    }
    if (leftParsed[key] > rightParsed[key]) {
      return "gt";
    }
  }
  return "eq";
}

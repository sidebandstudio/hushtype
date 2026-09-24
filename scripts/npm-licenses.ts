// Prints the licenses of every package in the production dependency tree of
// package.json, grouped by identical license text. Used by
// scripts/generate-third-party-notices.sh.
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";

interface Manifest {
  name: string;
  version: string;
  license?: string | { type: string };
  licenses?: { type: string }[];
  author?: string | { name: string };
  dependencies?: Record<string, string>;
  optionalDependencies?: Record<string, string>;
}

const root = process.cwd();
const rootManifest = JSON.parse(
  readFileSync(join(root, "package.json"), "utf8"),
) as Manifest;

// Node module resolution: nearest node_modules/<name> walking up from `from`.
const resolvePackage = (name: string, from: string): string | null => {
  for (let dir = from; ; dir = dirname(dir)) {
    const candidate = join(dir, "node_modules", name);
    if (existsSync(join(candidate, "package.json"))) return candidate;
    if (dirname(dir) === dir) return null;
  }
};

const licenseId = (m: Manifest): string => {
  if (typeof m.license === "string") return m.license;
  if (m.license?.type) return m.license.type;
  if (m.licenses?.length) return m.licenses.map((l) => l.type).join(" OR ");
  return "UNKNOWN";
};

const authorName = (m: Manifest): string =>
  typeof m.author === "string" ? m.author : (m.author?.name ?? "its authors");

const licenseText = (dir: string, m: Manifest): string => {
  const file = readdirSync(dir).find((f) =>
    /^(licen[cs]e|copying)(\.|-|$)/i.test(f),
  );
  if (file) return readFileSync(join(dir, file), "utf8").trim();
  return `${licenseId(m)} license. Copyright (c) ${authorName(m)}. The package ships no license file.`;
};

const groups = new Map<string, Set<string>>();
const seen = new Set<string>();
const queue: [string, string][] = Object.keys(
  rootManifest.dependencies ?? {},
).map((name) => [name, root]);

while (queue.length > 0) {
  const [name, from] = queue.shift()!;
  const dir = resolvePackage(name, from);
  if (!dir) continue; // optional dependency for another platform
  const manifest = JSON.parse(
    readFileSync(join(dir, "package.json"), "utf8"),
  ) as Manifest;
  const key = `${manifest.name}@${manifest.version}`;
  if (seen.has(key)) continue;
  seen.add(key);

  const text = licenseText(dir, manifest);
  if (!groups.has(text)) groups.set(text, new Set());
  groups.get(text)!.add(`${key} (${licenseId(manifest)})`);

  for (const dep of Object.keys({
    ...manifest.dependencies,
    ...manifest.optionalDependencies,
  })) {
    queue.push([dep, dir]);
  }
}

const sorted = [...groups.entries()].sort(([, a], [, b]) =>
  [...a].sort()[0].localeCompare([...b].sort()[0]),
);
for (const [text, packages] of sorted) {
  console.log("-".repeat(80));
  console.log("Used by:");
  for (const p of [...packages].sort()) console.log(`  - ${p}`);
  console.log();
  console.log(text);
  console.log();
}
console.error(`${seen.size} packages, ${groups.size} distinct license texts`);

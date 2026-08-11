import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

const roots = ["AGENTS.md", "README.md", "assets", "docs", "tests/oracles/README.md"];
const markdownFiles = roots.flatMap(collectMarkdown).toSorted();
const failures = [];

for (const source of markdownFiles) {
  const contents = readFileSync(source, "utf8");
  if (source !== "AGENTS.md" && !contents.includes("## Agent metadata")) {
    failures.push(`${source}: missing Agent metadata`);
  }

  for (const match of contents.matchAll(/(?<!!)\[[^\]]*\]\(([^)]+)\)/g)) {
    const rawTarget = match[1].trim();
    if (!rawTarget || rawTarget.startsWith("#") || /^[a-z][a-z0-9+.-]*:/i.test(rawTarget)) {
      continue;
    }

    const withoutFragment = rawTarget.split("#", 1)[0].replace(/^<|>$/g, "");
    const target = resolve(dirname(source), decodeURIComponent(withoutFragment));
    if (!existsSync(target)) {
      failures.push(`${source}: missing ${rawTarget}`);
    }
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log(`Documentation validation passed (${markdownFiles.length} Markdown files).`);

function collectMarkdown(path) {
  if (!existsSync(path)) {
    return [];
  }
  if (!statSync(path).isDirectory()) {
    return path.endsWith(".md") ? [path] : [];
  }
  return readdirSync(path, { withFileTypes: true }).flatMap((entry) => {
    const child = join(path, entry.name);
    return entry.isDirectory() ? collectMarkdown(child) : child.endsWith(".md") ? [child] : [];
  });
}

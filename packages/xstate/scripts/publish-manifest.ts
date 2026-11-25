import { copyFileSync, existsSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const PACKAGE = resolve(import.meta.dirname, "..");
const MANIFEST = join(PACKAGE, "package.json");
const NEXT_MANIFEST = join(PACKAGE, ".package.next.json");
const SOURCE = join(PACKAGE, "node_modules", ".package.source.json");

const replaceManifest = (text: string): void => {
  writeFileSync(NEXT_MANIFEST, text);
  renameSync(NEXT_MANIFEST, MANIFEST);
};

const restore = (): void => {
  if (existsSync(SOURCE)) {
    replaceManifest(readFileSync(SOURCE, "utf8"));
    rmSync(SOURCE);
  }
};

const apply = (): void => {
  restore();
  copyFileSync(MANIFEST, SOURCE);
  const manifest = JSON.parse(readFileSync(MANIFEST, "utf8")) as {
    publishConfig: { exports: unknown };
  };
  const published = { ...manifest, exports: manifest.publishConfig.exports };
  replaceManifest(`${JSON.stringify(published, null, 2)}\n`);
};

const [command] = process.argv.slice(2);
if (command === "apply") {
  apply();
} else if (command === "restore") {
  restore();
} else {
  process.stderr.write("usage: node scripts/publish-manifest.ts apply|restore\n");
  process.exit(2);
}

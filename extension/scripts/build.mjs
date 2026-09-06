import { build } from "esbuild";
import { mkdir, rm, copyFile, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
// Icons live at the repo root (shared across cli/, extension/, and any
// future component) rather than duplicated per-project.
const repoRoot = path.dirname(root);
const targets = ["chrome", "firefox"];

async function buildTarget(target) {
  const outDir = path.join(root, "dist", target);
  await rm(outDir, { recursive: true, force: true });
  await mkdir(path.join(outDir, "popup"), { recursive: true });
  await mkdir(path.join(outDir, "icons"), { recursive: true });

  await build({
    entryPoints: {
      background: path.join(root, "src/background.ts"),
      "popup/popup": path.join(root, "src/popup/popup.ts"),
    },
    outdir: outDir,
    bundle: true,
    // Bundled, but deliberately not minified: Firefox AMO requires
    // reviewers to reproduce the exact submitted build from source, and
    // minified output is specifically what draws the harshest scrutiny —
    // staying close to readable source avoids that friction.
    minify: false,
    format: "iife",
    target: "es2020",
    logLevel: "info",
  });

  await copyFile(path.join(root, "src/popup/popup.html"), path.join(outDir, "popup/popup.html"));
  await copyFile(path.join(root, "src/popup/popup.css"), path.join(outDir, "popup/popup.css"));

  for (const size of [16, 32, 48, 128]) {
    await copyFile(
      path.join(repoRoot, `icons/icon-${size}.png`),
      path.join(outDir, `icons/icon-${size}.png`),
    );
  }

  const base = JSON.parse(await readFile(path.join(root, "manifest/base.json"), "utf8"));
  const override = JSON.parse(await readFile(path.join(root, `manifest/${target}.json`), "utf8"));
  const manifest = { ...base, ...override };
  await writeFile(path.join(outDir, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);

  console.log(`built dist/${target}`);
}

for (const target of targets) {
  await buildTarget(target);
}

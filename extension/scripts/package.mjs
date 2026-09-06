import AdmZip from "adm-zip";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { existsSync } from "node:fs";

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));

function packageTarget(target, extension) {
  const srcDir = path.join(root, "dist", target);
  if (!existsSync(srcDir)) {
    throw new Error(`dist/${target} does not exist — run \`npm run build\` first`);
  }

  const zip = new AdmZip();
  zip.addLocalFolder(srcDir);
  const outFile = path.join(root, "dist", `sessionforge-${target}.${extension}`);
  zip.writeZip(outFile);
  console.log(`wrote ${path.relative(root, outFile)}`);
}

// Chrome/Edge both accept the same zip; Firefox AMO also accepts a plain
// zip for its own build (the .xpi naming is cosmetic — it's the same
// format), so both use the .zip extension here.
packageTarget("chrome", "zip");
packageTarget("firefox", "zip");

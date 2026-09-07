import fs from "node:fs";
import path from "node:path";

import { installCatalog } from "./builder-state.js";

const catalogPath = path.join(import.meta.dirname, "segment-catalog.json");

if (!fs.existsSync(catalogPath)) {
  process.stderr.write("segment-catalog.json is missing: run `cargo run -- --schema > site/segment-catalog.json`\n");
  process.exit(1);
}

export const catalog = JSON.parse(fs.readFileSync(catalogPath, "utf8"));
export const modules = installCatalog(catalog);

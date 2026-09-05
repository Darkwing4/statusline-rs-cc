"use strict";

const fs = require("node:fs");
const path = require("node:path");

const app = require("./app.js");

const catalogPath = path.join(__dirname, "segment-catalog.json");
if (!fs.existsSync(catalogPath)) {
  process.stderr.write("segment-catalog.json is missing: run `cargo run -- --schema > site/segment-catalog.json`\n");
  process.exit(1);
}

const modules = app.installCatalog(JSON.parse(fs.readFileSync(catalogPath, "utf8")));
app.state.segments = modules.map((entry) => app.createSegment(entry.id));
process.stdout.write(app.generateRon());

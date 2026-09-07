import { modules } from "./catalog-from-disk.js";
import { createSegment, state } from "./builder-state.js";
import { generateRon } from "./ron-config.js";

state.segments = modules.map((entry) => createSegment(entry.id));
process.stdout.write(generateRon());

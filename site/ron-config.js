import { hexToRgb } from "./color.js";
import { fieldLabel } from "./segment-catalog.js";
import { segmentModule, state } from "./builder-state.js";

export function configProblems() {
  const problems = [];
  state.segments.forEach((segment) => {
    const entry = segmentModule(segment);
    entry.fields
      .filter((field) => field.kind === "pairs")
      .forEach((field) => {
        if (segment.config[field.name].some(([from]) => from === "")) {
          problems.push(`${entry.label}: every ${fieldLabel(field.name).toLowerCase()} row needs a left side.`);
        }
      });
  });
  return problems;
}

export function generateRon() {
  const lines = [
    "(",
    `    separator: ${ronString(state.separator)},`,
    `    separator_color: ${colorToRon(state.separatorColor)},`,
    "    segments: ["
  ];
  state.segments.forEach((segment) => {
    lines.push(...segmentToRon(segment));
  });
  lines.push("    ],", ")");
  return `${lines.join("\n")}\n`;
}

export function segmentToRon(segment) {
  const entry = segmentModule(segment);
  const fields = entry.fields.map((field) => [field.name, valueToRon(field, segment.config[field.name])]);

  return [
    `        ${segment.type}(`,
    ...fields.map(([key, value]) => `            ${key}: ${value},`),
    "        ),"
  ];
}

function valueToRon(field, value) {
  switch (field.kind) {
    case "color":
      return colorToRon(value);
    case "text":
      return ronString(value);
    case "bool":
      return value ? "true" : "false";
    case "integer":
      return String(Math.max(0, Math.round(Number(value) || 0)));
    case "float":
      return ronFloat(value);
    case "enum":
      return value;
    case "list":
      return `[${value.map(ronString).join(", ")}]`;
    case "pairs":
      return `[${value.map(([from, to]) => `(${ronString(from)}, ${ronString(to)})`).join(", ")}]`;
    default:
      throw new Error(`Cannot serialize field kind: ${field.kind}`);
  }
}

function colorToRon(color) {
  if (color.kind === "Named") {
    return `Named(${color.code})`;
  }
  if (color.kind === "Rgb") {
    const [r, g, b] = hexToRgb(color.hex);
    return `Rgb(${r}, ${g}, ${b})`;
  }
  return "Gradient";
}

function ronFloat(value) {
  const number = Number(value);
  return Number.isInteger(number) ? number.toFixed(1) : String(number);
}

function ronString(value) {
  const escaped = String(value)
    .replaceAll("\\", "\\\\")
    .replaceAll("\"", "\\\"")
    .replaceAll("\b", "\\b")
    .replaceAll("\f", "\\f")
    .replaceAll("\n", "\\n")
    .replaceAll("\r", "\\r")
    .replaceAll("\t", "\\t");
  return `"${escaped}"`;
}

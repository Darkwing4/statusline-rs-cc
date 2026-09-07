const windowLabels = { FiveHour: "5h", SevenDay: "7d" };
const windowOverrides = {
  FiveHour: { prefix: "{t}h " },
  Fable: { prefix: "- {t}d ", active_marker: " |" }
};
const acronyms = { Ttl: "TTL", Llm: "LLM" };
const repeatableSegments = new Set(["Spacer", "LlmInsight"]);

export function segmentLabel(name) {
  return name
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .split(" ")
    .map((word, index) => acronyms[word] || (index === 0 ? word : word.toLowerCase()))
    .join(" ");
}

export function fieldLabel(name) {
  const text = name.replaceAll("_", " ");
  return text.charAt(0).toUpperCase() + text.slice(1);
}

export function validateCatalog(catalog) {
  if (!catalog || typeof catalog.version !== "string" || !Array.isArray(catalog.segments)) {
    throw new Error("The segment catalogue has an unexpected shape.");
  }
  if (catalog.segments.length === 0) {
    throw new Error("The segment catalogue is empty.");
  }
  catalog.segments.forEach((segment) => {
    if (typeof segment.name !== "string" || typeof segment.pitch !== "string" || !Array.isArray(segment.fields)) {
      throw new Error("A catalogue entry is missing its name, pitch, or fields.");
    }
    segment.fields.forEach((field) => {
      if (typeof field.name !== "string" || typeof field.kind !== "string") {
        throw new Error(`${segment.name} has a field without a name or a kind.`);
      }
      if (field.kind === "enum" && (!Array.isArray(field.variants) || field.variants.length === 0)) {
        throw new Error(`${segment.name}.${field.name} is an enum without variants.`);
      }
    });
  });
  return catalog;
}

export function buildModules(catalog) {
  return validateCatalog(catalog).segments.flatMap((segment) => {
    const base = { type: segment.name, description: segment.pitch, fields: segment.fields };
    if (segment.name === "RateLimit") {
      const window = segment.fields.find((field) => field.name === "window" && field.kind === "enum");
      if (!window) {
        throw new Error("RateLimit has no window enum to build the shelf from.");
      }
      return window.variants.map((variant) => ({
        ...base,
        id: `RateLimit:${variant}`,
        label: `Rate limit ${windowLabels[variant] || variant}`,
        overrides: { window: variant, prefix: "{t}d ", ...(windowOverrides[variant] || {}) },
        repeatable: false
      }));
    }
    return [{
      ...base,
      id: segment.name,
      label: segmentLabel(segment.name),
      overrides: {},
      repeatable: repeatableSegments.has(segment.name)
    }];
  });
}

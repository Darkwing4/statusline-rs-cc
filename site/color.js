export const ansiColors = [
  [30, "Black", "#303642"],
  [31, "Red", "#d85c68"],
  [32, "Green", "#69c88d"],
  [33, "Yellow", "#d4b967"],
  [34, "Blue", "#668fd4"],
  [35, "Magenta", "#ad7ad0"],
  [36, "Cyan", "#58c4d1"],
  [37, "White", "#c9d1dc"],
  [90, "Bright black", "#66758a"],
  [91, "Bright red", "#ff7c88"],
  [92, "Bright green", "#78d49b"],
  [93, "Bright yellow", "#e4c879"],
  [94, "Bright blue", "#7fa8ef"],
  [95, "Bright magenta", "#c99af2"],
  [96, "Bright cyan", "#63d9e6"],
  [97, "Bright white", "#e8edf5"]
];

export const contextGradientStops = [
  [0, [150, 150, 150]],
  [20, [180, 165, 100]],
  [30, [220, 60, 60]]
];

export const cacheTtlGradientStops = [
  [0, [120, 120, 120]],
  [70, [200, 180, 80]],
  [90, [230, 100, 60]],
  [100, [255, 50, 50]]
];

export const cacheColdGradientStops = [
  [0, [120, 120, 120]],
  [40, [200, 180, 80]],
  [75, [230, 100, 60]],
  [100, [255, 50, 50]]
];

export function namedColor(code) {
  return { kind: "Named", code };
}

export function rgbColor(r, g, b) {
  return { kind: "Rgb", hex: rgbToHex(r, g, b) };
}

export function gradientColor() {
  return { kind: "Gradient" };
}

export function neutralGrey() {
  return rgbColor(120, 125, 140);
}

export function colorSwatch(color) {
  if (color.kind === "Gradient") {
    return "linear-gradient(90deg, #78d49b, #e4c879, #ff7c88)";
  }
  return resolveColor(color, 50);
}

export function resolveContextColor(color, percentage) {
  if (color.kind !== "Gradient") {
    return resolveColor(color, percentage);
  }
  return rgbArrayToHex(gradientRgb(contextGradientStops, percentage, "truncate"));
}

export function resolveCacheTtlColor(color, cacheView) {
  if (color.kind !== "Gradient") {
    return resolveColor(color, cacheView.percentage);
  }
  const stops = cacheView.cold ? cacheColdGradientStops : cacheTtlGradientStops;
  return rgbArrayToHex(gradientRgb(stops, cacheView.percentage, "truncate"));
}

export function resolveColor(color, percentage) {
  if (color.kind === "Rgb") {
    return color.hex;
  }
  if (color.kind === "Named") {
    const entry = ansiColors.find(([code]) => code === color.code);
    return entry ? entry[2] : "#a6b1c2";
  }
  return interpolateColorStops(
    [60, 200, 60],
    [220, 200, 40],
    [220, 60, 60],
    percentage,
    50
  );
}

export function colorToRgb(color, fallback) {
  if (color.kind === "Rgb") {
    return hexToRgb(color.hex);
  }
  return fallback;
}

export function interpolateColorStops(low, middle, high, percentage, midpoint) {
  const stops = [
    [0, low],
    [midpoint, middle],
    [100, high]
  ];
  return rgbArrayToHex(gradientRgb(stops, percentage, "nearest"));
}

export function gradientRgb(stops, percentage, quantization) {
  if (Number.isNaN(percentage) || stops.length === 0) {
    return [0, 0, 0];
  }

  const [firstPosition, firstColor] = stops[0];
  if (percentage <= firstPosition) {
    return [...firstColor];
  }

  for (let index = 0; index < stops.length - 1; index += 1) {
    const [startPosition, startColor] = stops[index];
    const [endPosition, endColor] = stops[index + 1];
    if (percentage > endPosition) {
      continue;
    }
    const span = endPosition - startPosition;
    if (span <= 0) {
      return [...endColor];
    }
    const amount = Math.max(0, Math.min(1, (percentage - startPosition) / span));
    return startColor.map((channel, channelIndex) => {
      const value = channel + (endColor[channelIndex] - channel) * amount;
      const bounded = Math.max(0, Math.min(255, value));
      return quantization === "truncate" ? Math.trunc(bounded) : Math.round(bounded);
    });
  }

  return [...stops[stops.length - 1][1]];
}

export function rgbToHex(r, g, b) {
  return rgbArrayToHex([r, g, b]);
}

export function rgbArrayToHex(channels) {
  return `#${channels
    .map((channel) => Math.max(0, Math.min(255, channel)).toString(16).padStart(2, "0"))
    .join("")}`;
}

export function hexToRgb(hex) {
  const normalized = hex.replace("#", "");
  return [
    Number.parseInt(normalized.slice(0, 2), 16),
    Number.parseInt(normalized.slice(2, 4), 16),
    Number.parseInt(normalized.slice(4, 6), 16)
  ];
}

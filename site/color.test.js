import assert from "node:assert/strict";
import test from "node:test";

import {
  cacheColdGradientStops,
  cacheTtlGradientStops,
  colorToRgb,
  contextGradientStops,
  gradientRgb,
  interpolateColorStops,
  resolveCacheTtlColor,
  resolveContextColor
} from "./color.js";
import { cacheTtlPreview } from "./segment-preview.js";

const gradient = { kind: "Gradient" };
const named = { kind: "Named", code: 32 };

test("context gradient matches the runtime stops and truncation", () => {
  assert.deepEqual(gradientRgb(contextGradientStops, -1, "truncate"), [150, 150, 150]);
  assert.deepEqual(gradientRgb(contextGradientStops, 0, "truncate"), [150, 150, 150]);
  assert.deepEqual(gradientRgb(contextGradientStops, 10, "truncate"), [165, 157, 125]);
  assert.deepEqual(gradientRgb(contextGradientStops, 20, "truncate"), [180, 165, 100]);
  assert.deepEqual(gradientRgb(contextGradientStops, 25, "truncate"), [200, 112, 80]);
  assert.deepEqual(gradientRgb(contextGradientStops, 30, "truncate"), [220, 60, 60]);
  assert.deepEqual(gradientRgb(contextGradientStops, 100, "truncate"), [220, 60, 60]);
  assert.equal(resolveContextColor(gradient, 25), "#c87050");
});

test("cache gradient matches active and cold runtime modes", () => {
  assert.deepEqual(gradientRgb(cacheTtlGradientStops, 80, "truncate"), [215, 140, 70]);
  assert.deepEqual(gradientRgb(cacheTtlGradientStops, 95, "truncate"), [242, 75, 55]);
  assert.deepEqual(gradientRgb(cacheColdGradientStops, 57.5, "truncate"), [215, 140, 70]);
  assert.deepEqual(gradientRgb(cacheColdGradientStops, 87.5, "truncate"), [242, 75, 55]);
  assert.equal(resolveCacheTtlColor(gradient, { cold: false, percentage: 80 }), "#d78c46");
  assert.equal(resolveCacheTtlColor(gradient, { cold: true, percentage: 87.5 }), "#f24b37");
});

test("cache scenario derives text and burned percentage from runtime values", () => {
  const active = cacheTtlPreview({
    cacheTtlSeconds: 300,
    cacheRemainingSeconds: 272,
    context: 42
  });
  const clean = cacheTtlPreview({
    cacheTtlSeconds: 3600,
    cacheRemainingSeconds: 484,
    context: 12
  });
  const cold = cacheTtlPreview({
    cacheTtlSeconds: 300,
    cacheRemainingSeconds: 0,
    context: 88
  });

  assert.deepEqual(active, {
    cold: false,
    percentage: 28 / 300 * 100,
    text: "4m32s"
  });
  assert.deepEqual(clean, {
    cold: false,
    percentage: 3116 / 3600 * 100,
    text: "8m04s"
  });
  assert.deepEqual(cold, {
    cold: true,
    percentage: 88,
    text: "cold"
  });
  assert.equal(resolveCacheTtlColor(gradient, active), "#828072");
  assert.equal(resolveCacheTtlColor(gradient, clean), "#e0713f");
});

test("rate limit gradient uses runtime fallbacks for non RGB colors", () => {
  assert.deepEqual(colorToRgb(named, [60, 200, 60]), [60, 200, 60]);
  assert.equal(
    interpolateColorStops(
      colorToRgb(named, [60, 200, 60]),
      colorToRgb(named, [220, 200, 40]),
      colorToRgb(named, [220, 60, 60]),
      25,
      50
    ),
    "#8cc832"
  );
});

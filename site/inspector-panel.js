import { ansiColors, colorSwatch, colorToRgb, gradientColor, namedColor, neutralGrey, rgbColor } from "./color.js";
import { fieldLabel, segmentLabel } from "./segment-catalog.js";
import { segmentModule, selectedSegment, state } from "./builder-state.js";
import { element, elements } from "./dom-elements.js";

const gradientFields = new Set(["ContextUsage.color", "PromptCacheTtl.color"]);

export function renderInspector(changed) {
  elements.inspectorControls.replaceChildren();
  const segment = selectedSegment();

  if (!segment) {
    elements.inspectorKind.textContent = "Separator";
    elements.inspectorTitle.textContent = "Whole line";
    elements.inspectorDescription.textContent = "What sits between every piece.";
    elements.removeSelectedButton.hidden = true;
    elements.inspectorControls.append(
      createTextControl("Separator", state.separator, (value) => {
        state.separator = value;
        changed(false);
      }, 12),
      createColorControl(
        "Separator color",
        state.separatorColor,
        (value, rerender) => {
          state.separatorColor = value;
          changed(rerender);
        },
        false
      )
    );
    return;
  }

  const entry = segmentModule(segment);
  elements.inspectorKind.textContent = entry.type;
  elements.inspectorTitle.textContent = entry.label;
  elements.inspectorDescription.textContent = entry.description;
  elements.removeSelectedButton.hidden = false;

  entry.fields.forEach((field) => {
    const control = createControl(field, entry.type, segment.config[field.name], (value, rerender) => {
      segment.config[field.name] = value;
      changed(rerender);
    });
    elements.inspectorControls.append(control);
  });
}

function createControl(field, type, value, onChange) {
  const label = fieldLabel(field.name);
  switch (field.kind) {
    case "color":
      return createColorControl(label, value, onChange, gradientFields.has(`${type}.${field.name}`));
    case "bool":
      return createBooleanControl(label, value, onChange);
    case "enum":
      return createSelectControl(label, field.variants, value, onChange);
    case "integer":
      return createNumberControl(label, { min: 0, max: 1000000000, step: 1 }, value, onChange);
    case "float":
      return createNumberControl(label, floatRange(field.name), value, onChange);
    case "list":
      return createListControl(label, value, onChange);
    case "pairs":
      return createPairsControl(label, value, onChange);
    default:
      return createTextControl(label, value, onChange, field.name === "prompt" ? 2000 : 256);
  }
}

function floatRange(fieldName) {
  if (fieldName === "gradient_midpoint_percentage") {
    return { min: 0.1, max: 99.9, step: 0.1 };
  }
  return { min: 0, max: 1000000000, step: 0.1 };
}

function createTextControl(labelText, value, onChange, maxLength) {
  const label = element("label", "control-label");
  label.append(element("span", "", labelText));
  const input = element("input", "control-input");
  input.type = "text";
  input.value = value;
  input.maxLength = maxLength || 128;
  input.autocomplete = "off";
  input.spellcheck = false;
  input.addEventListener("input", () => onChange(input.value, false));
  label.append(input);
  return label;
}

function createNumberControl(labelText, range, value, onChange) {
  const label = element("label", "control-label");
  label.append(element("span", "", labelText));
  const input = element("input", "control-input");
  input.type = "number";
  input.value = String(value);
  input.min = String(range.min);
  input.max = String(range.max);
  input.step = String(range.step);
  input.addEventListener("input", () => {
    if (input.value === "" || !input.validity.valid) {
      return;
    }
    const number = Number(input.value);
    if (Number.isFinite(number)) {
      onChange(number, false);
    }
  });
  input.addEventListener("change", () => {
    let number = Number(input.value);
    if (!Number.isFinite(number)) {
      number = value;
    }
    number = Math.max(range.min, Math.min(range.max, number));
    if (range.step === 1) {
      number = Math.round(number);
    }
    input.value = String(number);
    onChange(number, false);
  });
  label.append(input);
  return label;
}

function createSelectControl(labelText, variants, value, onChange) {
  const label = element("label", "control-label");
  label.append(element("span", "", labelText));
  const select = element("select", "control-input select-control");
  variants.forEach((variant) => {
    const option = element("option", "", segmentLabel(variant));
    option.value = variant;
    select.append(option);
  });
  select.value = value;
  select.addEventListener("change", () => onChange(select.value, false));
  label.append(select);
  return label;
}

function createBooleanControl(labelText, value, onChange) {
  const label = element("label", "toggle-control");
  label.append(element("span", "", labelText));
  const input = element("input");
  input.type = "checkbox";
  input.checked = Boolean(value);
  input.addEventListener("change", () => onChange(input.checked, true));
  label.append(input);
  return label;
}

function createListControl(labelText, value, onChange) {
  const items = value.map((item) => String(item));
  const wrapper = element("div", "list-control");
  wrapper.append(element("span", "list-control-label", labelText));
  const rows = element("div", "list-rows");

  items.forEach((item, index) => {
    const row = element("div", "list-row");
    const input = element("input", "control-input");
    input.type = "text";
    input.value = item;
    input.maxLength = 256;
    input.autocomplete = "off";
    input.spellcheck = false;
    input.setAttribute("aria-label", `${labelText} ${index + 1}`);
    input.addEventListener("input", () => {
      items[index] = input.value;
      onChange(items.slice(), false);
    });
    row.append(input, createRemoveButton(`${labelText} ${index + 1}`, () => {
      onChange(items.filter((_, at) => at !== index), true);
    }));
    rows.append(row);
  });

  wrapper.append(rows, createAddButton(labelText, () => onChange([...items, ""], true)));
  return wrapper;
}

function createPairsControl(labelText, value, onChange) {
  const pairs = value.map(([from, to]) => [String(from), String(to)]);
  const wrapper = element("div", "list-control");
  wrapper.append(element("span", "list-control-label", labelText));
  const rows = element("div", "list-rows");

  pairs.forEach((pair, index) => {
    const row = element("div", "list-row list-row-pair");
    ["from", "to"].forEach((side, position) => {
      const input = element("input", "control-input");
      input.type = "text";
      input.value = pair[position];
      input.maxLength = 256;
      input.autocomplete = "off";
      input.spellcheck = false;
      input.placeholder = side;
      input.setAttribute("aria-label", `${labelText} ${index + 1} ${side}`);
      input.addEventListener("input", () => {
        pairs[index][position] = input.value;
        onChange(pairs.map((entry) => entry.slice()), false);
      });
      row.append(input);
    });
    row.append(createRemoveButton(`${labelText} ${index + 1}`, () => {
      onChange(pairs.filter((_, at) => at !== index), true);
    }));
    rows.append(row);
  });

  wrapper.append(rows, createAddButton(labelText, () => onChange([...pairs, ["", ""]], true)));
  return wrapper;
}

function createRemoveButton(what, onClick) {
  const button = element("button", "list-remove", "×");
  button.type = "button";
  button.setAttribute("aria-label", `Remove ${what}`);
  button.addEventListener("click", onClick);
  return button;
}

function createAddButton(what, onClick) {
  const button = element("button", "list-add", "+ add");
  button.type = "button";
  button.setAttribute("aria-label", `Add ${what}`);
  button.addEventListener("click", onClick);
  return button;
}

function createColorControl(labelText, value, onChange, allowGradient) {
  const wrapper = element("div", "color-control");
  const heading = element("div", "color-control-label");
  heading.append(element("span", "", labelText));
  const swatch = element("span", "color-swatch");
  swatch.setAttribute("aria-hidden", "true");
  swatch.style.setProperty("--swatch", colorSwatch(value));
  heading.append(swatch);

  const editor = element("div", "color-editor");
  const kindSelect = element("select", "color-kind");
  const colorKinds = [
    ["Named", "ANSI"],
    ["Rgb", "RGB"]
  ];
  if (allowGradient) {
    colorKinds.push(["Gradient", "Gradient"]);
  }
  colorKinds.forEach(([kind, name]) => {
    const option = element("option", "", name);
    option.value = kind;
    kindSelect.append(option);
  });
  kindSelect.value = value.kind;
  kindSelect.setAttribute("aria-label", `${labelText} color type`);
  kindSelect.addEventListener("change", () => {
    let next;
    if (kindSelect.value === "Named") {
      next = neutralGrey();
    } else if (kindSelect.value === "Rgb") {
      const [r, g, b] = colorToRgb(value, [183, 165, 255]);
      next = rgbColor(r, g, b);
    } else {
      next = gradientColor();
    }
    onChange(next, true);
  });
  editor.append(kindSelect);

  if (value.kind === "Named") {
    const select = element("select", "ansi-select");
    select.setAttribute("aria-label", `${labelText} ANSI color`);
    ansiColors.forEach(([code, name]) => {
      const option = element("option", "", `${code} · ${name}`);
      option.value = String(code);
      select.append(option);
    });
    select.value = String(value.code);
    select.addEventListener("change", () => {
      const next = namedColor(Number(select.value));
      swatch.style.setProperty("--swatch", colorSwatch(next));
      onChange(next, false);
    });
    editor.append(select);
  } else if (value.kind === "Rgb") {
    const input = element("input", "rgb-input");
    input.type = "color";
    input.value = value.hex;
    input.setAttribute("aria-label", `${labelText} RGB color`);
    input.addEventListener("input", () => {
      const next = { kind: "Rgb", hex: input.value };
      swatch.style.setProperty("--swatch", colorSwatch(next));
      onChange(next, false);
    });
    editor.append(input);
  } else {
    editor.append(element("div", "gradient-readout", "follows the value"));
  }

  wrapper.append(heading, editor);
  return wrapper;
}

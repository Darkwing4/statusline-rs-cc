import { validateCatalog } from "./segment-catalog.js";
import { scenarios } from "./preview-sessions.js";
import { installCatalog, installedCatalogVersion, resetState, state } from "./builder-state.js";
import { announce, element, elements, initializeElements } from "./dom-elements.js";
import { copyInstallCommand, downloadRon, invalidateSession, openInstallSheet, openRonSheet } from "./export-sheets.js";
import { bindEditorEvents, renderAll } from "./line-editor.js";

const catalogUrl = "segment-catalog.json";

function initializeScenarioOptions() {
  elements.scenarioSelect.replaceChildren();
  scenarios.forEach((entry) => {
    const option = element("option", "", entry.label);
    option.value = entry.id;
    elements.scenarioSelect.append(option);
  });
  elements.scenarioSelect.value = state.scenarioId;
}

function bindSheetEvents() {
  elements.resetButton.addEventListener("click", () => {
    resetState();
    elements.scenarioSelect.value = state.scenarioId;
    elements.terminalWidth.value = String(state.terminalWidth);
    invalidateSession();
    renderAll();
    announce("Line reset to defaults.");
  });
  elements.ronButton.addEventListener("click", openRonSheet);
  elements.downloadButton.addEventListener("click", downloadRon);
  elements.installButton.addEventListener("click", openInstallSheet);
  elements.copyCommandButton.addEventListener("click", copyInstallCommand);
}

async function loadCatalog() {
  const response = await fetch(catalogUrl, { cache: "no-cache" });
  if (!response.ok) {
    throw new Error(`${catalogUrl}: HTTP ${response.status}`);
  }
  return validateCatalog(await response.json());
}

function showCatalogError(error) {
  const message = error instanceof Error ? error.message : String(error);
  elements.shelf.replaceChildren(
    element(
      "p",
      "shelf-empty",
      `The segment catalogue could not be loaded (${message}). Generate it with: cargo run -- --schema > site/segment-catalog.json`
    )
  );
  elements.installButton.disabled = true;
  elements.ronButton.disabled = true;
  elements.resetButton.disabled = true;
}

async function initialize() {
  initializeElements();
  initializeScenarioOptions();
  bindSheetEvents();
  bindEditorEvents();
  elements.terminalWidth.value = String(state.terminalWidth);

  let catalog;
  try {
    catalog = await loadCatalog();
  } catch (error) {
    console.error("Could not load the segment catalogue.", error);
    showCatalogError(error);
    return;
  }

  installCatalog(catalog);
  elements.catalogVersion.textContent = `v${installedCatalogVersion()}`;
  resetState();
  renderAll();
}

initialize();

export const elements = {};

export function initializeElements() {
  const ids = [
    "catalogVersion",
    "scenarioSelect",
    "terminalWidth",
    "terminalWidthValue",
    "lineDimensions",
    "lineCanvas",
    "hiddenPieces",
    "shelf",
    "inspectorKind",
    "inspectorTitle",
    "inspectorDescription",
    "removeSelectedButton",
    "inspectorControls",
    "resetButton",
    "ronButton",
    "installButton",
    "ronDialog",
    "ronProblems",
    "ronOutput",
    "downloadButton",
    "installDialog",
    "commandOutput",
    "copyCommandButton",
    "configSizeOutput",
    "sessionError",
    "sessionResult",
    "liveRegion"
  ];
  ids.forEach((id) => {
    elements[id] = document.getElementById(id);
  });
}

export function element(tagName, className = "", text = "") {
  const node = document.createElement(tagName);
  if (className) {
    node.className = className;
  }
  if (text) {
    node.textContent = text;
  }
  return node;
}

export function announce(message) {
  elements.liveRegion.textContent = "";
  requestAnimationFrame(() => {
    elements.liveRegion.textContent = message;
  });
}

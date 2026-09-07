import { resolveColor } from "./color.js";
import { describeLineFill, displayWidth, layoutRows, runtimePreviewColumns } from "./line-layout.js";
import { LINE_SELECTION, activeScenario, isStandalone, segmentModule, shelfModules, state } from "./builder-state.js";
import { previewSegment } from "./segment-preview.js";
import { element, elements } from "./dom-elements.js";

export const drag = { payload: null };

export function renderLine(select) {
  const session = activeScenario();
  const drawn = state.segments.map((segment) => ({
    segment,
    pieces: previewSegment(segment, session, state.separatorColor),
    standalone: isStandalone(segment)
  }));
  const rendered = drawn.filter((entry) => entry.pieces && entry.pieces.length > 0);
  const maxColumns = runtimePreviewColumns(state.terminalWidth);

  renderHiddenPieces(drawn.filter((entry) => !entry.pieces || entry.pieces.length === 0), select);

  elements.lineCanvas.replaceChildren();
  elements.lineCanvas.style.maxWidth = `${maxColumns}ch`;

  if (rendered.length === 0) {
    elements.lineCanvas.append(
      element("span", "line-empty", "Nothing on the line yet — drag a piece up from below.")
    );
  } else {
    const rows = layoutRows(rendered, state.separator, maxColumns);
    let lastRow = null;
    rows.forEach((row) => {
      const rowNode = element("span", "line-row");
      if (row.standalone) {
        rowNode.classList.add("is-standalone");
      }
      row.cells.forEach((cell) => {
        if (cell.separator) {
          rowNode.append(createSeparatorNode(select));
        }
        rowNode.append(createPieceNode(cell.entry.segment, cell.entry.pieces, select));
      });
      elements.lineCanvas.append(rowNode);
      lastRow = rowNode;
    });
    lastRow.append(element("span", "line-cursor"));
  }

  elements.terminalWidthValue.textContent = String(state.terminalWidth);
  elements.lineDimensions.textContent = describeLineFill(rendered, state.separator, maxColumns);
}

function createPieceNode(segment, pieces, select) {
  const entry = segmentModule(segment);
  const node = element("button", "piece");
  node.type = "button";
  node.dataset.segmentId = segment.id;
  node.dataset.moduleId = segment.moduleId;
  node.title = entry.label;
  node.setAttribute("aria-label", `${entry.label} on the line`);
  if (segment.id === state.selectedId) {
    node.classList.add("is-selected");
  }
  if (pieces.every((part) => displayWidth(part.text) === 0)) {
    node.classList.add("is-blank");
  }

  pieces.forEach((part) => {
    const span = element("span", "", part.text);
    span.style.color = part.color;
    node.append(span);
  });

  node.addEventListener("click", () => select(segment.id));
  makeDraggable(node, { kind: "line", id: segment.id }, "move", entry.label);

  return node;
}

function renderHiddenPieces(entries, select) {
  elements.hiddenPieces.replaceChildren();
  if (entries.length === 0) {
    return;
  }

  elements.hiddenPieces.append(element("span", "hidden-pieces-label", "Silent in this session:"));
  entries.forEach(({ segment }) => {
    const entry = segmentModule(segment);
    const node = element("button", "ghost-piece", entry.label);
    node.type = "button";
    node.dataset.segmentId = segment.id;
    node.dataset.moduleId = segment.moduleId;
    node.title = entry.description;
    if (segment.id === state.selectedId) {
      node.classList.add("is-selected");
    }
    node.addEventListener("click", () => select(segment.id));
    makeDraggable(node, { kind: "line", id: segment.id }, "move", entry.label);
    elements.hiddenPieces.append(node);
  });
}

function createSeparatorNode(select) {
  const node = element("button", "sep", state.separator);
  node.type = "button";
  node.title = "Separator";
  node.setAttribute("aria-label", "Separator settings");
  node.style.color = resolveColor(state.separatorColor, 50);
  if (state.selectedId === LINE_SELECTION) {
    node.classList.add("is-selected");
  }
  node.addEventListener("click", () => select(LINE_SELECTION));
  return node;
}

export function renderShelf(add) {
  const available = shelfModules();
  elements.shelf.replaceChildren();

  if (available.length === 0) {
    elements.shelf.append(element("p", "shelf-empty", "Every piece is on the line."));
    return;
  }

  available.forEach((entry) => {
    const node = element("button", "shelf-piece");
    node.type = "button";
    node.dataset.moduleId = entry.id;
    node.title = entry.description;
    node.append(
      element("span", "shelf-piece-name", entry.label),
      element("span", "shelf-piece-pitch", entry.description)
    );
    node.addEventListener("click", () => add(entry.id));
    makeDraggable(node, { kind: "shelf", moduleId: entry.id }, "copy", entry.label);
    elements.shelf.append(node);
  });
}

function makeDraggable(node, payload, effect, label) {
  node.draggable = true;
  node.addEventListener("dragstart", (event) => {
    drag.payload = payload;
    node.classList.add("is-dragging");
    event.dataTransfer.effectAllowed = effect;
    event.dataTransfer.setData("text/plain", label);
  });
  node.addEventListener("dragend", () => {
    drag.payload = null;
    node.classList.remove("is-dragging");
    clearDropIndicators();
  });
}

export function visiblePieceNodes() {
  const draggedId = drag.payload && drag.payload.kind === "line" ? drag.payload.id : null;
  return [...elements.lineCanvas.querySelectorAll(".piece")].filter(
    (node) => node.dataset.segmentId !== draggedId
  );
}

export function clearDropIndicators() {
  elements.lineCanvas.classList.remove("is-dropzone");
  elements.lineCanvas.querySelectorAll(".drop-before, .drop-after").forEach((node) => {
    node.classList.remove("drop-before", "drop-after");
  });
}

export function capturePositions() {
  const positions = new Map();
  if (!elements.lineCanvas) {
    return positions;
  }
  document.querySelectorAll(".piece, .ghost-piece, .shelf-piece").forEach((node) => {
    positions.set(node.dataset.moduleId, node.getBoundingClientRect());
  });
  return positions;
}

export function playFlip(positions) {
  if (positions.size === 0 || motionIsReduced()) {
    return;
  }
  document.querySelectorAll(".piece, .ghost-piece, .shelf-piece").forEach((node) => {
    const before = positions.get(node.dataset.moduleId);
    if (!before) {
      return;
    }
    const after = node.getBoundingClientRect();
    const dx = before.left - after.left;
    const dy = before.top - after.top;
    if (Math.abs(dx) < 1 && Math.abs(dy) < 1) {
      return;
    }
    node.animate(
      [{ transform: `translate(${dx}px, ${dy}px)` }, { transform: "none" }],
      { duration: 260, easing: "cubic-bezier(0.2, 0.85, 0.25, 1)" }
    );
  });
}

function motionIsReduced() {
  return globalThis.matchMedia
    ? globalThis.matchMedia("(prefers-reduced-motion: reduce)").matches
    : false;
}

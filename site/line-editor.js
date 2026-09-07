import { nearestDropSlot } from "./line-layout.js";
import {
  LINE_SELECTION,
  createSegment,
  moduleEntry,
  segmentModule,
  selectedSegment,
  state,
  usedModuleIds
} from "./builder-state.js";
import { announce, elements } from "./dom-elements.js";
import { renderInspector } from "./inspector-panel.js";
import {
  capturePositions,
  clearDropIndicators,
  drag,
  playFlip,
  renderLine,
  renderShelf,
  visiblePieceNodes
} from "./line-view.js";
import { invalidateSession } from "./export-sheets.js";

export function renderAll() {
  const previous = capturePositions();
  renderShelf(addSegment);
  refreshLine();
  refreshInspector();
  playFlip(previous);
}

export function bindEditorEvents() {
  elements.scenarioSelect.addEventListener("change", () => {
    state.scenarioId = elements.scenarioSelect.value;
    refreshLine();
  });
  elements.terminalWidth.addEventListener("input", () => {
    state.terminalWidth = Number(elements.terminalWidth.value);
    refreshLine();
  });
  elements.removeSelectedButton.addEventListener("click", () => {
    const segment = selectedSegment();
    if (segment) {
      removeSegment(segment.id);
    }
  });
  elements.lineCanvas.addEventListener("click", (event) => {
    if (!event.target.closest(".piece, .sep")) {
      selectTarget(LINE_SELECTION);
    }
  });
  document.addEventListener("dragover", handleLineDragOver);
  document.addEventListener("dragleave", (event) => {
    if (event.relatedTarget === null) {
      clearDropIndicators();
    }
  });
  document.addEventListener("drop", handleLineDrop);
  document.addEventListener("keydown", handleSelectionKey);
}

function refreshLine() {
  renderLine(selectTarget);
}

function refreshInspector() {
  renderInspector(configMutated);
}

function configMutated(rerender) {
  invalidateSession();
  refreshLine();
  if (rerender) {
    refreshInspector();
  }
}

function addSegment(moduleId, index = state.segments.length) {
  const entry = moduleEntry(moduleId);
  if (!entry.repeatable && usedModuleIds().has(moduleId)) {
    return;
  }
  const segment = createSegment(moduleId);
  const target = Math.max(0, Math.min(index, state.segments.length));
  state.segments.splice(target, 0, segment);
  state.selectedId = segment.id;
  invalidateSession();
  renderAll();
  announce(`${entry.label} put on the line at position ${target + 1}.`);
  focusPiece(segment.id);
}

function removeSegment(id) {
  const index = state.segments.findIndex((segment) => segment.id === id);
  if (index < 0) {
    return;
  }

  const [removed] = state.segments.splice(index, 1);
  if (state.selectedId === id) {
    state.selectedId = LINE_SELECTION;
  }
  invalidateSession();
  renderAll();
  announce(`${moduleEntry(removed.moduleId).label} taken off the line.`);
}

function moveSegment(id, delta) {
  const from = state.segments.findIndex((segment) => segment.id === id);
  const to = from + delta;
  if (from < 0 || to < 0 || to >= state.segments.length) {
    return;
  }
  const [segment] = state.segments.splice(from, 1);
  state.segments.splice(to, 0, segment);
  invalidateSession();
  refreshLine();
  announce(`${segmentModule(segment).label} moved to position ${to + 1}.`);
  focusPiece(id);
}

function moveSegmentAlongLine(id, direction) {
  const visibleIds = [...elements.lineCanvas.querySelectorAll(".piece")].map(
    (node) => node.dataset.segmentId
  );
  const position = visibleIds.indexOf(id);
  if (position < 0) {
    moveSegment(id, direction);
    return;
  }
  const neighbourIndex = stateIndexOf(visibleIds[position + direction]);
  if (neighbourIndex < 0) {
    return;
  }
  moveSegmentToIndex(id, direction > 0 ? neighbourIndex + 1 : neighbourIndex);
}

function stateIndexOf(id) {
  return state.segments.findIndex((segment) => segment.id === id);
}

function moveSegmentToIndex(id, requestedIndex) {
  const from = state.segments.findIndex((segment) => segment.id === id);
  if (from < 0) {
    return;
  }
  let to = Math.max(0, Math.min(requestedIndex, state.segments.length));
  const [segment] = state.segments.splice(from, 1);
  if (from < to) {
    to -= 1;
  }
  if (from === to) {
    state.segments.splice(to, 0, segment);
    refreshLine();
    return;
  }
  state.segments.splice(to, 0, segment);
  state.selectedId = id;
  invalidateSession();
  refreshLine();
  refreshInspector();
  announce(`${segmentModule(segment).label} moved to position ${to + 1}.`);
  focusPiece(id);
}

function focusPiece(id) {
  requestAnimationFrame(() => {
    const node = elements.lineCanvas.querySelector(`[data-segment-id="${CSS.escape(id)}"]`);
    if (node) {
      node.focus();
    }
  });
}

function selectTarget(target) {
  if (state.selectedId === target) {
    return;
  }
  state.selectedId = target;
  document.querySelectorAll(".piece, .ghost-piece").forEach((node) => {
    node.classList.toggle("is-selected", node.dataset.segmentId === target);
  });
  elements.lineCanvas.querySelectorAll(".sep").forEach((node) => {
    node.classList.toggle("is-selected", target === LINE_SELECTION);
  });
  refreshInspector();
}

function handleSelectionKey(event) {
  if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) {
    return;
  }
  const target = event.target instanceof Element ? event.target : null;
  if (!target || target.closest("input, textarea, select, dialog, [contenteditable]")) {
    return;
  }
  const segment = selectedSegment();
  if (!segment) {
    return;
  }

  if (event.key === "ArrowLeft") {
    event.preventDefault();
    moveSegmentAlongLine(segment.id, -1);
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    moveSegmentAlongLine(segment.id, 1);
  } else if (event.key === "Delete" || event.key === "Backspace") {
    event.preventDefault();
    removeSegment(segment.id);
  }
}

function handleLineDragOver(event) {
  if (!drag.payload) {
    return;
  }
  event.preventDefault();
  event.dataTransfer.dropEffect = drag.payload.kind === "shelf" ? "copy" : "move";
  clearDropIndicators();
  elements.lineCanvas.classList.add("is-dropzone");
  const slot = lineDropSlot(event);
  if (!slot) {
    return;
  }
  slot.node.classList.add(slot.before ? "drop-before" : "drop-after");
}

function lineDropSlot(event) {
  const boxes = visiblePieceNodes().map((node) => ({ node, rect: node.getBoundingClientRect() }));
  return nearestDropSlot(boxes, event.clientX, event.clientY);
}

function lineDropIndex(event) {
  const slot = lineDropSlot(event);
  if (!slot) {
    return state.segments.length;
  }
  const visible = visiblePieceNodes();
  const leftNeighbour = slot.before ? visible[visible.indexOf(slot.node) - 1] : slot.node;
  if (!leftNeighbour) {
    return 0;
  }
  return stateIndexOf(leftNeighbour.dataset.segmentId) + 1;
}

function handleLineDrop(event) {
  if (!drag.payload) {
    return;
  }
  event.preventDefault();
  const payload = drag.payload;
  const index = lineDropIndex(event);
  drag.payload = null;
  clearDropIndicators();

  if (payload.kind === "shelf") {
    addSegment(payload.moduleId, index);
  } else {
    moveSegmentToIndex(payload.id, index);
  }
}

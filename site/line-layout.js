export function runtimePreviewColumns(columns) {
  return Math.max(0, Math.floor(columns) - 4);
}

export function wrapPreviewSegments(segments, separator, maxColumns) {
  if (segments.length === 0) {
    return [];
  }

  const lines = [];
  const separatorWidth = displayWidth(separator);
  let currentLine = [];
  let currentWidth = 0;

  segments.forEach((pieces) => {
    const segmentWidth = pieces.reduce((sum, value) => sum + displayWidth(value.text), 0);
    if (currentLine.length === 0) {
      currentLine.push({ pieces, separator: false });
      currentWidth = segmentWidth;
      return;
    }

    const projectedWidth = currentWidth + separatorWidth + segmentWidth;
    if (projectedWidth > maxColumns) {
      lines.push(currentLine);
      currentLine = [{ pieces, separator: false }];
      currentWidth = segmentWidth;
      return;
    }

    currentLine.push({ pieces, separator: true });
    currentWidth = projectedWidth;
  });

  if (currentLine.length > 0) {
    lines.push(currentLine);
  }
  return lines;
}

export function layoutRows(entries, separator, maxColumns) {
  if (entries.length === 0) {
    return [];
  }
  const rows = [];
  let block = [];

  const flushBlock = () => {
    const lines = wrapPreviewSegments(block.map((entry) => entry.pieces), separator, maxColumns);
    let cursor = 0;
    lines.forEach((line) => {
      rows.push({
        standalone: false,
        cells: line.map((cell) => {
          const entry = block[cursor];
          cursor += 1;
          return { entry, separator: cell.separator };
        })
      });
    });
    if (lines.length === 0) {
      rows.push({ standalone: false, cells: [] });
    }
    block = [];
  };

  entries.forEach((entry) => {
    if (!entry.standalone) {
      block.push(entry);
      return;
    }
    if (rows.length === 0 || block.length > 0) {
      flushBlock();
    }
    rows.push({ standalone: true, cells: [{ entry, separator: false }] });
  });

  if (rows.length === 0 || block.length > 0) {
    flushBlock();
  }

  return rows;
}

export function describeLineFill(entries, separator, maxColumns) {
  const rows = layoutRows(entries, separator, maxColumns);
  const separatorWidth = displayWidth(separator);
  const widest = rows.reduce((widestSoFar, row) => {
    const width = row.cells.reduce(
      (sum, cell) =>
        sum +
        (cell.separator ? separatorWidth : 0) +
        cell.entry.pieces.reduce((total, part) => total + displayWidth(part.text), 0),
      0
    );
    return Math.max(widestSoFar, width);
  }, 0);

  const count = rows.length === 1 ? "1 row" : `${rows.length} rows`;
  return `${widest} of ${maxColumns} cols, ${count}`;
}

export function displayWidth(value) {
  const segments = typeof Intl.Segmenter === "function"
    ? Array.from(new Intl.Segmenter(undefined, { granularity: "grapheme" }).segment(value), (entry) => entry.segment)
    : Array.from(value);

  return segments.reduce((width, grapheme) => {
    if (/^[\u0000-\u001f\u007f-\u009f]*$/u.test(grapheme)) {
      return width;
    }
    if (/\p{Extended_Pictographic}/u.test(grapheme)) {
      return width + 2;
    }
    if (/^\p{Regional_Indicator}{2}$/u.test(grapheme) || /\u20e3/u.test(grapheme)) {
      return width + 2;
    }
    const codePoint = grapheme.codePointAt(0);
    if (isWideCodePoint(codePoint)) {
      return width + 2;
    }
    if (/^[\p{Mark}\p{Format}]+$/u.test(grapheme)) {
      return width;
    }
    return width + 1;
  }, 0);
}

function isWideCodePoint(codePoint) {
  return codePoint >= 0x1100 && (
    codePoint <= 0x115f
    || codePoint === 0x2329
    || codePoint === 0x232a
    || codePoint >= 0x2e80 && codePoint <= 0xa4cf && codePoint !== 0x303f
    || codePoint >= 0xac00 && codePoint <= 0xd7a3
    || codePoint >= 0xf900 && codePoint <= 0xfaff
    || codePoint >= 0xfe10 && codePoint <= 0xfe19
    || codePoint >= 0xfe30 && codePoint <= 0xfe6f
    || codePoint >= 0xff00 && codePoint <= 0xff60
    || codePoint >= 0xffe0 && codePoint <= 0xffe6
    || codePoint >= 0x1b000 && codePoint <= 0x1b2ff
    || codePoint >= 0x20000 && codePoint <= 0x3fffd
  );
}

export function cut(text, maxChars) {
  const characters = Array.from(text);
  if (maxChars === 0 || characters.length <= maxChars) {
    return text;
  }
  return `${characters.slice(0, Math.max(0, maxChars - 1)).join("")}…`;
}

export function nearestDropSlot(boxes, x, y) {
  if (boxes.length === 0) {
    return null;
  }
  const rowDistance = (box) => {
    if (y < box.rect.top) {
      return box.rect.top - y;
    }
    if (y > box.rect.bottom) {
      return y - box.rect.bottom;
    }
    return 0;
  };
  const closestRow = Math.min(...boxes.map(rowDistance));
  const row = boxes.filter((box) => rowDistance(box) === closestRow);
  const centre = (box) => box.rect.left + (box.rect.right - box.rect.left) / 2;
  const nearest = row.reduce((best, box) =>
    Math.abs(centre(box) - x) < Math.abs(centre(best) - x) ? box : best
  );
  return { node: nearest.node, before: x < centre(nearest) };
}

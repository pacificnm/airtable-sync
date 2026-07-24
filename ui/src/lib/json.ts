/**
 * Extracts the first balanced JSON object/array from `text`, tolerating any
 * surrounding non-JSON text — e.g. `nest_app::lifecycle` console log lines
 * that can end up mixed into captured CLI stdout around a `--json` payload.
 * Returns `null` when no valid JSON value is found.
 */
export function extractJson<T = unknown>(rawText: string): T | null {
  // eslint-disable-next-line no-control-regex -- strips ANSI SGR color codes
  // (tracing's console output) so their CSI `[` doesn't get mistaken for the
  // start of a JSON array.
  const text = rawText.replace(/\x1b\[[0-9;]*m/g, "");
  const start = text.search(/[[{]/);
  if (start === -1) {
    return null;
  }

  const open = text[start];
  const close = open === "{" ? "}" : "]";
  let depth = 0;
  let inString = false;
  let escaped = false;

  for (let i = start; i < text.length; i++) {
    const char = text[i];
    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (char === "\\") {
        escaped = true;
      } else if (char === '"') {
        inString = false;
      }
      continue;
    }
    if (char === '"') {
      inString = true;
      continue;
    }
    if (char === open) {
      depth += 1;
    } else if (char === close) {
      depth -= 1;
      if (depth === 0) {
        try {
          return JSON.parse(text.slice(start, i + 1)) as T;
        } catch {
          return null;
        }
      }
    }
  }
  return null;
}

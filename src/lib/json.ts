const ESCAPES: Record<string, string> = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
};

function escapeHtml(text: string) {
  return text.replace(/[&<>]/g, (char) => ESCAPES[char]);
}

/**
 * Pretty-prints a value and wraps its tokens in spans.
 *
 * A 30-line tokenizer instead of highlight.js: the only language this app ever
 * shows is JSON, and the library was a third of the bundle.
 */
export function highlightJSON(value: unknown, indent = 2) {
  let text: string;
  try {
    text = JSON.stringify(value, replacer, indent) ?? 'null';
  } catch (error) {
    return escapeHtml(String(value));
  }

  return escapeHtml(text).replace(
    /("(?:\\.|[^"\\])*"(\s*:)?|\b(?:true|false|null)\b|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)/g,
    (match) => {
      let kind = 'number';
      if (match.startsWith('"')) {
        kind = match.trimEnd().endsWith(':') ? 'key' : 'string';
      } else if (match === 'true' || match === 'false') {
        kind = 'boolean';
      } else if (match === 'null') {
        kind = 'null';
      }
      return `<span class="tok-${kind}">${match}</span>`;
    },
  );
}

/** ids past 2^53 arrive as BigInt, which JSON.stringify refuses to serialize */
function replacer(_key: string, value: unknown) {
  return typeof value === 'bigint' ? value.toString() : value;
}

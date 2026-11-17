export type RenderMarkdownOptions = {
  /** Keep single line breaks inside a paragraph instead of joining lines with spaces. */
  preserveLineBreaks?: boolean;
};

const TABLE_ROW = /^\|.*\|$/;
const TABLE_SEPARATOR_CELL = /^:?-{3,}:?$/;
// A pipe that is not escaped with a backslash separates table cells.
const CELL_SEPARATOR = /(?<!\\)\|/;
const LIST_ITEM = /^([ \t]*)(?:([-*])|(\d+)[.)])\s+(.+)$/;
// Columns one tab of list indentation counts for. Matches the Jira converter.
const TAB_INDENT = 4;
const FIRST_LIST_NUMBER = 1;

type ListTag = "ul" | "ol";
type OpenList = {
  tag: ListTag;
  indent: number;
  /** The open item holds a nested list, so it closes on its own line. */
  itemHasChildren: boolean;
};

/**
 * Renders Markdown-flavored text as HTML. Raw HTML in the text is escaped.
 *
 * Supports headings, paragraphs, nested bullet and numbered lists, pipe tables,
 * fenced code, rules, and inline `**bold**`, `_italic_`, `~~strikethrough~~`,
 * `` `code` ``, `[label](url)` links to http, https, and mailto targets, and
 * backslash escapes. This is the syntax the Jira converter in
 * `src-tauri/src/jira/adf.rs` writes.
 */
export function renderMarkdown(markdown: string, options: RenderMarkdownOptions = {}): string {
  const lines = markdown.replace(/\r\n/g, "\n").split("\n");
  const output: string[] = [];
  const paragraph: string[] = [];
  const codeLines: string[] = [];
  const tableRows: string[][] = [];
  let tableHasHeader = false;
  const lists: OpenList[] = [];
  let inCodeBlock = false;

  function flushParagraph() {
    if (paragraph.length === 0) return;
    const body = options.preserveLineBreaks
      ? paragraph.map(renderInline).join("<br>")
      : renderInline(paragraph.join(" "));
    output.push(`<p>${body}</p>`);
    paragraph.length = 0;
  }

  function renderTableRow(cells: string[], cellTag: "th" | "td"): string {
    return `<tr>${cells.map((cell) => `<${cellTag}>${renderInline(cell)}</${cellTag}>`).join("")}</tr>`;
  }

  function closeTable() {
    if (tableRows.length === 0) return;
    const bodyRows = tableHasHeader ? tableRows.slice(1) : tableRows;
    output.push("<table>");
    if (tableHasHeader) {
      output.push(`<thead>${renderTableRow(tableRows[0], "th")}</thead>`);
    }
    output.push("<tbody>", ...bodyRows.map((row) => renderTableRow(row, "td")), "</tbody>", "</table>");
    tableRows.length = 0;
    tableHasHeader = false;
  }

  function closeItem(list: OpenList) {
    if (list.itemHasChildren) {
      output.push("</li>");
    } else {
      output[output.length - 1] += "</li>";
    }
    list.itemHasChildren = false;
  }

  function closeInnermostList() {
    const list = lists.pop();
    if (!list) return;
    closeItem(list);
    output.push(`</${list.tag}>`);
  }

  function closeList() {
    while (lists.length > 0) {
      closeInnermostList();
    }
  }

  /** Adds an item, nesting it in the item above when it is indented further. */
  function addListItem(tag: ListTag, indent: number, number: number, text: string) {
    let current = lists.at(-1);
    while (current && (current.indent > indent || (current.indent === indent && current.tag !== tag))) {
      closeInnermostList();
      current = lists.at(-1);
    }

    if (current && current.indent === indent) {
      closeItem(current);
    } else {
      if (current) {
        current.itemHasChildren = true;
      } else {
        flushParagraph();
      }
      const start = tag === "ol" && number !== FIRST_LIST_NUMBER ? ` start="${number}"` : "";
      output.push(`<${tag}${start}>`);
      lists.push({ tag, indent, itemHasChildren: false });
    }
    output.push(`<li>${renderInline(text)}`);
  }

  function closeCodeBlock() {
    output.push(`<pre><code>${escapeHtml(codeLines.join("\n"))}</code></pre>`);
    codeLines.length = 0;
    inCodeBlock = false;
  }

  for (const line of lines) {
    const trimmed = line.trim();

    if (inCodeBlock) {
      if (trimmed.startsWith("```")) {
        closeCodeBlock();
      } else {
        codeLines.push(line);
      }
      continue;
    }

    if (trimmed.length > 1 && TABLE_ROW.test(trimmed)) {
      flushParagraph();
      closeList();
      const cells = trimmed
        .slice(1, -1)
        .split(CELL_SEPARATOR)
        .map((cell) => cell.replace(/\\\|/g, "|").trim());
      if (cells.every((cell) => TABLE_SEPARATOR_CELL.test(cell))) {
        // A separator directly under the first row marks that row as the header.
        tableHasHeader = tableHasHeader || tableRows.length === 1;
      } else {
        tableRows.push(cells);
      }
      continue;
    }
    closeTable();

    if (trimmed.startsWith("```")) {
      flushParagraph();
      closeList();
      inCodeBlock = true;
      continue;
    }

    if (!trimmed) {
      flushParagraph();
      closeList();
      continue;
    }

    const heading = /^(#{1,4})\s+(.+)$/.exec(trimmed);
    if (heading) {
      flushParagraph();
      closeList();
      const level = heading[1].length;
      output.push(`<h${level}>${renderInline(heading[2])}</h${level}>`);
      continue;
    }

    if (/^-{3,}$/.test(trimmed)) {
      flushParagraph();
      closeList();
      output.push("<hr>");
      continue;
    }

    const item = LIST_ITEM.exec(line.trimEnd());
    if (item) {
      const [, leading, bullet, number, text] = item;
      const indent = [...leading].reduce((total, character) => total + (character === "\t" ? TAB_INDENT : 1), 0);
      addListItem(bullet ? "ul" : "ol", indent, bullet ? FIRST_LIST_NUMBER : Number(number), text.trim());
      continue;
    }

    closeList();
    paragraph.push(trimmed);
  }

  if (inCodeBlock) {
    closeCodeBlock();
  }
  closeTable();
  flushParagraph();
  closeList();

  return output.join("\n");
}

// Characters a backslash keeps literal. Matches the Jira converter.
const ESCAPABLE = new Set(["\\", "`", "*", "_", "~", "["]);
const HELD = /\u0000(\d+)\u0000/g;
const LINK = /\[([^\]]+)\]\(((?:https?:\/\/|mailto:)[^\s)\u0000]+)\)/g;
const STRONG = /\*\*(\S(?:[^]*?\S)?)\*\*/g;
const STRIKE = /~~(\S(?:[^]*?\S)?)~~/g;
// Underscores inside a word, as in snake_case, are not emphasis.
const EMPHASIS = /(^|[^\p{L}\p{N}])_(\S(?:[^]*?\S)?)_(?![\p{L}\p{N}])/gu;

function renderInline(value: string): string {
  // Escaped characters and code spans are held aside so later markup rules cannot touch them.
  const held: string[] = [];
  const hold = (html: string) => `\u0000${held.push(html) - 1}\u0000`;
  let rendered = "";

  for (let index = 0; index < value.length; index++) {
    const character = value[index];
    const next = value[index + 1];
    if (character === "\\" && next !== undefined && ESCAPABLE.has(next)) {
      rendered += hold(escapeHtml(next));
      index++;
      continue;
    }
    if (character === "`") {
      const end = value.indexOf("`", index + 1);
      if (end > index + 1) {
        rendered += hold(`<code>${escapeHtml(value.slice(index + 1, end))}</code>`);
        index = end;
        continue;
      }
    }
    rendered += escapeHtml(character);
  }

  rendered = rendered
    .replace(LINK, '<a href="$2" target="_blank" rel="noreferrer">$1</a>')
    .replace(STRONG, "<strong>$1</strong>")
    .replace(STRIKE, "<del>$1</del>")
    .replace(EMPHASIS, "$1<em>$2</em>");

  return rendered.replace(HELD, (_, index: string) => held[Number(index)] ?? "");
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

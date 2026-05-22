export function renderMarkdown(markdown: string): string {
  const lines = markdown.replace(/\r\n/g, "\n").split("\n");
  const output: string[] = [];
  const paragraph: string[] = [];
  const codeLines: string[] = [];
  let listType: "ul" | "ol" | null = null;
  let inCodeBlock = false;

  function flushParagraph() {
    if (paragraph.length === 0) return;
    output.push(`<p>${renderInline(paragraph.join(" "))}</p>`);
    paragraph.length = 0;
  }

  function closeList() {
    if (!listType) return;
    output.push(`</${listType}>`);
    listType = null;
  }

  function openList(nextType: "ul" | "ol") {
    if (listType === nextType) return;
    closeList();
    flushParagraph();
    output.push(`<${nextType}>`);
    listType = nextType;
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

    const unordered = /^[-*]\s+(.+)$/.exec(trimmed);
    if (unordered) {
      openList("ul");
      output.push(`<li>${renderInline(unordered[1])}</li>`);
      continue;
    }

    const ordered = /^\d+[.)]\s+(.+)$/.exec(trimmed);
    if (ordered) {
      openList("ol");
      output.push(`<li>${renderInline(ordered[1])}</li>`);
      continue;
    }

    closeList();
    paragraph.push(trimmed);
  }

  if (inCodeBlock) {
    closeCodeBlock();
  }
  flushParagraph();
  closeList();

  return output.join("\n");
}

function renderInline(value: string): string {
  const codeSpans: string[] = [];
  let rendered = escapeHtml(value).replace(/`([^`]+)`/g, (_, code: string) => {
    const index = codeSpans.push(`<code>${code}</code>`) - 1;
    return `\u0000CODE${index}\u0000`;
  });

  rendered = rendered.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  rendered = rendered.replace(/\u0000CODE(\d+)\u0000/g, (_, index: string) => codeSpans[Number(index)] ?? "");

  return rendered;
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

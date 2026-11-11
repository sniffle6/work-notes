import { describe, expect, it } from "vitest";

import { renderMarkdown } from "./markdown";

describe("renderMarkdown", () => {
  it("renders headings, lists, and inline code while escaping raw HTML", () => {
    const html = renderMarkdown("## Root cause\n\n- Missing `TransactionEvent`\n- <script>alert(1)</script>");

    expect(html).toContain("<h2>Root cause</h2>");
    expect(html).toContain("<li>Missing <code>TransactionEvent</code></li>");
    expect(html).toContain("&lt;script&gt;alert(1)&lt;/script&gt;");
    expect(html).not.toContain("<script>");
  });

  it("renders pipe tables with a header row and escaped cells", () => {
    const html = renderMarkdown(
      "Before\n| Do this | Expect |\n| --- | --- |\n| Insert `bill` | <b>Credits</b> rise |\nAfter",
    );

    expect(html).toBe(
      [
        "<p>Before</p>",
        "<table>",
        "<thead><tr><th>Do this</th><th>Expect</th></tr></thead>",
        "<tbody>",
        "<tr><td>Insert <code>bill</code></td><td>&lt;b&gt;Credits&lt;/b&gt; rise</td></tr>",
        "</tbody>",
        "</table>",
        "<p>After</p>",
      ].join("\n"),
    );
  });

  it("renders a pipe table without a separator row as body rows only", () => {
    const html = renderMarkdown("| a | b |\n| c | d |");

    expect(html).toBe(
      ["<table>", "<tbody>", "<tr><td>a</td><td>b</td></tr>", "<tr><td>c</td><td>d</td></tr>", "</tbody>", "</table>"].join(
        "\n",
      ),
    );
  });

  it("renders a line of dashes as a divider instead of text", () => {
    expect(renderMarkdown("Above\n\n---\n\nBelow")).toBe("<p>Above</p>\n<hr>\n<p>Below</p>");
    expect(renderMarkdown("Above\n-----\nBelow")).toBe("<p>Above</p>\n<hr>\n<p>Below</p>");
    expect(renderMarkdown("- item\n---")).toBe("<ul>\n<li>item</li>\n</ul>\n<hr>");
  });

  it("joins paragraph lines with spaces unless line breaks are preserved", () => {
    expect(renderMarkdown("first\nsecond")).toBe("<p>first second</p>");
    expect(renderMarkdown("first\nsecond", { preserveLineBreaks: true })).toBe("<p>first<br>second</p>");
  });
});

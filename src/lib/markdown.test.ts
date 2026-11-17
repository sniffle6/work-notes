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

  it("renders italic, strikethrough, and combined emphasis", () => {
    expect(renderMarkdown("Use _italic_, ~~gone~~ and **_both_**.")).toBe(
      "<p>Use <em>italic</em>, <del>gone</del> and <strong><em>both</em></strong>.</p>",
    );
  });

  it("leaves underscores inside words and lone markers as text", () => {
    expect(renderMarkdown("snake_case_name, 2 * 3 and a ~ b")).toBe(
      "<p>snake_case_name, 2 * 3 and a ~ b</p>",
    );
  });

  it("renders web and mail links that open outside the app", () => {
    expect(renderMarkdown("See [the docs](https://example.test/a_b?x=1&y=2) or [`run()`](mailto:dev@example.test).")).toBe(
      '<p>See <a href="https://example.test/a_b?x=1&amp;y=2" target="_blank" rel="noreferrer">the docs</a> or ' +
        '<a href="mailto:dev@example.test" target="_blank" rel="noreferrer"><code>run()</code></a>.</p>',
    );
  });

  it("keeps links with other schemes as text", () => {
    expect(renderMarkdown("[click](javascript:alert(1))")).toBe("<p>[click](javascript:alert(1))</p>");
  });

  it("treats backslash-escaped markup characters as text", () => {
    expect(renderMarkdown("\\*\\*not bold\\*\\*, \\_not italic\\_, \\`tick\\`, \\[x](https://example.test) and C:\\code\\notes\\\\")).toBe(
      "<p>**not bold**, _not italic_, `tick`, [x](https://example.test) and C:\\code\\notes\\</p>",
    );
  });

  it("keeps code spans literal", () => {
    expect(renderMarkdown("`a_b **c** \\_`")).toBe("<p><code>a_b **c** \\_</code></p>");
  });

  it("reads an escaped pipe as part of a table cell", () => {
    expect(renderMarkdown("| a \\| b | c |")).toBe(
      ["<table>", "<tbody>", "<tr><td>a | b</td><td>c</td></tr>", "</tbody>", "</table>"].join("\n"),
    );
  });

  it("nests indented list items inside the item above them", () => {
    expect(renderMarkdown("- parent\n  1. first\n  2. second\n- sibling")).toBe(
      [
        "<ul>",
        "<li>parent",
        "<ol>",
        "<li>first</li>",
        "<li>second</li>",
        "</ol>",
        "</li>",
        "<li>sibling</li>",
        "</ul>",
      ].join("\n"),
    );
  });

  it("starts a numbered list at its first number", () => {
    expect(renderMarkdown("4. fourth\n5. fifth")).toBe('<ol start="4">\n<li>fourth</li>\n<li>fifth</li>\n</ol>');
  });

  it("joins paragraph lines with spaces unless line breaks are preserved", () => {
    expect(renderMarkdown("first\nsecond")).toBe("<p>first second</p>");
    expect(renderMarkdown("first\nsecond", { preserveLineBreaks: true })).toBe("<p>first<br>second</p>");
  });
});

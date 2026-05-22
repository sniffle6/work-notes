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
});

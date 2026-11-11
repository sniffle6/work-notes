//! Conversion between Atlassian Document Format (ADF) and Markdown-flavored
//! plain text. Jira Cloud REST v3 uses ADF for descriptions and comments.
//!
//! Reading renders more than writing can express. Use
//! [`adf_round_trips_as_text`] before offering a plain-text edit of an
//! existing document.

use serde_json::{json, Value};

/// Renders an ADF node (or a legacy plain string) as readable text.
pub fn adf_to_text(node: &Value) -> String {
    match node {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        _ => render_block(node),
    }
}

/// Builds an ADF document from plain text.
///
/// Supports paragraphs, `#` headings, `-`/`*` bullets, `1.` numbered lists,
/// fenced code blocks, and `---` rules. Everything else becomes paragraph
/// text.
pub fn text_to_adf(text: &str) -> Value {
    let normalized = text.replace("\r\n", "\n");
    let mut builder = DocumentBuilder::default();
    let mut lines = normalized.lines();

    while let Some(line) = lines.next() {
        let line = line.trim_end();
        let content = line.trim_start();

        if let Some(language) = content.strip_prefix(CODE_FENCE) {
            let code: Vec<&str> = lines
                .by_ref()
                .take_while(|code_line| code_line.trim() != CODE_FENCE)
                .collect();
            builder.push_block(code_block(language.trim(), &code.join("\n")));
        } else if content.is_empty() {
            builder.end_block();
        } else if let Some((level, title)) = parse_heading(content) {
            builder.push_block(json!({
                "type": "heading",
                "attrs": { "level": level },
                "content": [text_node(title)],
            }));
        } else if content == RULE_MARKER {
            builder.push_block(json!({ "type": "rule" }));
        } else if let Some((kind, item)) = parse_list_item(content) {
            builder.push_list_item(kind, item);
        } else {
            builder.push_paragraph_line(line);
        }
    }

    builder.finish()
}

/// Reports whether [`text_to_adf`] can rebuild this document from
/// [`adf_to_text`] output without dropping structure or inline formatting.
pub fn adf_round_trips_as_text(node: &Value) -> bool {
    match node {
        Value::Null | Value::String(_) => true,
        _ => block_round_trips(node),
    }
}

const CODE_FENCE: &str = "```";
const RULE_MARKER: &str = "---";
const MAX_HEADING_LEVEL: usize = 6;

fn node_type(node: &Value) -> &str {
    node.get("type").and_then(Value::as_str).unwrap_or_default()
}

fn children(node: &Value) -> &[Value] {
    node.get("content")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn attr<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    node.get("attrs").and_then(|attrs| attrs.get(name))
}

fn attr_str<'a>(node: &'a Value, name: &str) -> &'a str {
    attr(node, name).and_then(Value::as_str).unwrap_or_default()
}

fn render_blocks(nodes: &[Value], separator: &str) -> String {
    nodes
        .iter()
        .map(render_block)
        .filter(|block| !block.is_empty())
        .collect::<Vec<_>>()
        .join(separator)
}

fn render_block(node: &Value) -> String {
    match node_type(node) {
        "paragraph" => render_inline_children(node),
        "heading" => {
            let level = attr(node, "level")
                .and_then(Value::as_u64)
                .map_or(1, |level| (level as usize).clamp(1, MAX_HEADING_LEVEL));
            format!("{} {}", "#".repeat(level), render_inline_children(node))
        }
        "bulletList" => render_list(node, |_| "- ".to_string()),
        "orderedList" => {
            let first = attr(node, "order").and_then(Value::as_u64).unwrap_or(1);
            render_list(node, |index| format!("{}. ", first + index as u64))
        }
        "codeBlock" => format!(
            "{CODE_FENCE}{}\n{}\n{CODE_FENCE}",
            attr_str(node, "language"),
            render_inline_children(node)
        ),
        "blockquote" => render_blocks(children(node), "\n\n")
            .lines()
            .map(|line| format!("> {line}").trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n"),
        "rule" => RULE_MARKER.to_string(),
        "table" => render_table(node),
        "text" | "hardBreak" | "mention" | "emoji" | "inlineCard" => render_inline(node),
        _ => render_blocks(children(node), "\n\n"),
    }
}

fn render_list(node: &Value, marker: impl Fn(usize) -> String) -> String {
    children(node)
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let marker = marker(index);
            let indent = " ".repeat(marker.len());
            let body = render_blocks(children(item), "\n").replace('\n', &format!("\n{indent}"));
            format!("{marker}{body}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_table(node: &Value) -> String {
    let mut lines = Vec::new();
    for (index, row) in children(node).iter().enumerate() {
        let cells = children(row);
        let rendered: Vec<String> = cells
            .iter()
            .map(|cell| render_blocks(children(cell), " ").replace('\n', " "))
            .collect();
        lines.push(format!("| {} |", rendered.join(" | ")));
        let is_header = cells.iter().any(|cell| node_type(cell) == "tableHeader");
        if index == 0 && is_header {
            lines.push(format!("| {} |", vec!["---"; cells.len()].join(" | ")));
        }
    }
    lines.join("\n")
}

fn render_inline_children(node: &Value) -> String {
    children(node).iter().map(render_inline).collect()
}

fn render_inline(node: &Value) -> String {
    match node_type(node) {
        "text" => {
            let text = node.get("text").and_then(Value::as_str).unwrap_or_default();
            if has_mark(node, "code") {
                format!("`{text}`")
            } else {
                text.to_string()
            }
        }
        "hardBreak" => "\n".to_string(),
        "mention" => attr_str(node, "text").to_string(),
        "emoji" => attr_str(node, "shortName").to_string(),
        "inlineCard" => attr_str(node, "url").to_string(),
        _ => render_inline_children(node),
    }
}

fn marks(node: &Value) -> &[Value] {
    node.get("marks")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn has_mark(node: &Value, name: &str) -> bool {
    marks(node).iter().any(|mark| node_type(mark) == name)
}

fn text_node(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}

fn code_block(language: &str, code: &str) -> Value {
    let mut block = json!({ "type": "codeBlock" });
    if !language.is_empty() {
        block["attrs"] = json!({ "language": language });
    }
    if !code.is_empty() {
        block["content"] = json!([text_node(code)]);
    }
    block
}

fn parse_heading(content: &str) -> Option<(usize, &str)> {
    let level = content.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=MAX_HEADING_LEVEL).contains(&level) {
        return None;
    }
    let title = content[level..].strip_prefix(' ')?.trim();
    (!title.is_empty()).then_some((level, title))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListKind {
    Bullet,
    Ordered,
}

impl ListKind {
    fn node_type(self) -> &'static str {
        match self {
            Self::Bullet => "bulletList",
            Self::Ordered => "orderedList",
        }
    }
}

fn parse_list_item(content: &str) -> Option<(ListKind, &str)> {
    if let Some(item) = content
        .strip_prefix("- ")
        .or_else(|| content.strip_prefix("* "))
    {
        return Some((ListKind::Bullet, item.trim()));
    }

    let digits = content.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let item = content[digits..]
        .strip_prefix(". ")
        .or_else(|| content[digits..].strip_prefix(") "))?;
    Some((ListKind::Ordered, item.trim()))
}

#[derive(Default)]
struct DocumentBuilder {
    blocks: Vec<Value>,
    paragraph_lines: Vec<String>,
    list: Option<(ListKind, Vec<Value>)>,
}

impl DocumentBuilder {
    fn push_paragraph_line(&mut self, line: &str) {
        self.end_list();
        self.paragraph_lines.push(line.to_string());
    }

    fn push_list_item(&mut self, kind: ListKind, item: &str) {
        self.end_paragraph();
        if self.list.as_ref().is_some_and(|(open, _)| *open != kind) {
            self.end_list();
        }
        let content = if item.is_empty() {
            json!([])
        } else {
            json!([text_node(item)])
        };
        self.list
            .get_or_insert_with(|| (kind, Vec::new()))
            .1
            .push(json!({
                "type": "listItem",
                "content": [{ "type": "paragraph", "content": content }],
            }));
    }

    fn push_block(&mut self, block: Value) {
        self.end_block();
        self.blocks.push(block);
    }

    fn end_block(&mut self) {
        self.end_paragraph();
        self.end_list();
    }

    fn end_paragraph(&mut self) {
        if self.paragraph_lines.is_empty() {
            return;
        }
        let mut content = Vec::new();
        for (index, line) in self.paragraph_lines.drain(..).enumerate() {
            if index > 0 {
                content.push(json!({ "type": "hardBreak" }));
            }
            content.push(text_node(&line));
        }
        self.blocks
            .push(json!({ "type": "paragraph", "content": content }));
    }

    fn end_list(&mut self) {
        if let Some((kind, items)) = self.list.take() {
            self.blocks
                .push(json!({ "type": kind.node_type(), "content": items }));
        }
    }

    fn finish(mut self) -> Value {
        self.end_block();
        if self.blocks.is_empty() {
            self.blocks
                .push(json!({ "type": "paragraph", "content": [] }));
        }
        json!({ "type": "doc", "version": 1, "content": self.blocks })
    }
}

fn block_round_trips(node: &Value) -> bool {
    match node_type(node) {
        "doc" => children(node).iter().all(block_round_trips),
        "paragraph" | "heading" => children(node).iter().all(inline_round_trips),
        "bulletList" => children(node).iter().all(list_item_round_trips),
        "orderedList" => {
            attr(node, "order").and_then(Value::as_u64).unwrap_or(1) == 1
                && children(node).iter().all(list_item_round_trips)
        }
        "codeBlock" => children(node)
            .iter()
            .all(|child| node_type(child) == "text" && marks(child).is_empty()),
        "rule" => true,
        _ => false,
    }
}

fn list_item_round_trips(item: &Value) -> bool {
    node_type(item) == "listItem"
        && matches!(children(item), [paragraph] if node_type(paragraph) == "paragraph"
            && children(paragraph).iter().all(inline_round_trips))
}

fn inline_round_trips(node: &Value) -> bool {
    match node_type(node) {
        "text" => marks(node).is_empty(),
        "hardBreak" => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{adf_round_trips_as_text, adf_to_text, text_to_adf};
    use serde_json::{json, Value};

    fn doc(content: Value) -> Value {
        json!({ "type": "doc", "version": 1, "content": content })
    }

    fn text(value: &str) -> Value {
        json!({ "type": "text", "text": value })
    }

    fn paragraph(content: Value) -> Value {
        json!({ "type": "paragraph", "content": content })
    }

    fn list_item(value: &str) -> Value {
        json!({ "type": "listItem", "content": [paragraph(json!([text(value)]))] })
    }

    #[test]
    fn renders_headings_paragraphs_and_lists() {
        let document = doc(json!([
            { "type": "heading", "attrs": { "level": 2 }, "content": [text("Context")] },
            paragraph(json!([text("Printer jams on reboot.")])),
            { "type": "bulletList", "content": [list_item("Paper out"), list_item("Cover open")] },
            { "type": "orderedList", "content": [list_item("Power cycle"), list_item("Print ticket")] },
        ]));

        assert_eq!(
            adf_to_text(&document),
            "## Context\n\nPrinter jams on reboot.\n\n- Paper out\n- Cover open\n\n1. Power cycle\n2. Print ticket"
        );
    }

    #[test]
    fn renders_inline_nodes() {
        let document = doc(json!([paragraph(json!([
            text("Ask "),
            { "type": "mention", "attrs": { "id": "abc", "text": "@Maya" } },
            text(" about "),
            { "type": "text", "text": "UsbUpdater.cs", "marks": [{ "type": "code" }] },
            { "type": "hardBreak" },
            { "type": "emoji", "attrs": { "shortName": ":warning:" } },
            text(" see "),
            { "type": "inlineCard", "attrs": { "url": "https://example.test/doc" } },
        ]))]));

        assert_eq!(
            adf_to_text(&document),
            "Ask @Maya about `UsbUpdater.cs`\n:warning: see https://example.test/doc"
        );
    }

    #[test]
    fn renders_code_blocks_rules_and_quotes() {
        let document = doc(json!([
            { "type": "codeBlock", "attrs": { "language": "json" }, "content": [text("{\n  \"a\": 1\n}")] },
            { "type": "rule" },
            { "type": "blockquote", "content": [paragraph(json!([text("Quoted line")]))] },
        ]));

        assert_eq!(
            adf_to_text(&document),
            "```json\n{\n  \"a\": 1\n}\n```\n\n---\n\n> Quoted line"
        );
    }

    #[test]
    fn renders_tables_as_pipe_rows() {
        let cell = |kind: &str, value: &str| json!({ "type": kind, "content": [paragraph(json!([text(value)]))] });
        let document = doc(json!([{
            "type": "table",
            "content": [
                { "type": "tableRow", "content": [cell("tableHeader", "Do this"), cell("tableHeader", "Expect")] },
                { "type": "tableRow", "content": [cell("tableCell", "Insert bill"), cell("tableCell", "Credits rise")] },
            ],
        }]));

        assert_eq!(
            adf_to_text(&document),
            "| Do this | Expect |\n| --- | --- |\n| Insert bill | Credits rise |"
        );
    }

    #[test]
    fn renders_content_of_unknown_containers() {
        let document = doc(json!([{
            "type": "panel",
            "attrs": { "panelType": "warning" },
            "content": [paragraph(json!([text("Known gap")]))],
        }]));

        assert_eq!(adf_to_text(&document), "Known gap");
    }

    #[test]
    fn plain_strings_and_null_are_accepted() {
        assert_eq!(
            adf_to_text(&json!("legacy description")),
            "legacy description"
        );
        assert_eq!(adf_to_text(&Value::Null), "");
    }

    #[test]
    fn builds_paragraphs_with_hard_breaks() {
        let document = text_to_adf("First line\r\nsecond line\n\nNext paragraph");

        assert_eq!(
            document,
            doc(json!([
                paragraph(
                    json!([text("First line"), { "type": "hardBreak" }, text("second line")])
                ),
                paragraph(json!([text("Next paragraph")])),
            ]))
        );
    }

    #[test]
    fn builds_bullet_and_numbered_lists() {
        let document = text_to_adf("Steps:\n- one\n* two\n\n1. first\n2) second");

        assert_eq!(
            document,
            doc(json!([
                paragraph(json!([text("Steps:")])),
                { "type": "bulletList", "content": [list_item("one"), list_item("two")] },
                { "type": "orderedList", "content": [list_item("first"), list_item("second")] },
            ]))
        );
    }

    #[test]
    fn builds_headings_code_blocks_and_rules() {
        let document =
            text_to_adf("## Dev Notes\n```rust\nlet a = 1;\n\nlet b = 2;\n```\n---\nDone");

        assert_eq!(
            document,
            doc(json!([
                { "type": "heading", "attrs": { "level": 2 }, "content": [text("Dev Notes")] },
                { "type": "codeBlock", "attrs": { "language": "rust" }, "content": [text("let a = 1;\n\nlet b = 2;")] },
                { "type": "rule" },
                paragraph(json!([text("Done")])),
            ]))
        );
    }

    #[test]
    fn code_block_without_language_omits_attrs() {
        assert_eq!(
            text_to_adf("```\nplain\n```"),
            doc(json!([{ "type": "codeBlock", "content": [text("plain")] }]))
        );
    }

    #[test]
    fn hash_without_space_is_not_a_heading() {
        assert_eq!(
            text_to_adf("#hashtag"),
            doc(json!([paragraph(json!([text("#hashtag")]))]))
        );
    }

    #[test]
    fn empty_text_becomes_one_empty_paragraph() {
        assert_eq!(
            text_to_adf("  \n\n"),
            doc(json!([{ "type": "paragraph", "content": [] }]))
        );
    }

    #[test]
    fn supported_documents_round_trip() {
        let source = "## Context\n\nPrinter jams on reboot.\nSecond line.\n\n- Paper out\n- Cover open\n\n1. Power cycle\n2. Print ticket\n\n```sh\nls -la\n```\n\n---\n\nDone";
        let document = text_to_adf(source);

        assert!(adf_round_trips_as_text(&document));
        assert_eq!(adf_to_text(&document), source);
        assert_eq!(text_to_adf(&adf_to_text(&document)), document);
    }

    #[test]
    fn formatting_the_editor_cannot_write_is_reported() {
        let table = doc(json!([{ "type": "table", "content": [] }]));
        let marked = doc(json!([paragraph(json!([
            { "type": "text", "text": "bold", "marks": [{ "type": "strong" }] },
        ]))]));
        let nested = doc(json!([{
            "type": "bulletList",
            "content": [{
                "type": "listItem",
                "content": [
                    paragraph(json!([text("parent")])),
                    { "type": "bulletList", "content": [list_item("child")] },
                ],
            }],
        }]));
        let mention = doc(json!([paragraph(json!([
            { "type": "mention", "attrs": { "id": "abc", "text": "@Maya" } },
        ]))]));

        for document in [table, marked, nested, mention] {
            assert!(
                !adf_round_trips_as_text(&document),
                "{document} should be lossy"
            );
        }
    }

    #[test]
    fn plain_string_and_null_descriptions_are_safe_to_edit() {
        assert!(adf_round_trips_as_text(&json!("legacy description")));
        assert!(adf_round_trips_as_text(&Value::Null));
    }
}

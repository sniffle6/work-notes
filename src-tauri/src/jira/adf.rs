//! Conversion between Atlassian Document Format (ADF) and Markdown-flavored
//! plain text. Jira Cloud REST v3 uses ADF for descriptions and comments.
//!
//! Reading renders more than writing can express. Use
//! [`adf_round_trips_as_text`] before offering a plain-text edit of an
//! existing document.

use serde_json::{json, Map, Value};

/// Renders an ADF node (or a legacy plain string) as readable text.
///
/// Bold, italic, strikethrough, code, and link marks render as Markdown.
/// Characters that [`text_to_adf`] would read as that markup are
/// backslash-escaped where they occur in plain text.
pub fn adf_to_text(node: &Value) -> String {
    match node {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        _ => render_block(node),
    }
}

/// Builds an ADF document from Markdown-flavored text.
///
/// Blocks: paragraphs, `#` headings, `-`/`*` bullets, `1.` numbered lists
/// (nested by indentation), pipe tables, fenced code blocks, and `---` rules.
/// Inline: `**bold**`, `_italic_`, `~~strikethrough~~`, `` `code` ``, and
/// `[label](url)` for `http`, `https`, and `mailto` links. A backslash before
/// a markup character keeps it literal. Everything else becomes paragraph
/// text.
pub fn text_to_adf(text: &str) -> Value {
    let normalized = text.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    let mut blocks = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index].trim_end();
        let content = line.trim_start();

        if let Some(language) = content.strip_prefix(CODE_FENCE) {
            end_paragraph(&mut paragraph, &mut blocks);
            let code_end = lines[index + 1..]
                .iter()
                .position(|code_line| code_line.trim() == CODE_FENCE)
                .map_or(lines.len(), |offset| index + 1 + offset);
            blocks.push(code_block(
                language.trim(),
                &lines[index + 1..code_end].join("\n"),
            ));
            index = code_end + 1;
            continue;
        }
        if parse_list_line(line).is_some() {
            end_paragraph(&mut paragraph, &mut blocks);
            let list_lines: Vec<ListLine> = lines[index..]
                .iter()
                .map_while(|list_line| parse_list_line(list_line.trim_end()))
                .collect();
            index += list_lines.len();
            blocks.extend(build_lists(&list_lines));
            continue;
        }
        if is_table_row(content) {
            end_paragraph(&mut paragraph, &mut blocks);
            let rows: Vec<Vec<String>> = lines[index..]
                .iter()
                .map(|row| row.trim())
                .take_while(|row| is_table_row(row))
                .map(split_table_row)
                .collect();
            index += rows.len();
            blocks.push(build_table(&rows));
            continue;
        }

        if content.is_empty() {
            end_paragraph(&mut paragraph, &mut blocks);
        } else if let Some((level, title)) = parse_heading(content) {
            end_paragraph(&mut paragraph, &mut blocks);
            blocks.push(json!({
                "type": "heading",
                "attrs": { "level": level },
                "content": parse_inline(title),
            }));
        } else if content == RULE_MARKER {
            end_paragraph(&mut paragraph, &mut blocks);
            blocks.push(json!({ "type": "rule" }));
        } else {
            paragraph.push(line);
        }
        index += 1;
    }

    end_paragraph(&mut paragraph, &mut blocks);
    if blocks.is_empty() {
        blocks.push(json!({ "type": "paragraph", "content": [] }));
    }
    json!({ "type": "doc", "version": 1, "content": blocks })
}

/// Reports whether saving [`adf_to_text`] output through [`text_to_adf`]
/// rebuilds this document.
///
/// The check performs that round trip and compares the result with the
/// original. Differences Jira does not show are ignored: node ids, how text
/// is split into nodes, the order of marks, whitespace at line ends, empty
/// paragraphs, and a table's default layout attributes.
pub fn adf_round_trips_as_text(node: &Value) -> bool {
    match node {
        Value::Null | Value::String(_) => true,
        _ => normalized(node) == normalized(&text_to_adf(&adf_to_text(node))),
    }
}

const CODE_FENCE: &str = "```";
const RULE_MARKER: &str = "---";
const MAX_HEADING_LEVEL: usize = 6;
const ESCAPE: char = '\\';
const CODE_DELIMITER: char = '`';
/// Columns one tab of list indentation counts for.
const TAB_INDENT: usize = 4;
/// Link targets the editor writes. Other schemes stay plain text.
const LINK_SCHEMES: [&str; 3] = ["https://", "http://", "mailto:"];

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

fn node_text(node: &Value) -> &str {
    node.get("text").and_then(Value::as_str).unwrap_or_default()
}

fn marks(node: &Value) -> &[Value] {
    node.get("marks")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
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
        "paragraph" => render_inline_nodes(children(node)),
        "heading" => {
            let level = attr(node, "level")
                .and_then(Value::as_u64)
                .map_or(1, |level| (level as usize).clamp(1, MAX_HEADING_LEVEL));
            format!(
                "{} {}",
                "#".repeat(level),
                render_inline_nodes(children(node))
            )
        }
        "bulletList" => render_list(node, |_| "- ".to_string()),
        "orderedList" => {
            let first = attr(node, "order").and_then(Value::as_u64).unwrap_or(1);
            render_list(node, |index| format!("{}. ", first + index as u64))
        }
        // Code is literal: no marks and no escaping.
        "codeBlock" => format!(
            "{CODE_FENCE}{}\n{}\n{CODE_FENCE}",
            attr_str(node, "language"),
            children(node).iter().map(node_text).collect::<String>()
        ),
        "blockquote" => render_blocks(children(node), "\n\n")
            .lines()
            .map(|line| format!("> {line}").trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n"),
        "rule" => RULE_MARKER.to_string(),
        "table" => render_table(node),
        "text" | "hardBreak" | "mention" | "emoji" | "inlineCard" => {
            render_inline_nodes(std::slice::from_ref(node))
        }
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
            .map(|cell| {
                render_blocks(children(cell), " ")
                    .replace('\n', " ")
                    .replace('|', "\\|")
            })
            .collect();
        lines.push(format!("| {} |", rendered.join(" | ")));
        let is_header = cells.iter().any(|cell| node_type(cell) == "tableHeader");
        if index == 0 && is_header {
            lines.push(format!("| {} |", vec!["---"; cells.len()].join(" | ")));
        }
    }
    lines.join("\n")
}

/// A mark the editor can write, in the order marks nest in the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MarkKind {
    Link,
    Strong,
    Em,
    Strike,
    Code,
}

impl MarkKind {
    const ALL: [Self; 5] = [Self::Link, Self::Strong, Self::Em, Self::Strike, Self::Code];
    /// Marks written with the same delimiter on both sides. `Em` closes only
    /// at a word boundary, so `snake_case` stays plain.
    const DELIMITED: [Self; 3] = [Self::Strong, Self::Strike, Self::Em];

    fn adf_type(self) -> &'static str {
        match self {
            Self::Link => "link",
            Self::Strong => "strong",
            Self::Em => "em",
            Self::Strike => "strike",
            Self::Code => "code",
        }
    }

    fn delimiter(self) -> &'static str {
        match self {
            Self::Link => "[",
            Self::Strong => "**",
            Self::Em => "_",
            Self::Strike => "~~",
            Self::Code => "`",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InlineMark {
    kind: MarkKind,
    /// Link target; empty for other marks.
    href: String,
}

impl InlineMark {
    fn plain(kind: MarkKind) -> Self {
        Self {
            kind,
            href: String::new(),
        }
    }

    fn closing(&self) -> String {
        match self.kind {
            MarkKind::Link => format!("]({})", self.href),
            kind => kind.delimiter().to_string(),
        }
    }

    fn to_adf(&self) -> Value {
        match self.kind {
            MarkKind::Link => json!({ "type": "link", "attrs": { "href": self.href } }),
            kind => json!({ "type": kind.adf_type() }),
        }
    }
}

/// The marks on a text node that the editor can write, outermost first.
fn writable_marks(node: &Value) -> Vec<InlineMark> {
    MarkKind::ALL
        .iter()
        .filter_map(|kind| {
            let mark = marks(node)
                .iter()
                .find(|mark| node_type(mark) == kind.adf_type())?;
            Some(InlineMark {
                kind: *kind,
                href: attr_str(mark, "href").to_string(),
            })
        })
        .collect()
}

fn close_marks(text: &mut String, open: &mut Vec<InlineMark>, keep: usize) {
    while open.len() > keep {
        if let Some(mark) = open.pop() {
            text.push_str(&mark.closing());
        }
    }
}

/// Renders inline nodes, keeping a mark open across neighbouring text nodes
/// that share it.
fn render_inline_nodes(nodes: &[Value]) -> String {
    let mut text = String::new();
    let mut open: Vec<InlineMark> = Vec::new();

    for node in nodes {
        let is_text = node_type(node) == "text";
        let wanted = if is_text {
            writable_marks(node)
        } else {
            Vec::new()
        };
        let shared = open
            .iter()
            .zip(&wanted)
            .take_while(|(current, next)| current == next)
            .count();
        close_marks(&mut text, &mut open, shared);
        for mark in &wanted[shared..] {
            text.push_str(mark.kind.delimiter());
            open.push(mark.clone());
        }

        if !is_text {
            text.push_str(&render_inline(node));
        } else if open.last().is_some_and(|mark| mark.kind == MarkKind::Code) {
            // A code span is literal and ends with its own node.
            text.push_str(node_text(node));
            let code_depth = open.len() - 1;
            close_marks(&mut text, &mut open, code_depth);
        } else {
            text.push_str(&escape_markup(node_text(node)));
        }
    }

    close_marks(&mut text, &mut open, 0);
    text
}

fn render_inline(node: &Value) -> String {
    match node_type(node) {
        "hardBreak" => "\n".to_string(),
        "mention" => attr_str(node, "text").to_string(),
        "emoji" => attr_str(node, "shortName").to_string(),
        "inlineCard" => attr_str(node, "url").to_string(),
        _ => render_inline_nodes(children(node)),
    }
}

fn is_escapable(character: char) -> bool {
    matches!(character, ESCAPE | CODE_DELIMITER | '*' | '_' | '~' | '[')
}

fn can_open_emphasis(previous: Option<char>, next: Option<char>) -> bool {
    !previous.is_some_and(char::is_alphanumeric)
        && next.is_some_and(|character| !character.is_whitespace())
}

fn can_close_emphasis(previous: Option<char>, next: Option<char>) -> bool {
    previous.is_some_and(|character| !character.is_whitespace())
        && !next.is_some_and(char::is_alphanumeric)
}

/// Escapes the characters in plain text that [`parse_inline`] would read as
/// markup. Characters that cannot start or end markup where they stand are
/// left alone, so ordinary punctuation stays readable.
fn escape_markup(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    let mut escaped = String::with_capacity(text.len());

    for (index, &character) in characters.iter().enumerate() {
        let previous = index.checked_sub(1).map(|before| characters[before]);
        let next = characters.get(index + 1).copied();
        let is_markup = match character {
            ESCAPE => next.is_some_and(is_escapable),
            CODE_DELIMITER => true,
            '*' | '~' => previous == Some(character) || next == Some(character),
            '_' => can_open_emphasis(previous, next) || can_close_emphasis(previous, next),
            '[' => characters[index..]
                .windows(2)
                .any(|pair| *pair == [']', '(']),
            _ => false,
        };
        if is_markup {
            escaped.push(ESCAPE);
        }
        escaped.push(character);
    }

    escaped
}

fn parse_inline(text: &str) -> Vec<Value> {
    let characters: Vec<char> = text.chars().collect();
    let mut nodes = Vec::new();
    parse_span(&characters, &[], &mut nodes);
    nodes
}

fn is_escape_at(characters: &[char], index: usize) -> bool {
    characters[index] == ESCAPE && characters.get(index + 1).copied().is_some_and(is_escapable)
}

fn find_character(characters: &[char], from: usize, wanted: char) -> Option<usize> {
    characters
        .get(from..)?
        .iter()
        .position(|character| *character == wanted)
        .map(|offset| from + offset)
}

/// The index after the code span opening at `index`, when it is closed.
fn code_span_end(characters: &[char], index: usize) -> Option<usize> {
    find_character(characters, index + 1, CODE_DELIMITER).map(|end| end + 1)
}

/// Parses inline markup in `characters`, which already carry the `active`
/// marks, and appends the resulting text nodes.
fn parse_span(characters: &[char], active: &[InlineMark], nodes: &mut Vec<Value>) {
    let mut plain = String::new();
    let mut index = 0;

    'characters: while index < characters.len() {
        if is_escape_at(characters, index) {
            plain.push(characters[index + 1]);
            index += 2;
            continue;
        }

        let character = characters[index];
        if character == CODE_DELIMITER {
            if let Some(end) = code_span_end(characters, index).filter(|end| *end > index + 2) {
                push_text(nodes, &std::mem::take(&mut plain), active);
                // Jira allows a code mark beside a link mark only.
                let mut code_marks: Vec<InlineMark> = active
                    .iter()
                    .filter(|mark| mark.kind == MarkKind::Link)
                    .cloned()
                    .collect();
                code_marks.push(InlineMark::plain(MarkKind::Code));
                let code: String = characters[index + 1..end - 1].iter().collect();
                push_text(nodes, &code, &code_marks);
                index = end;
                continue;
            }
        }

        let has_mark = |kind: MarkKind| active.iter().any(|mark| mark.kind == kind);
        if character == '[' && !has_mark(MarkKind::Link) {
            if let Some((label_end, href_end)) = find_link(characters, index) {
                push_text(nodes, &std::mem::take(&mut plain), active);
                let mut marks = active.to_vec();
                marks.push(InlineMark {
                    kind: MarkKind::Link,
                    href: characters[label_end + 2..href_end].iter().collect(),
                });
                parse_span(&characters[index + 1..label_end], &marks, nodes);
                index = href_end + 1;
                continue;
            }
        }

        for kind in MarkKind::DELIMITED {
            let delimiter: Vec<char> = kind.delimiter().chars().collect();
            if has_mark(kind) || !characters[index..].starts_with(&delimiter) {
                continue;
            }
            let content_start = index + delimiter.len();
            let previous = index.checked_sub(1).map(|before| characters[before]);
            let next = characters.get(content_start).copied();
            let opens = match kind {
                MarkKind::Em => can_open_emphasis(previous, next),
                _ => next.is_some_and(|following| !following.is_whitespace()),
            };
            if !opens {
                continue;
            }
            let Some(close) = find_closer(characters, content_start, &delimiter, kind) else {
                continue;
            };
            push_text(nodes, &std::mem::take(&mut plain), active);
            let mut marks = active.to_vec();
            marks.push(InlineMark::plain(kind));
            parse_span(&characters[content_start..close], &marks, nodes);
            index = close + delimiter.len();
            continue 'characters;
        }

        plain.push(character);
        index += 1;
    }

    push_text(nodes, &plain, active);
}

/// Finds the delimiter that closes a mark whose content starts at `start`.
/// Escaped characters and code spans are skipped.
fn find_closer(
    characters: &[char],
    start: usize,
    delimiter: &[char],
    kind: MarkKind,
) -> Option<usize> {
    let mut index = start;
    while index < characters.len() {
        if is_escape_at(characters, index) {
            index += 2;
            continue;
        }
        if characters[index] == CODE_DELIMITER {
            if let Some(end) = code_span_end(characters, index) {
                index = end;
                continue;
            }
        }
        if index > start && characters[index..].starts_with(delimiter) {
            let previous = Some(characters[index - 1]);
            let next = characters.get(index + delimiter.len()).copied();
            let closes = match kind {
                MarkKind::Em => can_close_emphasis(previous, next),
                _ => !characters[index - 1].is_whitespace(),
            };
            if closes {
                return Some(index);
            }
        }
        index += 1;
    }
    None
}

/// For a `[` at `open`, returns where the label's `]` and the target's `)`
/// are, when they form a link the editor writes.
fn find_link(characters: &[char], open: usize) -> Option<(usize, usize)> {
    let mut label_end = open + 1;
    while label_end < characters.len() && characters[label_end] != ']' {
        if is_escape_at(characters, label_end) {
            label_end += 2;
        } else if characters[label_end] == CODE_DELIMITER {
            label_end = code_span_end(characters, label_end).unwrap_or(label_end + 1);
        } else {
            label_end += 1;
        }
    }
    if label_end == open + 1 || characters.get(label_end + 1) != Some(&'(') {
        return None;
    }

    let href_end = find_character(characters, label_end + 2, ')')?;
    let href: String = characters[label_end + 2..href_end].iter().collect();
    let writable = LINK_SCHEMES.iter().any(|scheme| href.starts_with(scheme))
        && !href.chars().any(char::is_whitespace);
    writable.then_some((label_end, href_end))
}

/// Appends text with the given marks, extending the previous node when it
/// carries the same marks.
fn push_text(nodes: &mut Vec<Value>, text: &str, active: &[InlineMark]) {
    if text.is_empty() {
        return;
    }
    let mut ordered = active.to_vec();
    ordered.sort_by_key(|mark| mark.kind);
    let marks: Vec<Value> = ordered.iter().map(InlineMark::to_adf).collect();

    if let Some(previous) = nodes.last_mut() {
        if node_type(previous) == "text" && self::marks(previous) == marks.as_slice() {
            let joined = format!("{}{text}", node_text(previous));
            previous["text"] = Value::String(joined);
            return;
        }
    }

    let mut node = json!({ "type": "text", "text": text });
    if !marks.is_empty() {
        node["marks"] = Value::Array(marks);
    }
    nodes.push(node);
}

fn code_block(language: &str, code: &str) -> Value {
    let mut block = json!({ "type": "codeBlock" });
    if !language.is_empty() {
        block["attrs"] = json!({ "language": language });
    }
    if !code.is_empty() {
        block["content"] = json!([{ "type": "text", "text": code }]);
    }
    block
}

fn end_paragraph(lines: &mut Vec<&str>, blocks: &mut Vec<Value>) {
    if lines.is_empty() {
        return;
    }
    let mut content = Vec::new();
    for (index, line) in lines.drain(..).enumerate() {
        if index > 0 {
            content.push(json!({ "type": "hardBreak" }));
        }
        content.extend(parse_inline(line));
    }
    blocks.push(json!({ "type": "paragraph", "content": content }));
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

/// One list item line. `indent` decides nesting; `number` is the number an
/// ordered item was written with.
struct ListLine<'a> {
    indent: usize,
    kind: ListKind,
    number: u64,
    text: &'a str,
}

const FIRST_LIST_NUMBER: u64 = 1;

fn parse_list_line(line: &str) -> Option<ListLine<'_>> {
    let content = line.trim_start();
    let indent = line[..line.len() - content.len()]
        .chars()
        .map(|character| if character == '\t' { TAB_INDENT } else { 1 })
        .sum();

    if let Some(item) = content
        .strip_prefix("- ")
        .or_else(|| content.strip_prefix("* "))
    {
        return Some(ListLine {
            indent,
            kind: ListKind::Bullet,
            number: FIRST_LIST_NUMBER,
            text: item.trim(),
        });
    }

    let digits = content.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let item = content[digits..]
        .strip_prefix(". ")
        .or_else(|| content[digits..].strip_prefix(") "))?;
    Some(ListLine {
        indent,
        kind: ListKind::Ordered,
        number: content[..digits].parse().unwrap_or(FIRST_LIST_NUMBER),
        text: item.trim(),
    })
}

struct OpenList {
    indent: usize,
    kind: ListKind,
    first_number: u64,
    items: Vec<Value>,
}

impl OpenList {
    fn into_node(self) -> Value {
        let mut node = json!({ "type": self.kind.node_type(), "content": self.items });
        if self.kind == ListKind::Ordered && self.first_number != FIRST_LIST_NUMBER {
            node["attrs"] = json!({ "order": self.first_number });
        }
        node
    }
}

/// Closes the innermost open list into the item that holds it, or into the
/// document when it is a top-level list.
fn close_list(open: &mut Vec<OpenList>, blocks: &mut Vec<Value>) {
    let Some(list) = open.pop() else {
        return;
    };
    let node = list.into_node();
    let parent_item = open
        .last_mut()
        .and_then(|parent| parent.items.last_mut())
        .and_then(|item| item["content"].as_array_mut());
    match parent_item {
        Some(content) => content.push(node),
        None => blocks.push(node),
    }
}

/// Builds lists from consecutive item lines. A line indented further than the
/// item before it starts a list inside that item.
fn build_lists(lines: &[ListLine]) -> Vec<Value> {
    let mut blocks = Vec::new();
    let mut open: Vec<OpenList> = Vec::new();

    for line in lines {
        while open.last().is_some_and(|list| {
            list.indent > line.indent || (list.indent == line.indent && list.kind != line.kind)
        }) {
            close_list(&mut open, &mut blocks);
        }
        if !open.last().is_some_and(|list| list.indent == line.indent) {
            open.push(OpenList {
                indent: line.indent,
                kind: line.kind,
                first_number: line.number,
                items: Vec::new(),
            });
        }
        if let Some(list) = open.last_mut() {
            list.items.push(json!({
                "type": "listItem",
                "content": [{ "type": "paragraph", "content": parse_inline(line.text) }],
            }));
        }
    }

    while !open.is_empty() {
        close_list(&mut open, &mut blocks);
    }
    blocks
}

const CELL_SEPARATOR: char = '|';

fn is_table_row(content: &str) -> bool {
    content.len() >= 2 && content.starts_with(CELL_SEPARATOR) && content.ends_with(CELL_SEPARATOR)
}

/// Splits a `| a | b |` row into trimmed cells. `\|` is a literal pipe.
fn split_table_row(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut characters = row[1..row.len() - 1].chars().peekable();

    while let Some(character) = characters.next() {
        if character == ESCAPE && characters.peek() == Some(&CELL_SEPARATOR) {
            characters.next();
            cell.push(CELL_SEPARATOR);
        } else if character == CELL_SEPARATOR {
            cells.push(std::mem::take(&mut cell).trim().to_string());
        } else {
            cell.push(character);
        }
    }
    cells.push(cell.trim().to_string());
    cells
}

/// True for the `| --- | --- |` row that marks the row above as a header.
fn is_header_separator(cells: &[String]) -> bool {
    cells.iter().all(|cell| {
        cell.contains('-')
            && cell
                .chars()
                .all(|character| character == '-' || character == ':')
    })
}

fn build_table(rows: &[Vec<String>]) -> Value {
    let has_header = rows.len() >= 2 && is_header_separator(&rows[1]);
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let row_node = |cells: &Vec<String>, cell_type: &str| {
        let content: Vec<Value> = (0..columns)
            .map(|column| {
                let text = cells.get(column).map(String::as_str).unwrap_or_default();
                json!({
                    "type": cell_type,
                    "content": [{ "type": "paragraph", "content": parse_inline(text) }],
                })
            })
            .collect();
        json!({ "type": "tableRow", "content": content })
    };

    let mut content = Vec::new();
    let mut body = rows;
    if has_header {
        content.push(row_node(&rows[0], "tableHeader"));
        body = &rows[2..];
    }
    content.extend(body.iter().map(|cells| row_node(cells, "tableCell")));
    json!({ "type": "table", "content": content })
}

/// Reduces a document to what a reader sees, so two documents that differ
/// only in Jira's bookkeeping compare equal.
fn normalized(node: &Value) -> Value {
    let kind = node_type(node);
    let mut result = Map::new();
    result.insert("type".to_string(), json!(kind));

    if kind == "text" {
        result.insert("text".to_string(), json!(node_text(node)));
    }
    if let Some(attrs) = normalized_attrs(kind, node.get("attrs")) {
        result.insert("attrs".to_string(), attrs);
    }
    let mut marks = marks(node).to_vec();
    if !marks.is_empty() {
        marks.sort_by_key(|mark| mark.to_string());
        result.insert("marks".to_string(), Value::Array(marks));
    }

    let mut content: Vec<Value> = children(node).iter().map(normalized).collect();
    match kind {
        "paragraph" | "heading" => content = normalized_inline(content),
        "doc" => content
            .retain(|block| node_type(block) != "paragraph" || block.get("content").is_some()),
        _ => {}
    }
    if !content.is_empty() {
        result.insert("content".to_string(), Value::Array(content));
    }
    Value::Object(result)
}

/// Drops attributes that carry no visible formatting, such as node ids and
/// values equal to Jira's defaults.
fn normalized_attrs(kind: &str, attrs: Option<&Value>) -> Option<Value> {
    let mut attrs = attrs?.as_object()?.clone();
    attrs.retain(|_, value| !value.is_null());
    attrs.remove("localId");

    let mut drop_default = |name: &str, default: Value| {
        if attrs.get(name) == Some(&default) {
            attrs.remove(name);
        }
    };
    match kind {
        "codeBlock" => drop_default("language", json!("")),
        "orderedList" => drop_default("order", json!(FIRST_LIST_NUMBER)),
        "table" => {
            drop_default("isNumberColumnEnabled", json!(false));
            attrs.remove("layout");
            attrs.remove("width");
        }
        "tableCell" | "tableHeader" => {
            drop_default("colspan", json!(1));
            drop_default("rowspan", json!(1));
        }
        _ => {}
    }

    (!attrs.is_empty()).then_some(Value::Object(attrs))
}

/// Joins neighbouring text nodes that carry the same marks, trims each line,
/// and drops empty lines at the end.
fn normalized_inline(nodes: Vec<Value>) -> Vec<Value> {
    let mut lines: Vec<Vec<Value>> = vec![Vec::new()];
    for node in nodes {
        let line = lines.last_mut().expect("at least one line");
        if node_type(&node) == "hardBreak" {
            lines.push(Vec::new());
            continue;
        }
        match line.last_mut() {
            Some(previous)
                if node_type(&node) == "text"
                    && node_type(previous) == "text"
                    && previous.get("marks") == node.get("marks") =>
            {
                let joined = format!("{}{}", node_text(previous), node_text(&node));
                previous["text"] = Value::String(joined);
            }
            _ => line.push(node),
        }
    }

    for line in &mut lines {
        trim_line_edge(line, LineEdge::Start);
        trim_line_edge(line, LineEdge::End);
    }
    while lines.len() > 1 && lines.last().is_some_and(Vec::is_empty) {
        lines.pop();
    }

    let mut content = Vec::new();
    for (index, line) in lines.into_iter().enumerate() {
        if index > 0 {
            content.push(json!({ "type": "hardBreak" }));
        }
        content.extend(line);
    }
    content
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LineEdge {
    Start,
    End,
}

/// Trims whitespace from one end of a line of inline nodes, removing text
/// nodes that become empty.
fn trim_line_edge(line: &mut Vec<Value>, edge: LineEdge) {
    loop {
        let edge_index = match edge {
            LineEdge::Start => 0,
            LineEdge::End => line.len().saturating_sub(1),
        };
        let Some(node) = line
            .get_mut(edge_index)
            .filter(|node| node_type(node) == "text")
        else {
            return;
        };
        let trimmed = match edge {
            LineEdge::Start => node_text(node).trim_start().to_string(),
            LineEdge::End => node_text(node).trim_end().to_string(),
        };
        if !trimmed.is_empty() {
            node["text"] = Value::String(trimmed);
            return;
        }
        line.remove(edge_index);
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

    fn marked(value: &str, marks: Value) -> Value {
        json!({ "type": "text", "text": value, "marks": marks })
    }

    fn link(href: &str) -> Value {
        json!({ "type": "link", "attrs": { "href": href } })
    }

    fn cell(kind: &str, content: Value) -> Value {
        json!({ "type": kind, "content": [paragraph(content)] })
    }

    fn nested_list() -> Value {
        doc(json!([{
            "type": "bulletList",
            "content": [
                {
                    "type": "listItem",
                    "content": [
                        paragraph(json!([text("parent")])),
                        { "type": "orderedList", "content": [list_item("first"), list_item("second")] },
                    ],
                },
                list_item("sibling"),
            ],
        }]))
    }

    #[test]
    fn inline_marks_render_as_markdown_and_build_back() {
        let document = doc(json!([paragraph(json!([
            text("Use "),
            marked("bold", json!([{ "type": "strong" }])),
            text(", "),
            marked("italic", json!([{ "type": "em" }])),
            text(", "),
            marked("gone", json!([{ "type": "strike" }])),
            text(" and "),
            marked("both", json!([{ "type": "strong" }, { "type": "em" }])),
            text(". See "),
            marked("the docs", json!([link("https://example.test/a_b")])),
            text(" or "),
            marked(
                "run()",
                json!([link("https://example.test/run"), { "type": "code" }])
            ),
            text("."),
        ]))]));
        let source = "Use **bold**, _italic_, ~~gone~~ and **_both_**. See [the docs](https://example.test/a_b) or [`run()`](https://example.test/run).";

        assert_eq!(adf_to_text(&document), source);
        assert_eq!(text_to_adf(source), document);
        assert!(adf_round_trips_as_text(&document));
    }

    #[test]
    fn marks_apply_inside_headings_and_list_items() {
        assert_eq!(
            text_to_adf("## A **loud** title\n- an _item_"),
            doc(json!([
                {
                    "type": "heading",
                    "attrs": { "level": 2 },
                    "content": [text("A "), marked("loud", json!([{ "type": "strong" }])), text(" title")],
                },
                {
                    "type": "bulletList",
                    "content": [{
                        "type": "listItem",
                        "content": [paragraph(json!([text("an "), marked("item", json!([{ "type": "em" }]))]))],
                    }],
                },
            ]))
        );
    }

    #[test]
    fn code_spans_keep_their_text_and_drop_other_marks() {
        assert_eq!(
            text_to_adf("**`a_b * **c**`**"),
            doc(json!([paragraph(json!([marked(
                "a_b * **c**",
                json!([{ "type": "code" }])
            )]))]))
        );
    }

    #[test]
    fn markup_characters_in_plain_text_survive_a_round_trip() {
        for literal in [
            "2 * 3 * 4 and snake_case_name stay as typed",
            "a**b**c is not bold and `tick` is not code",
            "_leading and trailing_ underscores, ~~tildes~~ too",
            "a [bracket](https://example.test) that is plain text",
            "C:\\code\\_tmp and a trailing slash\\",
            "an unmatched ** pair, a lone ` tick and one _ underscore",
        ] {
            let document = doc(json!([paragraph(json!([text(literal)]))]));

            assert_eq!(
                text_to_adf(&adf_to_text(&document)),
                document,
                "{literal:?} rendered as {:?}",
                adf_to_text(&document)
            );
            assert!(adf_round_trips_as_text(&document), "{literal:?}");
        }
    }

    #[test]
    fn ordinary_punctuation_is_not_escaped() {
        let document = doc(json!([paragraph(json!([text(
            "2 * 3, snake_case, C:\\code\\notes, [WIP] and a ~ b"
        )]))]));

        assert_eq!(
            adf_to_text(&document),
            "2 * 3, snake_case, C:\\code\\notes, [WIP] and a ~ b"
        );
    }

    #[test]
    fn unmatched_or_unsupported_markup_stays_plain_text() {
        for source in [
            "**never closed",
            "snake_case_name",
            "[label](javascript:alert)",
            "[label] (https://example.test)",
            "** spaced **",
        ] {
            assert_eq!(
                text_to_adf(source),
                doc(json!([paragraph(json!([text(source)]))])),
                "{source:?}"
            );
        }
    }

    #[test]
    fn nested_lists_render_indented_and_build_back() {
        let document = nested_list();
        let source = "- parent\n  1. first\n  2. second\n- sibling";

        assert_eq!(adf_to_text(&document), source);
        assert_eq!(text_to_adf(source), document);
        assert!(adf_round_trips_as_text(&document));
    }

    #[test]
    fn a_list_returns_to_an_outer_level_after_deeper_items() {
        let item_with = |value: &str, child: Value| json!({ "type": "listItem", "content": [paragraph(json!([text(value)])), child] });

        assert_eq!(
            text_to_adf("- a\n    - b\n        - c\n    - d\n- e"),
            doc(json!([{
                "type": "bulletList",
                "content": [
                    item_with("a", json!({
                        "type": "bulletList",
                        "content": [
                            item_with("b", json!({ "type": "bulletList", "content": [list_item("c")] })),
                            list_item("d"),
                        ],
                    })),
                    list_item("e"),
                ],
            }]))
        );
    }

    #[test]
    fn numbered_lists_keep_their_starting_number() {
        let document = doc(json!([{
            "type": "orderedList",
            "attrs": { "order": 4 },
            "content": [list_item("fourth"), list_item("fifth")],
        }]));

        assert_eq!(adf_to_text(&document), "4. fourth\n5. fifth");
        assert_eq!(text_to_adf("4. fourth\n5. fifth"), document);
        assert!(adf_round_trips_as_text(&document));
    }

    #[test]
    fn tables_build_from_pipe_rows() {
        let source = "| Do **this** | Expect |\n| --- | --- |\n| a \\| b |  |";

        let document = doc(json!([{
            "type": "table",
            "content": [
                { "type": "tableRow", "content": [
                    cell("tableHeader", json!([text("Do "), marked("this", json!([{ "type": "strong" }]))])),
                    cell("tableHeader", json!([text("Expect")])),
                ] },
                { "type": "tableRow", "content": [
                    cell("tableCell", json!([text("a | b")])),
                    cell("tableCell", json!([])),
                ] },
            ],
        }]));

        assert_eq!(text_to_adf(source), document);
        assert_eq!(adf_to_text(&document), source);
        assert!(adf_round_trips_as_text(&document));
    }

    #[test]
    fn a_table_without_a_separator_row_has_no_header() {
        assert_eq!(
            text_to_adf("| a | b |\n| c |"),
            doc(json!([{
                "type": "table",
                "content": [
                    { "type": "tableRow", "content": [
                        cell("tableCell", json!([text("a")])),
                        cell("tableCell", json!([text("b")])),
                    ] },
                    { "type": "tableRow", "content": [
                        cell("tableCell", json!([text("c")])),
                        cell("tableCell", json!([])),
                    ] },
                ],
            }]))
        );
    }

    #[test]
    fn jira_bookkeeping_attributes_do_not_make_a_document_lossy() {
        let document = doc(json!([
            {
                "type": "table",
                "attrs": { "isNumberColumnEnabled": false, "layout": "center", "localId": "t-1", "width": 760 },
                "content": [{ "type": "tableRow", "content": [{
                    "type": "tableHeader",
                    "attrs": { "colspan": 1, "rowspan": 1 },
                    "content": [paragraph(json!([text("Only")]))],
                }] }],
            },
            { "type": "orderedList", "attrs": { "order": 1, "localId": "l-1" }, "content": [list_item("one")] },
            { "type": "paragraph", "content": [] },
            paragraph(json!([
                marked("split ", json!([{ "type": "em" }, { "type": "strong" }])),
                marked("bold", json!([{ "type": "strong" }, { "type": "em" }])),
                text(" tail  "),
            ])),
        ]));

        assert!(adf_round_trips_as_text(&document));
    }

    #[test]
    fn formatting_the_editor_cannot_write_is_reported() {
        let mention = doc(json!([paragraph(json!([
            { "type": "mention", "attrs": { "id": "abc", "text": "@Maya" } },
        ]))]));
        let underlined = doc(json!([paragraph(json!([marked(
            "under",
            json!([{ "type": "underline" }])
        )]))]));
        let panel = doc(json!([{
            "type": "panel",
            "attrs": { "panelType": "warning" },
            "content": [paragraph(json!([text("Known gap")]))],
        }]));
        let sized_table = doc(json!([{
            "type": "table",
            "content": [{ "type": "tableRow", "content": [{
                "type": "tableCell",
                "attrs": { "colwidth": [120] },
                "content": [paragraph(json!([text("Sized")]))],
            }] }],
        }]));
        let merged_table = doc(json!([{
            "type": "table",
            "content": [{ "type": "tableRow", "content": [{
                "type": "tableCell",
                "attrs": { "colspan": 2 },
                "content": [paragraph(json!([text("Wide")]))],
            }] }],
        }]));
        let centered = doc(json!([{
            "type": "paragraph",
            "marks": [{ "type": "alignment", "attrs": { "align": "center" } }],
            "content": [text("Centered")],
        }]));
        let two_paragraph_item = doc(json!([{
            "type": "bulletList",
            "content": [{
                "type": "listItem",
                "content": [paragraph(json!([text("one")])), paragraph(json!([text("two")]))],
            }],
        }]));
        let heading_lookalike = doc(json!([paragraph(json!([text("# not a heading")]))]));

        for document in [
            mention,
            underlined,
            panel,
            sized_table,
            merged_table,
            centered,
            two_paragraph_item,
            heading_lookalike,
        ] {
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

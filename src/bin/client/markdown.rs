// Copyright (C) 2026 The pgmoneta community
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::{collections::BTreeMap, sync::OnceLock};

use markdown::{
    ParseOptions,
    mdast::{Image, Link, List, ListItem, Node},
    to_mdast,
};

use syntect::{
    easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet, util::LinesWithEndings,
};

const ANSI_BOLD_ON: &str = "\x1b[1m";
const ANSI_BOLD_OFF: &str = "\x1b[22m";
const ANSI_ITALIC_ON: &str = "\x1b[3m";
const ANSI_ITALIC_OFF: &str = "\x1b[23m";
const ANSI_STRIKETHROUGH_ON: &str = "\x1b[9m";
const ANSI_STRIKETHROUGH_OFF: &str = "\x1b[29m";
const ANSI_FG_CODE: &str = "\x1b[38;5;245m";
const ANSI_FG_LINK: &str = "\x1b[4;34m";
const ANSI_FG_RESET: &str = "\x1b[39;24m";

struct SyntaxHighlightAssets {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

static SYNTAX_HIGHLIGHT_ASSETS: OnceLock<SyntaxHighlightAssets> = OnceLock::new();

fn syntax_highlight_assets() -> &'static SyntaxHighlightAssets {
    SYNTAX_HIGHLIGHT_ASSETS.get_or_init(|| SyntaxHighlightAssets {
        syntax_set: SyntaxSet::load_defaults_newlines(),
        theme_set: ThemeSet::load_defaults(),
    })
}

pub fn render_markdown_for_console(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    match to_mdast(text, &ParseOptions::gfm()) {
        Ok(mut tree) => {
            resolve_reference_links(&mut tree);
            render_markdown_node(&tree)
        }
        Err(_) => text.to_string(),
    }
}

fn render_markdown_node(node: &Node) -> String {
    match node {
        Node::Root(root) => render_block_nodes(&root.children, false),
        Node::Paragraph(paragraph) => render_inline_nodes(&paragraph.children),
        Node::Heading(heading) => render_heading(heading.depth, &heading.children),
        Node::Blockquote(blockquote) => {
            prefix_lines(&render_block_nodes(&blockquote.children, false), "│ ")
        }
        Node::List(list) => render_list(list),
        Node::ListItem(item) => render_list_item(item, "-"),
        Node::Code(code) => render_code_block(code.lang.as_deref(), &code.value),
        Node::ThematicBreak(_) => "-".repeat(40),
        Node::Table(table) => render_table(&table.children),
        Node::Definition(_) => String::new(),
        Node::Break(_) => "\n".to_string(),
        _ => render_inline_node(node),
    }
}

fn render_block_nodes(nodes: &[Node], compact: bool) -> String {
    let separator = if compact { "\n" } else { "\n\n" };

    nodes
        .iter()
        .map(render_markdown_node)
        .filter(|rendered| !rendered.trim().is_empty())
        .collect::<Vec<_>>()
        .join(separator)
}

fn render_inline_nodes(nodes: &[Node]) -> String {
    nodes.iter().map(render_inline_node).collect()
}

fn render_inline_node(node: &Node) -> String {
    match node {
        Node::Text(text) => text.value.clone(),

        Node::Strong(strong) => {
            format!(
                "{ANSI_BOLD_ON}{}{ANSI_BOLD_OFF}",
                render_inline_nodes(&strong.children)
            )
        }

        Node::Emphasis(emphasis) => {
            format!(
                "{ANSI_ITALIC_ON}{}{ANSI_ITALIC_OFF}",
                render_inline_nodes(&emphasis.children)
            )
        }

        Node::Delete(delete) => {
            format!(
                "{ANSI_STRIKETHROUGH_ON}{}{ANSI_STRIKETHROUGH_OFF}",
                render_inline_nodes(&delete.children)
            )
        }

        Node::InlineCode(code) => {
            format!("{ANSI_FG_CODE}{}{ANSI_FG_RESET}", code.value)
        }

        Node::InlineMath(math) => {
            format!("{ANSI_FG_CODE}{}{ANSI_FG_RESET}", math.value)
        }

        Node::Link(link) => render_link(&render_inline_nodes(&link.children), &link.url),

        Node::LinkReference(link) => render_inline_nodes(&link.children),

        Node::Image(image) => {
            format!("[image: {}] ({})", image.alt, image.url)
        }

        Node::ImageReference(image) => {
            format!("[image: {}]", image.alt)
        }

        Node::FootnoteReference(reference) => {
            format!("[^{}]", reference.identifier)
        }

        Node::Break(_) => "\n".to_string(),

        Node::Html(html) => html.value.clone(),
        Node::Math(math) => math.value.clone(),

        Node::MdxFlowExpression(expression) => expression.value.clone(),
        Node::MdxTextExpression(expression) => expression.value.clone(),
        Node::MdxjsEsm(esm) => esm.value.clone(),
        Node::Toml(toml) => toml.value.clone(),
        Node::Yaml(yaml) => yaml.value.clone(),

        _ => render_markdown_node(node),
    }
}

fn render_heading(depth: u8, children: &[Node]) -> String {
    let content = render_inline_nodes(children);

    match depth {
        1 => format!(
            "{ANSI_BOLD_ON}{content}{ANSI_BOLD_OFF}\n{}",
            "━".repeat(visible_line_width(&content).max(12))
        ),
        2 => format!("{ANSI_BOLD_ON}  ▌ {content}{ANSI_BOLD_OFF}"),
        3 => format!("{ANSI_BOLD_ON}  ▪ {content}{ANSI_BOLD_OFF}"),
        _ => format!("{ANSI_BOLD_ON}{content}{ANSI_BOLD_OFF}"),
    }
}

fn render_link(label: &str, url: &str) -> String {
    let shown = if label.is_empty() { url } else { label };

    format!("\x1b]8;;{url}\x1b\\{ANSI_FG_LINK}{shown}{ANSI_FG_RESET}\x1b]8;;\x1b\\")
}

fn render_list(list: &List) -> String {
    let start = list.start.unwrap_or(1);

    list.children
        .iter()
        .enumerate()
        .filter_map(|(index, child)| match child {
            Node::ListItem(item) => {
                let marker = if list.ordered {
                    format!("{}.", start + index as u32)
                } else {
                    "-".to_string()
                };

                Some(render_list_item(item, &marker))
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_list_item(item: &ListItem, marker: &str) -> String {
    let body = render_block_nodes(&item.children, !item.spread);

    let task_marker = match item.checked {
        Some(true) => "[x] ",
        Some(false) => "[ ] ",
        None => "",
    };

    let first_prefix = format!("{marker} {task_marker}");
    let continuation = " ".repeat(first_prefix.len());

    indent_lines(&body, &first_prefix, &continuation)
}

fn render_syntax_highlighted_code(language: &str, value: &str) -> Option<Vec<String>> {
    let assets = syntax_highlight_assets();

    let syntax = assets
        .syntax_set
        .find_syntax_by_token(language)
        .or_else(|| assets.syntax_set.find_syntax_by_extension(language))?;

    let theme = assets.theme_set.themes.get("base16-ocean.dark")?;

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut lines = Vec::new();

    for line in LinesWithEndings::from(value) {
        let ranges = highlighter.highlight_line(line, &assets.syntax_set).ok()?;

        let highlighted = syntect::util::as_24_bit_terminal_escaped(&ranges, false);
        lines.push(highlighted);
    }

    Some(lines)
}

fn render_code_block(language: Option<&str>, value: &str) -> String {
    let opener = match language {
        Some(language) if !language.is_empty() => format!("```{language}"),
        _ => "```".to_string(),
    };

    let mut lines = vec![format!("{ANSI_FG_CODE}{opener}{ANSI_FG_RESET}")];

    if value.is_empty() {
        lines.push(String::new());
    } else if let Some(language) = language {
        if let Some(highlighted) = render_syntax_highlighted_code(language, value) {
            lines.extend(highlighted);
        } else {
            lines.extend(render_plain_code_lines(value));
        }
    } else {
        lines.extend(render_plain_code_lines(value));
    }

    lines.push(format!("{ANSI_FG_CODE}```{ANSI_FG_RESET}"));

    lines.join("\n")
}

fn render_plain_code_lines(value: &str) -> Vec<String> {
    if value.is_empty() {
        return vec![String::new()];
    }

    value
        .lines()
        .map(|line| format!("{ANSI_FG_CODE}{line}{ANSI_FG_RESET}"))
        .collect()
}

fn render_table(rows: &[Node]) -> String {
    let rendered_rows = rows
        .iter()
        .filter_map(|row| match row {
            Node::TableRow(row) => Some(
                row.children
                    .iter()
                    .filter_map(|cell| match cell {
                        Node::TableCell(cell) => Some(render_inline_nodes(&cell.children)),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .collect::<Vec<_>>();

    if rendered_rows.is_empty() {
        return String::new();
    }

    let column_count = rendered_rows.iter().map(Vec::len).max().unwrap_or(0);

    let widths = (0..column_count)
        .map(|column| {
            rendered_rows
                .iter()
                .filter_map(|row| row.get(column))
                .flat_map(|cell| cell.split('\n'))
                .map(visible_line_width)
                .max()
                .unwrap_or(0)
                .max(3)
        })
        .collect::<Vec<_>>();

    let border = |left: &str, join: &str, right: &str| {
        format!(
            "{left}{}{right}",
            widths
                .iter()
                .map(|width| "─".repeat(width + 2))
                .collect::<Vec<_>>()
                .join(join)
        )
    };

    let render_row = |row: &[String], header: bool| {
        let cells = (0..column_count)
            .map(|column| {
                row.get(column)
                    .map(String::as_str)
                    .unwrap_or("")
                    .split('\n')
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let height = cells.iter().map(Vec::len).max().unwrap_or(1);

        (0..height)
            .map(|line| {
                let rendered_cells = cells
                    .iter()
                    .zip(&widths)
                    .map(|(cell_lines, width)| {
                        let cell = cell_lines.get(line).copied().unwrap_or("");
                        let padding = width.saturating_sub(visible_line_width(cell));

                        if header {
                            format!(
                                "{ANSI_BOLD_ON} {cell}{} {ANSI_BOLD_OFF}",
                                " ".repeat(padding)
                            )
                        } else {
                            format!(" {cell}{} ", " ".repeat(padding))
                        }
                    })
                    .collect::<Vec<_>>();

                format!("│{}│", rendered_cells.join("│"))
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let mut output = Vec::new();

    output.push(border("┌", "┬", "┐"));

    if let Some(header) = rendered_rows.first() {
        output.push(render_row(header, true));
    }

    if rendered_rows.len() > 1 {
        output.push(border("├", "┼", "┤"));

        for row in &rendered_rows[1..] {
            output.push(render_row(row, false));
        }
    }

    output.push(border("└", "┴", "┘"));

    output.join("\n")
}

fn prefix_lines(text: &str, prefix: &str) -> String {
    text.lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn indent_lines(text: &str, first_prefix: &str, continuation: &str) -> String {
    text.lines()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                format!("{first_prefix}{line}")
            } else {
                format!("{continuation}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn visible_line_width(text: &str) -> usize {
    let mut width = 0;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if chars.peek() == Some(&']') {
                chars.next();

                while let Some(ch) = chars.next() {
                    if ch == '\x07' {
                        break;
                    }

                    if ch == '\x1b' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            } else if chars.peek() == Some(&'[') {
                chars.next();

                for ch in chars.by_ref() {
                    if ch.is_ascii_alphabetic() {
                        break;
                    }
                }
            }

            continue;
        }

        width += 1;
    }

    width
}

fn resolve_reference_links(tree: &mut Node) {
    let mut definitions = BTreeMap::new();
    collect_link_definitions(tree, &mut definitions);

    if !definitions.is_empty() {
        replace_reference_nodes(tree, &definitions);
    }
}

fn collect_link_definitions(node: &Node, definitions: &mut BTreeMap<String, String>) {
    if let Node::Definition(definition) = node {
        definitions.insert(definition.identifier.clone(), definition.url.clone());
    }

    if let Some(children) = node.children() {
        for child in children {
            collect_link_definitions(child, definitions);
        }
    }
}

fn replace_reference_nodes(node: &mut Node, definitions: &BTreeMap<String, String>) {
    let Some(children) = node.children_mut() else {
        return;
    };

    for child in children {
        let replacement = match child {
            Node::LinkReference(reference) => definitions.get(&reference.identifier).map(|url| {
                Node::Link(Link {
                    children: std::mem::take(&mut reference.children),
                    position: None,
                    url: url.clone(),
                    title: None,
                })
            }),

            Node::ImageReference(reference) => definitions.get(&reference.identifier).map(|url| {
                Node::Image(Image {
                    position: None,
                    alt: std::mem::take(&mut reference.alt),
                    url: url.clone(),
                    title: None,
                })
            }),

            _ => None,
        };

        if let Some(replacement) = replacement {
            *child = replacement;
        }

        replace_reference_nodes(child, definitions);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_markdown_emphasis() {
        let rendered = render_markdown_for_console("**bold** and *italic* and ~~deleted~~");

        assert!(rendered.contains("\x1b[1mbold\x1b[22m"));
        assert!(rendered.contains("\x1b[3mitalic\x1b[23m"));
        assert!(rendered.contains("\x1b[9mdeleted\x1b[29m"));
    }

    #[test]
    fn renders_markdown_blocks_with_spacing() {
        let rendered = render_markdown_for_console("# Title\n\nParagraph\n\n- one\n- two");

        assert!(rendered.contains("Title"));
        assert!(rendered.contains("Paragraph"));
        assert!(rendered.contains("- one\n- two"));
        assert!(rendered.contains("\n\n"));
    }

    #[test]
    fn renders_task_lists() {
        let rendered = render_markdown_for_console("- [x] Done\n- [ ] Pending");

        assert!(rendered.contains("- [x] Done"));
        assert!(rendered.contains("- [ ] Pending"));
    }

    #[test]
    fn renders_code_blocks() {
        let rendered = render_markdown_for_console("```rust\nfn main() {}\n```");

        assert!(rendered.contains("fn"));
        assert!(rendered.contains("main"));
        assert!(rendered.contains("{"));
        assert!(rendered.contains("}"));
        assert!(rendered.contains("\x1b["));
    }
    #[test]
    fn renders_tables() {
        let rendered =
            render_markdown_for_console("| Name | Value |\n| --- | --- |\n| foo | bar |");

        assert!(rendered.contains("Name"));
        assert!(rendered.contains("Value"));
        assert!(rendered.contains("foo"));
        assert!(rendered.contains("bar"));
        assert!(rendered.contains("┌"));
        assert!(rendered.contains("└"));
    }

    #[test]
    fn renders_links() {
        let rendered = render_markdown_for_console("[pgmoneta](https://pgmoneta.github.io/)");

        assert!(rendered.contains("pgmoneta"));
        assert!(rendered.contains("https://pgmoneta.github.io/"));
    }

    #[test]
    fn empty_input_is_unchanged() {
        assert_eq!(render_markdown_for_console(""), "");
    }
    #[test]
    fn removes_markdown_delimiters_from_terminal_output() {
        let rendered =
            render_markdown_for_console("# Title\n\nThis is **bold**, *italic*, and `code`.");

        assert!(rendered.contains("Title"));
        assert!(rendered.contains("bold"));
        assert!(rendered.contains("italic"));
        assert!(rendered.contains("code"));

        assert!(!rendered.contains("**bold**"));
        assert!(!rendered.contains("*italic*"));
        assert!(!rendered.contains("`code`"));
    }

    #[test]
    fn renders_ordered_lists() {
        let rendered = render_markdown_for_console("1. First\n2. Second\n3. Third");

        assert!(rendered.contains("1. First"));
        assert!(rendered.contains("2. Second"));
        assert!(rendered.contains("3. Third"));
    }

    #[test]
    fn renders_blockquotes() {
        let rendered = render_markdown_for_console("> Important information");

        assert!(rendered.contains("│ Important information"));
    }

    #[test]
    fn preserves_visual_spacing_between_blocks() {
        let rendered = render_markdown_for_console(
            "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.",
        );

        assert!(rendered.contains("First paragraph.\n\nSecond paragraph."));
        assert!(rendered.contains("Second paragraph.\n\nThird paragraph."));
    }
    #[test]
    fn resolves_reference_links() {
        let rendered =
            render_markdown_for_console("[pgmoneta][project]\n\n[project]: https://pgmoneta.org/");

        assert!(rendered.contains("pgmoneta"));
        assert!(rendered.contains("https://pgmoneta.org/"));
    }
    #[test]
    fn renders_syntax_highlighted_code_blocks() {
        let rendered =
            render_markdown_for_console("```rust\nfn main() {\n    println!(\"Hello\");\n}\n```");

        assert!(rendered.contains("fn"));
        assert!(rendered.contains("main"));
        assert!(rendered.contains("println!"));
        assert!(rendered.contains("\x1b["));
    }
    #[test]
    fn renders_unknown_language_code_blocks() {
        let rendered = render_markdown_for_console("```unknownlang\nhello world\n```");

        assert!(rendered.contains("hello world"));
        assert!(rendered.contains("```unknownlang"));
    }
}

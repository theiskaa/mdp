use crate::styling::{MdPdfFont, StyleMatch};
use crate::Token;
use genpdf::elements::{Paragraph, UnorderedList};
use genpdf::fonts::{FontData, FontFamily};
use genpdf::style::{Color, Style};
use genpdf::Margins;

#[derive(Debug)]
enum Block {
    Heading(Vec<Token>, usize),
    Paragraph(Vec<Token>),
    List(Vec<Vec<Token>>), // List of list items
    BlockQuote(Vec<Token>),
    HorizontalRule,
    EmptyLine,
}

pub struct Pdf {
    input: Vec<Token>,
}

impl Pdf {
    pub fn new(input: Vec<Token>) -> Self {
        Self { input }
    }

    pub fn render(document: genpdf::Document, file: &str) {
        match document.render_to_file(file) {
            Ok(_) => {
                println!("Successfully saved your PDF to {}", file);
            }
            Err(err) => {
                println!("Failed to save file to {}: {}", file, err);
            }
        }
    }

    pub fn create_document(self, style_match: StyleMatch) -> genpdf::Document {
        let font_family = MdPdfFont::load_font_family(style_match.text.font_family)
            .expect("Failed to load font family");
        let code_font_family = MdPdfFont::load_font_family(style_match.code.font_family)
            .expect("Failed to load code font family");

        let mut doc = genpdf::Document::new(font_family.clone());
        // Set document decorator and margins
        let mut decorator = genpdf::SimplePageDecorator::new();
        decorator.set_margins(Margins::trbl(
            style_match.margins.top,
            style_match.margins.right,
            style_match.margins.bottom,
            style_match.margins.left,
        ));
        doc.set_page_decorator(decorator);
        doc.set_font_size(style_match.text.size);

        // Process tokens into blocks
        let blocks = self.group_tokens(self.input.clone());

        for block in blocks {
            match block {
                Block::Heading(content, level) => {
                    let heading_style = match level {
                        1 => &style_match.heading_1,
                        2 => &style_match.heading_2,
                        3 => &style_match.heading_3,
                        _ => &style_match.text,
                    };
                    let mut style = Style::new().with_font_size(heading_style.size);
                    if heading_style.bold {
                        style = style.bold();
                    }
                    if heading_style.italic {
                        style = style.italic();
                    }
                    if let Some(color) = heading_style.text_color {
                        style = style.with_color(Color::Rgb(color.0, color.1, color.2));
                    }
                    let paragraph = self.process_inline_tokens(content, style, &font_family, &code_font_family, &style_match);
                    doc.push(paragraph);
                    doc.push(genpdf::elements::Break::new(heading_style.after_spacing));
                }
                Block::Paragraph(content) => {
                    let mut style = Style::new().with_font_size(style_match.text.size);
                    if let Some(color) = style_match.text.text_color {
                        style = style.with_color(Color::Rgb(color.0, color.1, color.2));
                    }
                    let paragraph = self.process_inline_tokens(content, style, &font_family, &code_font_family, &style_match);
                    doc.push(paragraph);
                    doc.push(genpdf::elements::Break::new(style_match.text.after_spacing));
                }
                Block::List(items) => {
                    let mut list = UnorderedList::new();
                    for item_tokens in items {
                        let mut style = Style::new().with_font_size(style_match.list_item.size);
                        if let Some(color) = style_match.list_item.text_color {
                            style = style.with_color(Color::Rgb(color.0, color.1, color.2));
                        }
                        let item_paragraph = self.process_inline_tokens(item_tokens, style, &font_family, &code_font_family, &style_match);
                        list.push(item_paragraph);
                    }
                    doc.push(list);
                    doc.push(genpdf::elements::Break::new(style_match.list_item.after_spacing));
                }
                Block::BlockQuote(content) => {
                    let mut style = Style::new().with_font_size(style_match.block_quote.size);
                    if style_match.block_quote.italic {
                        style = style.italic();
                    }
                    if let Some(color) = style_match.block_quote.text_color {
                        style = style.with_color(Color::Rgb(color.0, color.1, color.2));
                    }
                    let paragraph = self.process_inline_tokens(content, style, &font_family, &code_font_family, &style_match);
                    doc.push(paragraph);
                    doc.push(genpdf::elements::Break::new(style_match.block_quote.after_spacing));
                }
                Block::HorizontalRule => {
                    // TODO: implement horizontal rule element.
                    doc.push(genpdf::elements::Break::new(style_match.horizontal_rule.after_spacing));
                }
                Block::EmptyLine => {
                    doc.push(genpdf::elements::Break::new(1.0));
                }
            }
        }

        doc
    }

    // Function to group tokens into blocks
    fn group_tokens(&self, tokens: Vec<Token>) -> Vec<Block> {
        let mut blocks = Vec::new();
        let mut idx = 0;
        let len = tokens.len();
        let mut current_inline_content = Vec::new();
        let mut newline_count = 0;

        while idx < len {
            match &tokens[idx] {
                Token::Heading(content, level) => {
                    if !current_inline_content.is_empty() {
                        blocks.push(Block::Paragraph(current_inline_content.clone()));
                        current_inline_content.clear();
                    }
                    blocks.push(Block::Heading(content.clone(), *level));
                    idx += 1;
                    newline_count = 0;
                }
                Token::ListItem(content) => {
                    if !current_inline_content.is_empty() {
                        blocks.push(Block::Paragraph(current_inline_content.clone()));
                        current_inline_content.clear();
                    }
                    // Start a new list
                    let mut items = Vec::new();
                    items.push(content.clone());
                    idx += 1;

                    // Collect subsequent list items
                    while idx < len {
                        if let Token::ListItem(content) = &tokens[idx] {
                            items.push(content.clone());
                            idx += 1;
                        } else {
                            break;
                        }
                    }

                    blocks.push(Block::List(items));
                    newline_count = 0;
                }
                Token::BlockQuote(content) => {
                    if !current_inline_content.is_empty() {
                        blocks.push(Block::Paragraph(current_inline_content.clone()));
                        current_inline_content.clear();
                    }
                    // Process the blockquote content as inline tokens
                    let content_tokens = vec![Token::Text(content.clone())];
                    blocks.push(Block::BlockQuote(content_tokens));
                    idx += 1;
                    newline_count = 0;
                }
                Token::HorizontalRule => {
                    if !current_inline_content.is_empty() {
                        blocks.push(Block::Paragraph(current_inline_content.clone()));
                        current_inline_content.clear();
                    }
                    blocks.push(Block::HorizontalRule);
                    idx += 1;
                    newline_count = 0;
                }
                Token::Newline => {
                    idx += 1;
                    newline_count += 1;

                    if newline_count >= 2 {
                        if !current_inline_content.is_empty() {
                            blocks.push(Block::Paragraph(current_inline_content.clone()));
                            current_inline_content.clear();
                        }
                        blocks.push(Block::EmptyLine);
                        newline_count = 0;
                    } else {
                        // Single newline within inline content
                        current_inline_content.push(Token::Newline);
                    }
                }
                Token::Text(_)
                | Token::Emphasis { .. }
                | Token::StrongEmphasis(_)
                | Token::Code(_)
                | Token::Link(_, _) => {
                    current_inline_content.push(tokens[idx].clone());
                    idx += 1;
                    newline_count = 0;
                }
                _ => {
                    idx += 1;
                    newline_count = 0;
                }
            }
        }

        // Flush any remaining inline content as a paragraph
        if !current_inline_content.is_empty() {
            blocks.push(Block::Paragraph(current_inline_content));
        }

        blocks
    }

    // Function to process inline tokens into a paragraph
    fn process_inline_tokens(
        &self,
        tokens: Vec<Token>,
        style: Style,
        font_family: &FontFamily<FontData>,
        code_font_family: &FontFamily<FontData>,
        style_match: &StyleMatch,
    ) -> Paragraph {
        let mut paragraph = Paragraph::default();
        self.render_inline_tokens(&mut paragraph, tokens, style, font_family, code_font_family, style_match);
        paragraph
    }

    // Function to render inline tokens within a paragraph
    fn render_inline_tokens(
        &self,
        paragraph: &mut Paragraph,
        tokens: Vec<Token>,
        style: Style,
        font_family: &FontFamily<FontData>,
        code_font_family: &FontFamily<FontData>,
        style_match: &StyleMatch,
    ) {
        for token in tokens {
            match token {
                Token::Text(content) => {
                    paragraph.push_styled(content, style.clone());
                }
                Token::Emphasis { level, content } => {
                    let mut nested_style = style.clone();
                    match level {
                        1 => {
                            nested_style = nested_style.italic();
                        }
                        2 => {
                            nested_style = nested_style.bold();
                        }
                        3 | _ => {
                            nested_style = nested_style.bold().italic();
                        }
                    }
                    self.render_inline_tokens(
                        paragraph,
                        content,
                        nested_style,
                        font_family,
                        code_font_family,
                        style_match,
                    );
                }
                Token::StrongEmphasis(content) => {
                    let nested_style = style.clone().bold();
                    self.render_inline_tokens(
                        paragraph,
                        content,
                        nested_style,
                        font_family,
                        code_font_family,
                        style_match,
                    );
                }
                Token::Link(text, url) => {
                    let mut link_style = style.clone();
                    if let Some(color) = style_match.link.text_color {
                        link_style = link_style.with_color(Color::Rgb(color.0, color.1, color.2));
                    }
                    // TODO: Adding a space after link text to fix spacing between consecutive links.
                    // This workaround should be moved to the lexer level for a proper fix.
                    paragraph.push_link(format!("{} ", text), url, link_style);
                }
                Token::Code(content) => {
                    let mut code_style = style.clone();
                    if let Some(color) = style_match.code.text_color {
                        code_style = code_style.with_color(Color::Rgb(color.0, color.1, color.2));
                    }
                    paragraph.push_styled(content, code_style);
                }
                Token::Newline => {} // DO NOTHING
                _ => {}
            }
        }
    }
}

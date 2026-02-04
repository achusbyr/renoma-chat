use pulldown_cmark::{Event, Options, Parser, Tag};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct MarkdownProps {
    pub content: String,
}

#[function_component(Markdown)]
pub fn markdown(props: &MarkdownProps) -> Html {
    render_markdown(&props.content)
}

fn render_markdown(markdown: &str) -> Html {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(markdown, options);
    let events = parser.collect::<Vec<_>>();
    render_events(&mut events.into_iter())
}

fn render_events<'a>(events: &mut impl Iterator<Item = Event<'a>>) -> Html {
    let mut nodes = Vec::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(tag) => {
                nodes.push(render_tag(tag, events));
            }
            Event::End(_) => {
                break;
            }
            Event::Text(text) => {
                nodes.push(html! { {text.as_ref()} });
            }
            Event::Code(code) => {
                nodes.push(html! { <code>{code.as_ref()}</code> });
            }
            Event::SoftBreak => {
                nodes.push(html! { " " });
            }
            Event::HardBreak => {
                nodes.push(html! { <br/> });
            }
            Event::Rule => {
                nodes.push(html! { <hr/> });
            }
            Event::TaskListMarker(checked) => {
                nodes.push(html! { <input type="checkbox" checked={checked} disabled=true /> });
            }
            _ => {}
        }
    }

    html! { { for nodes } }
}

fn render_tag<'a>(tag: Tag<'a>, events: &mut impl Iterator<Item = Event<'a>>) -> Html {
    let content = render_events(events);
    match tag {
        Tag::Paragraph => html! { <p>{content}</p> },
        Tag::Heading { level, .. } => {
            let tag_name = format!("h{}", level as u8);
            html! { <@{tag_name}>{content}</@> }
        }
        Tag::BlockQuote(_) => html! { <blockquote>{content}</blockquote> },
        Tag::CodeBlock(kind) => {
            let lang = match kind {
                pulldown_cmark::CodeBlockKind::Fenced(lang) => Some(lang.to_string()),
                pulldown_cmark::CodeBlockKind::Indented => None,
            };
            html! {
                <pre>
                    <code class={lang}>{content}</code>
                </pre>
            }
        }
        Tag::List(first_num) => {
            if let Some(num) = first_num {
                html! { <ol start={num.to_string()}>{content}</ol> }
            } else {
                html! { <ul>{content}</ul> }
            }
        }
        Tag::Item => html! { <li>{content}</li> },
        Tag::Table(_) => html! { <table>{content}</table> },
        Tag::TableHead => html! { <thead>{content}</thead> },
        Tag::TableRow => html! { <tr>{content}</tr> },
        Tag::TableCell => html! { <td>{content}</td> },
        Tag::Emphasis => html! { <em>{content}</em> },
        Tag::Strong => html! { <strong>{content}</strong> },
        Tag::Strikethrough => html! { <del>{content}</del> },
        Tag::Link {
            dest_url, title, ..
        } => {
            html! { <a href={dest_url.to_string()} title={title.to_string()} target="_blank" rel="noopener noreferrer">{content}</a> }
        }
        Tag::Image {
            dest_url, title, ..
        } => {
            html! { <img src={dest_url.to_string()} title={title.to_string()} alt="" /> }
        }
        _ => html! { {content} },
    }
}

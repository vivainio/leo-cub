use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use quick_xml::{Reader, events::Event};
use thiserror::Error;

use crate::{Node, NodeId, Outline, Position};

#[derive(Debug, Error)]
pub enum LeoXmlError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("invalid Leo file: {0}")]
    Invalid(String),
}

/// A parsed outline plus the original envelope. Saving replaces only `<vnodes>`
/// and `<tnodes>`, retaining globals, preferences, namespaces and extensions.
#[derive(Clone, Debug)]
pub struct LeoDocument {
    pub outline: Outline,
    original: String,
}

impl LeoDocument {
    /// Create a minimal Leo document containing one root node.
    pub fn new(headline: impl Into<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let id = NodeId(format!("cub.{}.{}", now.as_secs(), now.subsec_nanos()));
        let node = Node {
            id: id.clone(),
            headline: headline.into(),
            body: String::new(),
            vnode_attributes: HashMap::new(),
            tnode_attributes: HashMap::new(),
        };
        Self {
            outline: Outline {
                nodes: HashMap::from([(id.clone(), node)]),
                roots: vec![Position {
                    node: id,
                    children: vec![],
                }],
            },
            original: NEW_DOCUMENT_ENVELOPE.to_owned(),
        }
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, LeoXmlError> {
        Self::parse(&fs::read_to_string(path)?)
    }

    pub fn parse(source: &str) -> Result<Self, LeoXmlError> {
        let mut reader = Reader::from_str(source);
        reader.config_mut().trim_text(false);
        let mut outline = Outline::default();
        let mut stack: Vec<Position> = vec![];
        let mut first_seen = HashSet::new();
        let mut in_vnodes = false;
        let mut in_tnodes = false;
        let mut current_vh: Option<NodeId> = None;
        let mut current_t: Option<NodeId> = None;
        loop {
            match reader.read_event()? {
                Event::Start(e) if e.name().as_ref() == b"vnodes" => in_vnodes = true,
                Event::End(e) if e.name().as_ref() == b"vnodes" => in_vnodes = false,
                Event::Start(e) if e.name().as_ref() == b"tnodes" => in_tnodes = true,
                Event::End(e) if e.name().as_ref() == b"tnodes" => in_tnodes = false,
                Event::Start(e) if in_vnodes && e.name().as_ref() == b"v" => {
                    let attrs = attributes(&e)?;
                    let id = NodeId(
                        attrs
                            .get("t")
                            .cloned()
                            .ok_or_else(|| LeoXmlError::Invalid("<v> without t".into()))?,
                    );
                    let vnode_attributes = attrs.into_iter().filter(|(k, _)| k != "t").collect();
                    if first_seen.insert(id.clone()) {
                        outline.nodes.insert(
                            id.clone(),
                            Node {
                                id: id.clone(),
                                headline: String::new(),
                                body: String::new(),
                                vnode_attributes,
                                tnode_attributes: HashMap::new(),
                            },
                        );
                    }
                    stack.push(Position {
                        node: id,
                        children: vec![],
                    });
                }
                Event::Empty(e) if in_vnodes && e.name().as_ref() == b"v" => {
                    let id = attributes(&e)?;
                    let id = NodeId(
                        id.get("t")
                            .cloned()
                            .ok_or_else(|| LeoXmlError::Invalid("<v> without t".into()))?,
                    );
                    attach(
                        Position {
                            node: id,
                            children: vec![],
                        },
                        &mut stack,
                        &mut outline.roots,
                    );
                }
                Event::End(e) if in_vnodes && e.name().as_ref() == b"v" => {
                    let p = stack
                        .pop()
                        .ok_or_else(|| LeoXmlError::Invalid("unbalanced <v>".into()))?;
                    attach(p, &mut stack, &mut outline.roots);
                }
                Event::Start(e) if in_vnodes && e.name().as_ref() == b"vh" => {
                    current_vh = stack.last().map(|p| p.node.clone())
                }
                Event::End(e) if e.name().as_ref() == b"vh" => current_vh = None,
                Event::Start(e) if in_tnodes && e.name().as_ref() == b"t" => {
                    let attrs = attributes(&e)?;
                    let id = NodeId(
                        attrs
                            .get("tx")
                            .cloned()
                            .ok_or_else(|| LeoXmlError::Invalid("<t> without tx".into()))?,
                    );
                    let node = outline.nodes.entry(id.clone()).or_insert(Node {
                        id: id.clone(),
                        headline: String::new(),
                        body: String::new(),
                        vnode_attributes: HashMap::new(),
                        tnode_attributes: HashMap::new(),
                    });
                    node.tnode_attributes = attrs.into_iter().filter(|(k, _)| k != "tx").collect();
                    current_t = Some(id);
                }
                Event::End(e) if e.name().as_ref() == b"t" => current_t = None,
                Event::Text(e) => {
                    let text = e
                        .decode()
                        .map_err(|e| LeoXmlError::Invalid(e.to_string()))?
                        .into_owned();
                    if let Some(id) = &current_vh
                        && let Some(n) = outline.nodes.get_mut(id)
                    {
                        n.headline.push_str(&text);
                    }
                    if let Some(id) = &current_t
                        && let Some(n) = outline.nodes.get_mut(id)
                    {
                        n.body.push_str(&text);
                    }
                }
                Event::GeneralRef(e) => {
                    let name = e
                        .decode()
                        .map_err(|e| LeoXmlError::Invalid(e.to_string()))?;
                    let text = match name.as_ref() {
                        "amp" => "&".to_owned(),
                        "lt" => "<".to_owned(),
                        "gt" => ">".to_owned(),
                        "quot" => "\"".to_owned(),
                        "apos" => "'".to_owned(),
                        value if value.starts_with("#x") => u32::from_str_radix(&value[2..], 16)
                            .ok()
                            .and_then(char::from_u32)
                            .map(String::from)
                            .ok_or_else(|| {
                                LeoXmlError::Invalid(format!("invalid entity &{value};"))
                            })?,
                        value if value.starts_with('#') => value[1..]
                            .parse::<u32>()
                            .ok()
                            .and_then(char::from_u32)
                            .map(String::from)
                            .ok_or_else(|| {
                                LeoXmlError::Invalid(format!("invalid entity &{value};"))
                            })?,
                        value => {
                            return Err(LeoXmlError::Invalid(format!("unknown entity &{value};")));
                        }
                    };
                    if let Some(id) = &current_vh
                        && let Some(n) = outline.nodes.get_mut(id)
                    {
                        n.headline.push_str(&text);
                    }
                    if let Some(id) = &current_t
                        && let Some(n) = outline.nodes.get_mut(id)
                    {
                        n.body.push_str(&text);
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }
        if !stack.is_empty() {
            return Err(LeoXmlError::Invalid("unbalanced vnodes".into()));
        }
        Ok(Self {
            outline,
            original: source.to_owned(),
        })
    }

    pub fn to_xml(&self) -> Result<String, LeoXmlError> {
        let vnodes = render_vnodes(&self.outline);
        let tnodes = render_tnodes(&self.outline);
        let result = replace_section(&self.original, "vnodes", &vnodes)?;
        replace_section(&result, "tnodes", &tnodes)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), LeoXmlError> {
        let path = path.as_ref();
        let tmp = path.with_extension("leo.tmp");
        fs::write(&tmp, self.to_xml()?)?;
        fs::rename(tmp, path)?;
        Ok(())
    }

    /// Save a newly created document without replacing an existing file.
    pub fn save_new(&self, path: impl AsRef<Path>) -> Result<(), LeoXmlError> {
        let xml = self.to_xml()?;
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(xml.as_bytes())?;
        Ok(())
    }
}

const NEW_DOCUMENT_ENVELOPE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<!-- Created by cub: https://github.com/vivainio/leo-cub -->
<leo_file xmlns:leo="https://leo-editor.github.io/leo-editor/namespaces/leo-python-editor/1.1">
<leo_header file_format="2"/>
<globals/>
<preferences/>
<find_panel_settings/>
<vnodes></vnodes>
<tnodes></tnodes>
</leo_file>
"#;

fn attributes(
    e: &quick_xml::events::BytesStart<'_>,
) -> Result<HashMap<String, String>, LeoXmlError> {
    let mut map = HashMap::new();
    for a in e.attributes() {
        let a = a.map_err(|e| LeoXmlError::Invalid(e.to_string()))?;
        #[allow(deprecated)]
        let value = a
            .unescape_value()
            .map_err(|e| LeoXmlError::Invalid(e.to_string()))?
            .into_owned();
        map.insert(String::from_utf8_lossy(a.key.as_ref()).into_owned(), value);
    }
    Ok(map)
}

fn attach(p: Position, stack: &mut [Position], roots: &mut Vec<Position>) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(p)
    } else {
        roots.push(p)
    }
}

fn replace_section(source: &str, tag: &str, replacement: &str) -> Result<String, LeoXmlError> {
    let start_tag = format!("<{tag}>");
    let end_tag = format!("</{tag}>");
    let start = source
        .find(&start_tag)
        .ok_or_else(|| LeoXmlError::Invalid(format!("missing {start_tag}")))?;
    let end = source[start..]
        .find(&end_tag)
        .map(|n| start + n + end_tag.len())
        .ok_or_else(|| LeoXmlError::Invalid(format!("missing {end_tag}")))?;
    Ok(format!(
        "{}{}{}",
        &source[..start],
        replacement,
        &source[end..]
    ))
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_vnodes(outline: &Outline) -> String {
    fn position(
        p: &Position,
        outline: &Outline,
        seen: &mut HashSet<NodeId>,
        out: &mut String,
        depth: usize,
    ) {
        let pad = "  ".repeat(depth);
        if !seen.insert(p.node.clone()) {
            out.push_str(&format!("{pad}<v t=\"{}\"></v>\n", esc(&p.node.0)));
            return;
        }
        let n = &outline.nodes[&p.node];
        let attrs: String = n
            .vnode_attributes
            .iter()
            .map(|(k, v)| format!(" {}=\"{}\"", k, esc(v)))
            .collect();
        out.push_str(&format!(
            "{pad}<v t=\"{}\"{attrs}><vh>{}</vh>",
            esc(&p.node.0),
            esc(&n.headline)
        ));
        if p.children.is_empty() {
            out.push_str("</v>\n");
        } else {
            out.push('\n');
            for c in &p.children {
                position(c, outline, seen, out, depth + 1);
            }
            out.push_str(&format!("{pad}</v>\n"));
        }
    }
    let mut out = "<vnodes>\n".to_owned();
    let mut seen = HashSet::new();
    for p in &outline.roots {
        position(p, outline, &mut seen, &mut out, 1);
    }
    out.push_str("</vnodes>");
    out
}

fn render_tnodes(outline: &Outline) -> String {
    let mut ids: Vec<_> = outline.nodes.keys().collect();
    ids.sort();
    let mut out = "<tnodes>\n".to_owned();
    for id in ids {
        let n = &outline.nodes[id];
        // Leo omits <t> entirely for a node with an empty body and no attributes;
        // the parser already treats a missing <t> as an empty body, so preserve
        // that on write to avoid materializing hundreds of empty tags on re-save.
        if n.body.is_empty() && n.tnode_attributes.is_empty() {
            continue;
        }
        let attrs: String = n
            .tnode_attributes
            .iter()
            .map(|(k, v)| format!(" {}=\"{}\"", k, esc(v)))
            .collect();
        out.push_str(&format!(
            "  <t tx=\"{}\"{attrs}>{}</t>\n",
            esc(&id.0),
            esc(&n.body)
        ));
    }
    out.push_str("</tnodes>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_document_is_valid_and_round_trips() {
        let document = LeoDocument::new("Project & notes");
        assert!(document.outline.validate().is_empty());

        let reparsed = LeoDocument::parse(&document.to_xml().unwrap()).unwrap();
        assert_eq!(reparsed.outline, document.outline);
        assert_eq!(reparsed.outline.roots.len(), 1);
        let root = &reparsed.outline.roots[0].node;
        assert_eq!(reparsed.outline.nodes[root].headline, "Project & notes");
    }

    #[test]
    fn save_new_does_not_overwrite() {
        let path = std::env::temp_dir().join(format!(
            "cub-new-test-{}-{}.leo",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let document = LeoDocument::new("First");
        document.save_new(&path).unwrap();
        assert!(LeoDocument::new("Second").save_new(&path).is_err());
        assert_eq!(LeoDocument::open(&path).unwrap().outline, document.outline);
        fs::remove_file(path).unwrap();
    }
}

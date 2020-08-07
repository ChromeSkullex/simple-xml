//! XML parser and writer
//! This crate can load xml from a file or string and parse it into memory
//! XML can also be manipulated or created and the written to file
//! ## Loading xml from a file
//! ```
//! fn load_message() -> Result<(), simple_xml::Error> {
//!     let root = simple_xml::from_file("examples/message.xml")?;
//!     // Since there can multiple nodes/tags with the same name, we need to index twice
//!     let heading = &root["heading"][0];
//!     println!("Heading: {}", heading.content);
//!     // Access attributes
//!     let lang = root.get_attribute("lang").expect("Missing lang attribute");
//!     println!("Language: {}", lang);
//!     Ok(())
//! }
//!
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::{fmt, ops};

mod split_unquoted;
use split_unquoted::SplitUnquoted;

#[derive(Debug)]
pub struct Node {
    pub tag: String,
    pub attributes: HashMap<String, String>,
    nodes: HashMap<String, Vec<Node>>,
    pub content: String,
}

struct Payload<'a> {
    prolog: &'a str,
    node: Option<Node>,
    remaining: &'a str,
}

#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    ContentOutsideRoot(usize),
    MissingClosingTag(String, usize),
    MissingClosingDelimiter(usize),
    MissingAttributeValue(String, usize),
    MissingQuotes(String, usize),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IOError(e)
    }
}

fn validate_root(root: Result<Payload, Error>) -> Result<Node, Error> {
    match root {
        Ok(v) if v.prolog.len() != 0 => Err(Error::ContentOutsideRoot(999)),
        Ok(v) => Ok(v.node.unwrap_or(Node {
            tag: String::new(),
            content: String::new(),
            nodes: HashMap::new(),
            attributes: HashMap::new(),
        })),
        Err(e) => Err(e),
    }
}

/// Loads an xml structure from a file and returns appropriate errors
pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Node, Error> {
    validate_root(load_from_slice(&std::fs::read_to_string(path)?))
}

/// Loads an xml structure from a string and returns appropriate errors
pub fn from_string(string: &str) -> Result<Node, Error> {
    validate_root(load_from_slice(string))
}

/// Creates a new empty node
/// Nodes and attributes can be added later
/// Content is taken owned as to avoid large copy
/// Tag is not taken owned as it is most often a string literal
pub fn new(tag: &str, content: String) -> Node {
    Node {
        attributes: HashMap::new(),
        content,
        tag: tag.to_owned(),
        nodes: HashMap::new(),
    }
}

/// Creates a new node with given tag, attributes content, and child nodes
pub fn new_filled(
    tag: &str,
    attributes: HashMap<String, String>,
    content: String,
    nodes: HashMap<String, Vec<Node>>,
) -> Node {
    Node {
        tag: tag.to_owned(),
        attributes,
        nodes,
        content,
    }
}

/// Loads a xml structure from a slice
/// Ok variant contains a payload with the child node, name prolog, and remaining stringtuple with (prolog, tag_name, tag_data, remaining_from_in)
fn load_from_slice(string: &str) -> Result<Payload, Error> {
    let opening_del = match string.find("<") {
        Some(v) => v,
        None => {
            return Ok(Payload {
                prolog: "",
                node: None,
                remaining: string,
            });
        }
    };

    let closing_del = match string.find(">") {
        Some(v) => v,
        None => return Err(Error::MissingClosingDelimiter(999)),
    };

    let mut tag_parts = SplitUnquoted::split(&string[opening_del + 1..closing_del], ' ');

    let tag_name = tag_parts.next().unwrap().trim();

    // Collect the prolog as everything before opening tag excluding whitespace
    let prolog = string[..opening_del].trim();

    // Is a comment
    // Attempt to read past comment
    if &tag_name[0..1] == "?" {
        return load_from_slice(&string[closing_del + 1..]);
    }

    let mut attributes = HashMap::new();
    for part in tag_parts {
        // Last closing of empty node
        if part == "/" {
            break;
        }

        let equal_sign = match part.find("=") {
            Some(v) => v,
            None => return Err(Error::MissingAttributeValue(part.to_owned(), 999)),
        };

        // Get key and value from attribute
        let (k, v) = part.split_at(equal_sign);

        // Remove quotes from value
        let v = if &v[1..2] == "\"" && &v[v.len() - 1..] == "\"" {
            &v[2..v.len() - 1]
        } else {
            return Err(Error::MissingQuotes(part.to_owned(), 999));
        };
        attributes.insert(k.to_owned(), v.to_owned());
    }

    // Empty but valid node
    if string[opening_del + 1..closing_del].ends_with("/") {
        return Ok(Payload {
            prolog,
            node: Some(Node {
                tag: tag_name.to_owned(),
                nodes: HashMap::new(),
                attributes: attributes,
                content: String::new(),
            }),
            remaining: &string[closing_del + 1..],
        });
    }

    // Find the closing tag index
    let closing_tag = match string.find(&format!("</{}>", tag_name)) {
        Some(v) => v,
        None => return Err(Error::MissingClosingTag(tag_name.to_owned(), 999)),
    };

    let mut content = String::with_capacity(512);
    let mut nodes = HashMap::new();

    // Load the inside contents and nodes
    let mut buf = &string[closing_del + 1..closing_tag];

    while buf.len() != 0 {
        let payload = load_from_slice(buf)?;

        if let Some(node) = payload.node {
            let v = nodes
                .entry(node.tag.clone())
                .or_insert(Vec::with_capacity(1));
            v.push(node);
        }

        // Nothing was read by node, no more nodes
        if payload.remaining.as_ptr() == buf.as_ptr() {
            break;
        }

        // Put what was before the next tag into the content of the parent tag
        content.push_str(&payload.prolog);
        buf = payload.remaining;
    }

    // Add the remaining inside content to content after no more nodes where found
    content.push_str(buf);

    let remaining = &string[closing_tag + tag_name.len() + 3..];

    Ok(Payload {
        prolog,
        node: Some(Node {
            tag: tag_name.to_owned(),
            attributes,
            nodes,
            content: content.trim().into(),
        }),
        remaining,
    })
}

impl Node {
    /// Returns a list of all node nodes with the specified tag
    /// If no nodes with the specified tag exists, None is returned
    pub fn get_nodes(&self, tag: &str) -> Option<&Vec<Node>> {
        self.nodes.get(tag)
    }

    /// Adds or updates an attribute
    /// If an attribute with that key already exists it is returned
    pub fn add_attribute(&mut self, key: &str, val: &str) -> Option<String> {
        self.attributes.insert(key.to_owned(), val.to_owned())
    }

    pub fn get_attribute(&self, key: &str) -> Option<&String> {
        self.attributes.get(key)
    }

    /// Inserts a new node node with the name of the node field
    pub fn add_node(&mut self, node: Node) {
        let v = self
            .nodes
            .entry(node.tag.clone())
            .or_insert(Vec::with_capacity(1));
        v.push(node);
    }

    // Converts an xml structure to a string with whitespace formatting
    pub fn to_string_pretty(&self) -> String {
        fn internal(node: &Node, depth: usize) -> String {
            if node.tag == "" {
                return "".to_owned();
            }

            match node.nodes.len() + node.content.len() {
                0 => format!(
                    "{indent}<{}{}/>\n",
                    node.tag,
                    node.attributes
                        .iter()
                        .map(|(k, v)| format!(" {}=\"{}\"", k, v))
                        .collect::<String>(),
                    indent = " ".repeat(depth * 4)
                ),
                _ => format!(
                    "{indent}<{tag}{attr}>{beg}{nodes}{content}{end}</{tag}>\n",
                    tag = node.tag,
                    attr = node
                        .attributes
                        .iter()
                        .map(|(k, v)| format!(" {}=\"{}\"", k, v))
                        .collect::<String>(),
                    nodes = node
                        .nodes
                        .iter()
                        .flat_map(|(_, nodes)| nodes.iter())
                        .map(|node| internal(node, depth + 1))
                        .collect::<String>(),
                    beg = match node.nodes.len() {
                        0 => "",
                        _ => "\n",
                    },
                    end = match node.nodes.len() {
                        0 => "".to_owned(),
                        _ => " ".repeat(depth * 4),
                    },
                    content = node.content,
                    indent = " ".repeat(depth * 4),
                ),
            }
        }
        internal(&self, 0)
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        if self.tag == "" {
            return write!(f, "");
        }

        match self.nodes.len() + self.content.len() {
            0 => write!(
                f,
                "<{}{}/>",
                self.tag,
                self.attributes
                    .iter()
                    .map(|(k, v)| format!(" {}=\"{}\"", k, v))
                    .collect::<String>(),
            ),
            _ => write!(
                f,
                "<{tag}{attr}>{nodes}{content}</{tag}>",
                tag = self.tag,
                attr = self
                    .attributes
                    .iter()
                    .map(|(k, v)| format!(" {}=\"{}\"", k, v))
                    .collect::<String>(),
                nodes = self
                    .nodes
                    .iter()
                    .flat_map(|(_, nodes)| nodes.iter())
                    .map(|node| node.to_string())
                    .collect::<String>(),
                content = self.content,
            ),
        }
    }
}

/// Returns a slice of all node nodes with the specified tag
/// If no nodes with the specified tag exists, an empty slice is returned
impl ops::Index<&str> for Node {
    type Output = [Node];
    fn index(&self, tag: &str) -> &Self::Output {
        match self.nodes.get(tag) {
            Some(v) => &v[..],
            None => &[],
        }
    }
}

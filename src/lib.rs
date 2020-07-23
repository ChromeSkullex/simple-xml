use std::collections::HashMap;
use std::path::Path;

mod split_unquoted;
use split_unquoted::SplitUnquoted;

#[derive(Debug)]
pub struct XMLNode {
    attributes: HashMap<String, String>,
    children: HashMap<String, XMLNode>,
    content: String,
}

struct Payload<'a> {
    prolog: &'a str,
    name: &'a str,
    node: Option<XMLNode>,
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

fn validate_root(root: Result<Payload, Error>) -> Result<XMLNode, Error> {
    match root {
        Ok(v) if v.prolog.len() != 0 => Err(Error::ContentOutsideRoot(999)),
        Ok(v) => Ok(v.node.unwrap_or(XMLNode {
            content: String::new(),
            children: HashMap::new(),
            attributes: HashMap::new(),
        })),
        Err(e) => Err(e),
    }
}

pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<XMLNode, Error> {
    validate_root(load_from_slice(&std::fs::read_to_string(path)?))
}

pub fn load_from_string(string: &str) -> Result<XMLNode, Error> {
    validate_root(load_from_slice(string))
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
                name: "",
            });
        }
    };

    let closing_del = match string.find(">") {
        Some(v) => v,
        None => return Err(Error::MissingClosingDelimiter(999)),
    };

    let mut tag_parts = SplitUnquoted::split(&string[opening_del + 1..closing_del], ' ');

    let tag_name = tag_parts.next().unwrap();

    // Is a comment
    // Attempt to read past comment
    if &tag_name[0..1] == "?" {
        return load_from_slice(&string[closing_del + 1..]);
    }

    let mut attributes = HashMap::new();
    for part in tag_parts {
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

    // Collect the prolog as everything before opening tag exlcluding whitespace

    let prolog = string[..opening_del].trim();
    // Find the closing tag index
    let closing_tag = match string.find(&format!("</{}>", tag_name)) {
        Some(v) => v,
        None => return Err(Error::MissingClosingTag(tag_name.to_owned(), 999)),
    };

    let mut content = String::with_capacity(512);
    let mut children = HashMap::new();

    // Load the inside contents and children
    let mut buf = &string[closing_del + 1..closing_tag];

    // println!("Buf: {}", buf);
    while buf.len() != 0 {
        let payload = load_from_slice(buf)?;

        if let Some(child) = payload.node {
            children.insert(payload.name.to_owned(), child);
        }

        // Nothing was read by child, no more nodes
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
        name: tag_name,
        node: Some(XMLNode {
            attributes,
            children,
            content: content.trim().into(),
        }),
        remaining,
    })
}
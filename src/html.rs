use std::default::Default;
use html5ever::{parse_document};
use html5ever::driver::ParseOpts;
use html5ever::rcdom::{RcDom, NodeData, Handle, Node};
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use std::cell::{RefCell, Ref};
use std::collections::HashSet;
use std::option::Option;
use std::rc::Weak;
use std::fmt;

static VOID_ELEMENTS: [&'static str; 23] = [
    "area",
    "base",
    "basefont",
    "bgsound",
    "br",
    "col",
    "command",
    "embed",
    "frame",
    "hr",
    "image",
    "img",
    "input",
    "isindex",
    "keygen",
    "link",
    "menuitem",
    "meta",
    "nextid",
    "param",
    "source",
    "track",
    "wbr" ];

thread_local!(
    static VOID_ELMS_HASH: RefCell<HashSet<String>> = RefCell::new(
        VOID_ELEMENTS.iter().cloned().map(|x| String::from(x)).collect()
    );
);

fn is_void_element(name: &str) -> bool {
    VOID_ELMS_HASH.with(|hash| {
        let h: Ref<HashSet<_>> = hash.borrow();
        h.contains(&String::from(name).to_lowercase())
    })
}

fn walk(prefix: &str, handle: Handle, mut buffer: &mut String) {
    let node = handle;

    match *node.clone() {
        Node {data: NodeData::Document, .. } => buffer.push_str(&format!("<!DOCTYPE html>\n")),

        Node {data: NodeData::Text { ref contents, .. },  ref parent, .. }  => {
            let taken_parent: Option<Weak<_>> = parent.take();

            let parent_name = taken_parent.as_ref().
                and_then(|weak_ptr|weak_ptr.upgrade()).
                and_then(|x| match &*x {
                    Node {data: NodeData::Element {ref name, ..}, ..} =>
                        Some(String::from(&*name.local).to_lowercase()),
                    _ => None
            }).unwrap_or(String::new());

            parent.set(taken_parent);

            if contents.borrow().trim().len() > 0 {
                if parent_name == "script" || parent_name == "style" {
                    buffer.push_str(&format!("{}{}\n", prefix, &contents.borrow()));
                } else {
                    buffer.push_str(&format!("{}{}\n", prefix, EscapeText(&contents.borrow())));
                }
            }
        },

        Node {data: NodeData::Element { ref name, ref attrs, .. }, ..} => {
            buffer.push_str(&format!("{}<{}", prefix, name.local));

            for attr in attrs.borrow().iter() {
                let attr_name: &str = &attr.name.local;
                let attr_value: &str = &attr.value;
                buffer.push_str(&format!(" {}=\"{}\"", attr_name, EscapeAttr(attr_value)));
            }

            if is_void_element(&name.local) {
                buffer.push_str(&format!("/>\n"));
            } else {
                buffer.push_str(&format!(">\n"));
            }

        },

        _ => {},
    }

    let new_indent = {
        let mut temp = String::new();
        temp.push_str(prefix);
        match *node.clone() {
            Node {data: NodeData::Document, .. } => (),
            _ => temp.push_str("    "),
        };
        temp
    };

    for child in node.children.borrow().iter()
        .filter(|child| match child.data {
            NodeData::Text { .. } | NodeData::Element { .. } => true,
            _ => false,
        }
        ) {
        walk(&new_indent, child.clone(), &mut buffer);
    }

    match node.data {
        NodeData::Element { ref name, .. } => {
            if !is_void_element(&name.local) {
                buffer.push_str(&format!("{}</{}>\n", prefix, name.local));
            }
        },

        _ => {},
    }
}

pub struct EscapeAttr<'a>(pub &'a str);

impl<'a> fmt::Display for EscapeAttr<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let EscapeAttr(s) = *self;
        let pile_o_bits = s;
        let mut last = 0;
        for (i, ch) in s.bytes().enumerate() {
            match ch as char {
                '<' | '>' | '&' | '\'' | '"' => {
                    fmt.write_str(&pile_o_bits[last.. i])?;
                    let s = match ch as char {
                        '>' => "&gt;",
                        '<' => "&lt;",
                        '&' => "&amp;",
                        '\'' => "&#39;",
                        '"' => "&quot;",
                        _ => unreachable!()
                    };
                    fmt.write_str(s)?;
                    last = i + 1;
                }
                _ => {}
            }
        }

        if last < s.len() {
            fmt.write_str(&pile_o_bits[last..])?;
        }
        Ok(())
    }
}

pub struct EscapeText<'a>(pub &'a str);

impl<'a> fmt::Display for EscapeText<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let EscapeText(s) = *self;
        let pile_o_bits = s;
        let mut last = 0;
        for (i, ch) in s.bytes().enumerate() {
            match ch as char {
                '<' | '>' | '&' => {
                    fmt.write_str(&pile_o_bits[last.. i])?;
                    let s = match ch as char {
                        '>' => "&gt;",
                        '<' => "&lt;",
                        '&' => "&amp;",
                        _ => unreachable!()
                    };
                    fmt.write_str(s)?;
                    last = i + 1;
                }
                _ => {}
            }
        }

        if last < s.len() {
            fmt.write_str(&pile_o_bits[last..])?;
        }
        Ok(())
    }
}

pub fn beautify_html(html: &str) -> String {
    let opts = ParseOpts {
        tree_builder: TreeBuilderOpts {
            drop_doctype: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let dom = parse_document(RcDom::default(), opts)
        .from_utf8()
        .one(html.as_bytes());

    let mut result = String::new();
    walk("", dom.document, &mut result);

    result
}
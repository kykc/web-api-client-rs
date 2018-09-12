use reqwest::header::{Headers, ContentType};
use reqwest::mime::{Mime, TEXT_PLAIN, APPLICATION, JSON, TEXT, XML, HTML};
use static_map;
use syntax_highlight::{CONTENT_TYPE_DEFAULT, CONTENT_TYPE_JSON, CONTENT_TYPE_XML, CONTENT_TYPE_HTML};
use serde_json;
use libxml;
use libxml::bindings::xmlKeepBlanksDefault;

static KNOWN_HEADERS: static_map::Map<&'static str, bool> = static_map! {
    Default: false,
    "cookie" => true,
    "accept-encoding" => true,
    "content-type" => true,
};

pub fn populate_headers(text: &str) -> Headers {
    let mut headers = Headers::new();

    for line in text.lines() {
            let tokens = line.split(":").collect::<Vec<&str>>();
            let entry = KNOWN_HEADERS.get_entry(String::from(tokens[0]).to_lowercase().as_str());

            match entry {
                Some(e) => headers.append_raw(*e.0, String::from(tokens[1]).into_bytes()),
                None => () // TODO: show warning about unsupported header
            }
        }

    headers
}

pub fn detect_mime_type(headers: &Headers) -> Mime {
    match headers.get::<ContentType>() {
        Some(x) => x.0.clone(),
        None => TEXT_PLAIN
    }
}

pub fn conv_mime_type_to_extension(mime: &Mime) -> &'static str {
    match (mime.type_(), mime.subtype()) {
        (APPLICATION, JSON) | (TEXT, JSON) => {
            CONTENT_TYPE_JSON
        },
        (APPLICATION, XML) | (TEXT, XML) => {
            CONTENT_TYPE_XML
        },
        (TEXT, HTML) => {
            CONTENT_TYPE_HTML
        }
        _ => CONTENT_TYPE_DEFAULT
    }
}

pub fn beautify_response_text(extension: &'static str, text: &str) -> String {
    match extension {
        CONTENT_TYPE_JSON => {
            let json_result: serde_json::Result<serde_json::Value> = serde_json::from_str(&text);
            match json_result {
                Ok(json) => serde_json::ser::to_string_pretty(&json).expect("Cannot stringify JSON"),
                Err(_) => text.to_owned()
            }
        },
        CONTENT_TYPE_XML => {
            unsafe {xmlKeepBlanksDefault(0); }
            let parser = libxml::parser::Parser::default();
            match parser.parse_string(&text) {
                Ok(doc) => doc.to_string(true),
                Err(_) => text.to_owned()
            }
        },
        CONTENT_TYPE_HTML => {
            let parser = libxml::parser::Parser::default_html();
            match parser.parse_string(&text) {
                Ok(doc) => if parser.is_well_formed_html(text) { doc.to_string(true) } else { text.to_owned() },
                Err(_) => text.to_owned()
            }
        }
        _ => text.to_owned()
    }
}
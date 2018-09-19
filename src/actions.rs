use reqwest::header::{Headers, ContentType};
use reqwest::mime::{Mime, TEXT_PLAIN, APPLICATION, JSON, TEXT, XML, HTML};
use serde_json;
use libxml;
use libxml::bindings::xmlKeepBlanksDefault;
use gtk;
use gtk::{TextViewExt, Cast};
use gtk_ext;
use gtk_ext::{TextWidget};
use sourceview::{BufferExt, LanguageManagerExt};
use sourceview;
use std;
use hyper;
use reqwest;
use std::error::{Error};
use glib;

pub const CONTENT_TYPE_JSON: &'static str = "json";
pub const CONTENT_TYPE_DEFAULT: &'static str = "";
pub const CONTENT_TYPE_XML: &'static str = "xml";
pub const CONTENT_TYPE_HTML: &'static str = "html";

pub fn populate_headers<T: gtk::prelude::IsA<gtk::Window>>(text: &str, win: &T) -> Headers {
    let mut headers = Headers::new();

    for line in text.lines() {
        let tokens = line.split(":").collect::<Vec<&str>>();
            
        match tokens.len() {
            2 => {
                let name = String::from(tokens[0]);
                headers.append_raw(name, String::from(tokens[1]).into_bytes());
            },
            _ => {
                let msg = String::from("Invalid header omitted: ") + line;
                gtk_ext::show_message(&msg, win);
            }
        };
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

pub fn output_to_sourceview(target: &::MainWindow, resp: &::Response) {
    let highlight_override = resp.highlight.as_ref().map(String::as_str);
    let mime_str = resp.mime_type.to_string();
    let text = beautify_response_text(resp.extension, &resp.text);
    let extension = match highlight_override {Some(x) => x, _ => resp.extension};
    let content_type = match highlight_override {Some(_) => None, _ => Some(mime_str.as_str())};

    target.lang_manager.
        guess_language(Some((String::from("dummy.") + extension).as_str()), content_type).
        map(|lang| target.resp_mtx.get_buffer().unwrap().downcast_ref::<sourceview::Buffer>().unwrap().set_language(&lang));
    
    target.resp_mtx.replace_all_text(&text);
}

pub fn create_post_req_data<'a>(text: &'a str) -> Vec<(&'a str, &'a str)> {
    let mut form = Vec::new();

    for line in text.lines() {
        let tokens = line.splitn(2,'=');                    
        let mut key_value: (&str, &str) = ("", "");

        for (i, item) in tokens.enumerate() {
            match i {
                0 => key_value.0 = item,
                1 => key_value.1 = item,
                _ => panic!("should never happen")
            };
        }

        form.push(key_value);
    }

    form
}

pub fn http_worker(
    request_method: ::RequestMethod, 
    url: &str, 
    req: String, 
    highlight_override: Option<String>,
    headers: hyper::Headers,
    tx: std::sync::mpsc::Sender<std::result::Result<::Response, std::string::String>>)
{
    let client = reqwest::Client::new();
            
    let parsed_uri: Result<hyper::Uri, String> = url.parse::<hyper::Uri>().or_else(|e| Err(e.to_string()));

    let req_error_to_string = |err: reqwest::Error| Err(String::from("Request failed: ") + err.description());

    let request_result = parsed_uri.and_then(|_| {
        let sent = match request_method {
            ::RequestMethod::GetWithUri => {
                client.get(url).headers(headers).send()
            },
            ::RequestMethod::PostWithForm => {
                client.post(url).headers(headers).form(create_post_req_data(&req).as_slice()).send()
            },
            ::RequestMethod::PostRaw => {
                client.post(url).headers(headers).body(req).send()
            }
        };

        sent.or_else(req_error_to_string)
    });

    let result = request_result.map(|mut x| {
        ::Response::from(&mut x).with_highlight_override(highlight_override)
    });

    tx.send(result).unwrap();
    
    glib::idle_add(::receive);
}
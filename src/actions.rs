use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use mime::{Mime, TEXT_PLAIN, APPLICATION, JSON, TEXT, XML, HTML};
use serde_json;
use gtk;
use gtk_ext;
use gtk_ext::{TextWidget};
use sourceview::{BufferExt, LanguageManagerExt};
use std;
use reqwest;
use std::error::{Error};
use glib;
use ::xml;
use ::html;

pub const CONTENT_TYPE_JSON: &'static str = "json";
pub const CONTENT_TYPE_DEFAULT: &'static str = "";
pub const CONTENT_TYPE_XML: &'static str = "xml";
pub const CONTENT_TYPE_HTML: &'static str = "html";

pub fn to_pair_if_both<T, U>(t: Option<T>, u: Option<U>) -> Option<(T, U)> {
    match (t, u) {
        (Some(x), Some(y)) => Some((x, y)),
        _ => None
    }
}

pub fn populate_headers<T: gtk::prelude::IsA<gtk::Window>>(text: &str, win: &T) -> HeaderMap {
    let mut headers = HeaderMap::new();

    for line in text.lines() {
        let pair = line.find(":").map(|x| line.split_at(x));

        let parsed_pair = pair.and_then(|x| {
            let name = HeaderName::from_bytes(x.0.as_bytes()).ok();
            let val = x.1.trim_left_matches(':').parse::<HeaderValue>().ok();

            to_pair_if_both(name, val)
        });

        match parsed_pair {
            Some(p) => { headers.append(p.0, p.1); },
            None => { gtk_ext::show_message(&(String::from("Failed to parse header - ") + line), win); }
        };
    }

    headers
}

pub fn detect_mime_type(headers: &HeaderMap) -> Mime {
    headers.get("content-type").
        and_then(|x| x.to_str().ok()).
        and_then(|x| x.parse::<Mime>().ok()).unwrap_or(TEXT_PLAIN)
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
            xml::beautify_xml(&text).unwrap_or(text.to_string())
        },
        CONTENT_TYPE_HTML => {
            html::beautify_html(&text)
        },
        _ => text.to_owned()
    }
}

pub fn output_to_sourceview(target: &::MainWindow, resp: &::Response) {
    let highlight_override = resp.highlight.as_ref().map(String::as_str);
    let mime_str = resp.mime_type.to_string();
    let text = beautify_response_text(resp.extension, &resp.text);
    let extension = match highlight_override {Some(x) => x, _ => resp.extension};
    let content_type = match highlight_override {Some(_) => None, _ => Some(mime_str.as_str())};

    target.resp_mtx.replace_all_text(&text);

    let mut headers_text = String::new();

    for ref header in &resp.headers {
        headers_text += header.0.as_str();
        headers_text += ": ";
        headers_text += header.1.to_str().unwrap();
        headers_text += "\n";
    }

    target.resp_headers_mtx.replace_all_text(&headers_text);

    ::CONFIG.with(|conf| {
        let mut state = conf.borrow_mut();
        state.current_extension = Some(String::from(extension));
        state.current_mime = content_type.map(|x| String::from(x));
    });

    update_resp_body_highlighting(target);
}

pub fn update_resp_body_highlighting(target: &::MainWindow) {
    ::CONFIG.with(|conf| {
        let state = conf.borrow();

        let extension = state.current_extension.as_ref().map(|x| x.as_str()).unwrap_or("text/plain");
        let mime_str = state.current_mime.as_ref().map(|x| x.as_str());

        target.lang_manager.
            guess_language(Some((String::from("dummy.") + extension).as_str()), mime_str).
            map(|lang| gtk_ext::apply_to_src_buf(&target.resp_mtx, &|x| x.set_language(&lang)));
    });
}

pub fn create_post_req_data(text: &str) -> Vec<(&str, &str)> {
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
    headers: HeaderMap,
    tx: std::sync::mpsc::Sender<std::result::Result<::Response, std::string::String>>)
{
    let client = reqwest::Client::new();
            
    let req_error_to_string = |err: reqwest::Error| Err(String::from("Request failed: ") + err.description());

    let request_result = match request_method {
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

    let result = request_result.or_else(req_error_to_string).map(|mut x| {
        ::Response::from(&mut x).with_highlight_override(highlight_override)
    });

    tx.send(result).unwrap();
    
    glib::idle_add(::receive);
}
extern crate gio;
extern crate gtk;
extern crate syntect;
extern crate serde_json;
extern crate reqwest;
extern crate gdk;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::{Builder, Button, Entry, ApplicationWindow, TextView};

use std::env::args;

use reqwest::mime::{Mime, APPLICATION, JSON};

use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style, Color};
use syntect::util::as_24_bit_terminal_escaped;


// make moving clones into closures more convenient
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

fn from_parser_to_gdk(c: Color) -> gdk::RGBA {
    let mut result = gdk::RGBA::red();
    let norm = 255f64;
    result.alpha = (c.a as f64) / norm;
    result.blue = (c.b as f64) / norm;
    result.red = (c.r as f64) / norm;
    result.green = (c.g as f64) / norm;

    result
}

fn conv_to_tag(buffer: &mut gtk::TextBuffer, style: Style) -> String {
    let hash: String = serde_json::to_string(&style).unwrap();
    let table = buffer.get_tag_table().unwrap();
    
    if !table.lookup(&hash).is_some() {
        
        let tag = gtk::TextTag::new(Some(hash.as_str()));
        tag.set_property_foreground_rgba(Some(&from_parser_to_gdk(style.foreground)));
        tag.set_property_background_rgba(Some(&from_parser_to_gdk(style.background)));
        table.add(&tag);
    }

    hash
}

pub fn build_ui(application: &gtk::Application) {
    let glade_src = include_str!("main.glade");
    let builder = Builder::new_from_string(glade_src);
    let json_def = syntect::parsing::syntax_definition::SyntaxDefinition::load_from_str(include_str!("json.sublime-syntax"), true, None).unwrap();
    let mut ps = SyntaxSet::new();
    ps.add_syntax(json_def);
    let ts = ThemeSet::load_defaults();
    ps.link_syntaxes();
    let window: ApplicationWindow = builder.get_object("window1").expect("Couldn't get window1");

    window.set_application(application);
    window.connect_delete_event(clone!(window => move |_, _| {
        window.destroy();
        Inhibit(false)
    }));

    let perform_btn: Button = builder.get_object("performBtn").expect("performBtn not found");
    let url_inp: Entry = builder.get_object("urlInp").expect("urlInp not found");
    let resp_mtx: TextView = builder.get_object("respMtx").expect("respMtx not found");

    // TODO: take from color theme
    let back_color = gdk::RGBA::black();
    #[allow(deprecated)]
    resp_mtx.override_background_color(gtk::StateFlags::NORMAL, Some(&back_color));
    let front_color = gdk::RGBA::white();
    #[allow(deprecated)]
    resp_mtx.override_color(gtk::StateFlags::NORMAL, Some(&front_color));

    perform_btn.connect_clicked(clone!(resp_mtx, url_inp => move |_| {
        
        let mut cookie = reqwest::header::Cookie::new();
        cookie.append("PHPSESSID", "sg51fc0t3na1p7bhq0f8ns6up2");
        let client = reqwest::Client::new();
        let result = client.get(&url_inp.get_text().unwrap()).header(cookie).send();
        match result {

            Ok(mut x) => {
                let mime: Mime = x.headers().get::<reqwest::header::ContentType>().unwrap().0.clone();
                match (mime.type_(), mime.subtype()) {
                    (APPLICATION, JSON) => {
                        let json: serde_json::Value = serde_json::from_str(&x.text().unwrap()).unwrap();
                        let json_syntax = ps.find_syntax_by_extension("json").unwrap();
                        let mut h = HighlightLines::new(json_syntax, &ts.themes["base16-ocean.dark"]);
                        let json_string = serde_json::ser::to_string_pretty(&json).unwrap();
                        resp_mtx.get_buffer().unwrap().delete(&mut resp_mtx.get_buffer().unwrap().get_start_iter(), &mut resp_mtx.get_buffer().unwrap().get_end_iter());
                        for line in json_string.lines() {
                            let ranges: Vec<(Style, &str)> = h.highlight(line);

                            for range in &ranges {
                                let mut end = resp_mtx.get_buffer().unwrap().get_end_iter();
                                resp_mtx.get_buffer().unwrap().insert(&mut end, range.1);
                                let tag_name = conv_to_tag(&mut resp_mtx.get_buffer().unwrap(), range.0);
                                let mut end1 = resp_mtx.get_buffer().unwrap().get_end_iter();
                                let mut start1 = end1.clone();
                                start1.backward_chars(range.1.len() as i32);
                                resp_mtx.get_buffer().unwrap().apply_tag_by_name(&tag_name, &start1, &end1);
                                //let tag = resp_mtx.get_buffer().unwrap().get_tag_table().unwrap().lookup(&tag_name).unwrap();
                            }
                            resp_mtx.get_buffer().unwrap().insert(&mut resp_mtx.get_buffer().unwrap().get_end_iter(), "\n");
                            let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
                            println!("{}", escaped);
                        }

                        //resp_mtx.get_buffer().expect("resp_mtx has no buffer").set_text(&serde_json::ser::to_string_pretty(&json).unwrap());
                        //let iter1 = resp_mtx.get_buffer().unwrap().get_iter_at_line_offset(2, 0);
                        //let iter2 = resp_mtx.get_buffer().unwrap().get_iter_at_line_offset(4, 0);
                        //resp_mtx.get_buffer().unwrap().apply_tag_by_name("sntx_comment", &iter1, &iter2);
                    },
                    _ => resp_mtx.get_buffer().expect("resp_mtx has no buffer").set_text(&x.text().unwrap())
                }
            },
            Err(_y) => println!("Request failed"),
        }
    }));

    window.show_all();
}

pub fn main() {
    let application = gtk::Application::new("com.github.builder_basics",
                                            gio::ApplicationFlags::empty())
        .expect("Initialization failed...");

    application.connect_startup(move |app| {
        build_ui(app);
    });
    application.connect_activate(|_| {});

    application.run(&args().collect::<Vec<_>>());
}


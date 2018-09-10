extern crate gio;
extern crate gtk;
extern crate syntect;
extern crate serde_json;
extern crate reqwest;
extern crate gdk;
#[macro_use]
extern crate static_map;
#[macro_use]
extern crate static_map_macros;
#[macro_use]
extern crate serde_derive;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::{Builder, Button, Entry, ApplicationWindow, TextView};

use std::env::args;
use std::fs::File;
use std::error::Error;
use std::io::Write;
use std::io::Read;
use std::path::Path;

use reqwest::mime::{Mime, APPLICATION, JSON};

use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style, Color};

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

trait TextWidget {
    fn get_all_text(&self) -> String;
    fn replace_all_text(&self, &str);
    fn clear_all_text(&self);
    fn append_text(&self, &str);
}

trait TextWidgetStyled {
    fn append_styled_text(&self, &str, &Style);
}

impl TextWidgetStyled for gtk::TextView {
    fn append_styled_text(&self, text: &str, style: &Style) {
        match self.get_buffer() {
            Some(buf) => {
                self.append_text(text);
                let tag_name = conv_to_tag(&buf, style);
                let mut end = buf.get_end_iter();
                let mut start = end.clone();
                start.backward_chars(text.len() as i32);
                buf.apply_tag_by_name(&tag_name, &start, &end);
            },
            _ => ()
        }
    }
}

impl TextWidget for gtk::Entry {
    fn get_all_text(&self) -> String {
        self.get_text().unwrap_or(String::from(""))
    }

    fn replace_all_text(&self, new_text: &str) {
        self.set_text(new_text);
    }

    fn clear_all_text(&self) {
        self.set_text("");
    }

    fn append_text(&self, add_text: &str) {
        let new_text = self.get_all_text() + add_text;
        self.replace_all_text(&new_text);
    }
}

impl TextWidget for gtk::TextView {
    fn get_all_text(&self) -> String {
        match self.get_buffer() {
            Some(buf) => buf.get_text(&mut buf.get_start_iter(), &mut buf.get_end_iter(), true).unwrap_or(String::from("")),
            _ => String::from("")
        }
    }

    fn replace_all_text(&self, new_text: &str) {
        match self.get_buffer() {
            Some(buf) => {
                buf.delete(&mut buf.get_start_iter(), &mut buf.get_end_iter());
                buf.insert(&mut buf.get_start_iter(), new_text);
            },
            _ => ()
        }
    }

    fn clear_all_text(&self) {
        match self.get_buffer() {
            Some(buf) => {
                buf.delete(&mut buf.get_start_iter(), &mut buf.get_end_iter());
            },
            _ => ()
        }
    }

    fn append_text(&self, add_text: &str) {
        match self.get_buffer() {
            Some(buf) => {
                buf.insert(&mut buf.get_end_iter(), add_text);
            },
            _ => ()
        }
    }
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

fn conv_to_tag(buffer: &gtk::TextBuffer, style: &Style) -> String {
    let hash: String = serde_json::to_string(&style).expect("Cannot serialize style to JSON string");
    let table = buffer.get_tag_table().unwrap();
    
    if !table.lookup(&hash).is_some() {
        
        let tag = gtk::TextTag::new(Some(hash.as_str()));
        tag.set_property_foreground_rgba(Some(&from_parser_to_gdk(style.foreground)));
        tag.set_property_background_rgba(Some(&from_parser_to_gdk(style.background)));
        table.add(&tag);
    }

    hash
}

use static_map::Map;

#[derive(Serialize, Deserialize)]
struct Config {
    pub width: u32,
    pub height: u32,
    pub url: String,
    pub headers: String,
}

struct DisplayJobPlainText;

struct DisplayJobJson<'a> {
    syntaxes: &'a syntect::parsing::SyntaxSet,
    themes: &'a syntect::highlighting::ThemeSet,
    json: serde_json::Value,
}

trait DisplayJobExt {
    fn perform(&self, &TextView, &str);
}

impl DisplayJobExt for DisplayJobPlainText {
    fn perform(&self, target: &TextView, raw_text: &str) {
        target.replace_all_text(raw_text);
    }
}

impl<'a> DisplayJobExt for DisplayJobJson<'a> {
    fn perform(&self, target: &TextView, _raw_text: &str) {
        let json_syntax = self.syntaxes.find_syntax_by_extension("json").expect("Cannot find JSON syntax highlighter");
        let mut h = HighlightLines::new(json_syntax, &self.themes.themes["base16-ocean.dark"]);
        let json_string = serde_json::ser::to_string_pretty(&self.json).expect("Cannot stringify JSON");
        target.clear_all_text();
        for line in json_string.lines() {
            let ranges: Vec<(Style, &str)> = h.highlight(line);
            for range in &ranges {
                target.append_styled_text(range.1, &range.0);
            }
            target.append_text("\n");
        }
    }
}

static KNOWN_HEADERS: Map<&'static str, bool> = static_map! {
    Default: false,
    "cookie" => true,
    "accept-encoding" => true,
};

fn write_config(config: &Config) {
    let config_path = get_config_path();

    let j = serde_json::to_string_pretty(&config).unwrap();

    let display = config_path.display();

    let mut file = match std::fs::File::create(&config_path) {
        Err(why) => panic!("couldn't create {}: {}",
                           display,
                           why.description()),
        Ok(file) => file,
    };

    match file.write_all(j.as_bytes()) {
        Err(why) => {
            panic!("couldn't write to {}: {}", display,
                                               why.description())
        },
        Ok(_) => (),
    }
}

fn get_config_path() -> std::path::PathBuf {
    let executable_path: std::path::PathBuf = std::env::current_exe().expect("Cannot get executable path");
    let config_path: std::path::PathBuf = executable_path.with_file_name("config.json");

    config_path
}

fn get_current_confg() -> Config {
    let mut default_config = Config {height: (600u32), width: (1366u32), url: "https://api.github.com/users/kykc/repos".to_string(), headers: "".to_string()};
    let config_path = get_config_path();

    if Path::new(config_path.to_str().unwrap()).exists() {
        let mut f = File::open(config_path.clone()).expect("Config file not found");

        let mut contents = String::new();

        f.read_to_string(&mut contents).expect("something went wrong reading config the file");

        default_config = serde_json::from_str(&contents).unwrap();
    }

    write_config(&default_config);

    default_config
}

fn show_message<T: gtk::prelude::IsA<gtk::Window>>(msg: &str, window: &T) {
    let dialog = gtk::MessageDialog::new(Some(window), gtk::DialogFlags::MODAL, gtk::MessageType::Warning, gtk::ButtonsType::Ok, msg);
    dialog.connect_response(|dialog, _| dialog.destroy());
    dialog.run();
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
    let config = get_current_confg();
    let perform_btn: Button = builder.get_object("performBtn").expect("performBtn not found");
    let url_inp: Entry = builder.get_object("urlInp").expect("urlInp not found");
    let resp_mtx: TextView = builder.get_object("respMtx").expect("respMtx not found");
    let headers_mtx: TextView = builder.get_object("headersMtx").expect("headersMtx not found");
    let mut window_rect = window.get_allocation();
    window_rect.width = config.width as i32;
    window_rect.height = config.height as i32;
    window.set_allocation(&window_rect);
    url_inp.set_text(&config.url);
    headers_mtx.replace_all_text(&config.headers);

    window.set_application(application);
    window.connect_delete_event(clone!(window, url_inp, headers_mtx => move |_, _| {
        let headers_buffer = headers_mtx.get_buffer().unwrap();
        let new_config = Config {
            url: url_inp.get_text().unwrap(), 
            height: window.get_allocated_height() as u32, 
            width: window.get_allocated_width() as u32,
            headers: headers_buffer.get_text(&headers_buffer.get_start_iter(), &headers_buffer.get_end_iter(), true).unwrap(),
        };

        write_config(&new_config);
        window.destroy();
        Inhibit(false)
    }));

    // TODO: take from color theme programmatically
    let front_color = gdk::RGBA::white();
    let back_color = gdk::RGBA{
        red:0.16862745098039217,
        green:0.18823529411764706,
        blue: 0.23137254901960785, 
        alpha: 1f64
    };
    
    #[allow(deprecated)]
    resp_mtx.override_background_color(gtk::StateFlags::NORMAL, Some(&back_color));
    #[allow(deprecated)]
    resp_mtx.override_color(gtk::StateFlags::NORMAL, Some(&front_color));
    #[allow(deprecated)]
    resp_mtx.override_background_color(gtk::StateFlags::SELECTED, Some(&gdk::RGBA::black()));

    perform_btn.connect_clicked(clone!(resp_mtx, url_inp, window => move |_| {
        let mut headers = reqwest::header::Headers::new();
        for line in headers_mtx.get_all_text().lines() {
            let tokens = line.clone().split(":").collect::<Vec<&str>>();
            let entry = KNOWN_HEADERS.get_entry(String::from(tokens[0]).to_lowercase().as_str());

            match entry {
                Some(e) => headers.append_raw(*e.0, String::from(tokens[1]).into_bytes()),
                None => ()
            }
        }
        
        let client = reqwest::Client::new();
        let result = client.get(&url_inp.get_all_text()).headers(headers).send();
        match result {
            Ok(mut x) => {
                let response_text = x.text().unwrap_or(String::from(""));
                // TODO: assume text/plain if no header is supplied
                let mime: Mime = x.headers().get::<reqwest::header::ContentType>().unwrap().0.clone();
                let job: Box<DisplayJobExt> = match (mime.type_(), mime.subtype()) {
                    (APPLICATION, JSON) => {
                        match serde_json::from_str(&response_text) {
                            Ok(json) => Box::new(DisplayJobJson{syntaxes: &ps, themes: &ts, json: json}),
                            Err(_) => Box::new(DisplayJobPlainText{})
                        }
                    },
                    _ => Box::new(DisplayJobPlainText{})
                };

                job.perform(&resp_mtx, &response_text);
            },
            Err(err) => show_message(&(String::from("Request failed: ") + err.description()), &window),
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


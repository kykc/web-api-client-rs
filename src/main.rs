extern crate gio;
extern crate gtk;
extern crate syntect;
extern crate serde_json;
extern crate reqwest;
extern crate gdk;
extern crate libxml;
#[macro_use] extern crate static_map;
#[macro_use] extern crate static_map_macros;
#[macro_use] extern crate serde_derive;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::{Builder, Button, Entry, ApplicationWindow, TextView, Cast, StyleContextExt, ComboBoxText, MenuItemExt};

use std::env::args;
use std::error::Error;
use reqwest::mime::{Mime};

use syntect::highlighting::{ThemeSet};
use text_out::{TextWidget};

mod syntax_highlight;
mod config;
mod text_out;
#[macro_use] mod gtk_ext;
mod actions;

#[derive(Clone)]
struct MainWindow {
    pub window: ApplicationWindow,
    pub perform_btn: Button,
    pub url_inp: Entry,
    pub resp_mtx: TextView,
    pub headers_mtx: TextView,
    pub method_sel: ComboBoxText,
    pub req_mtx: TextView,
}

enum RequestMethod {
    GetWithUri = 1,
    PostWithForm = 2,
    PostRaw = 3,
}

impl MainWindow {
    fn apply_css(window: &ApplicationWindow) {
        let css = gtk::CssProvider::new();
        match css.load_from_data(include_str!("main.css").as_bytes()) {
            Ok(_) => (),
            Err(x) => panic!(x)
        };

        let cont: &gtk::Container = window.upcast_ref();
        gtk_ext::traverse_gtk_container(cont, &|x| x.get_style_context().unwrap().add_provider(&css, 600));
    }

    fn get_request_method(&self) -> RequestMethod {
        match self.method_sel.get_active_id().unwrap_or("".to_owned()).parse::<i32>().unwrap_or(1) {
            2 => RequestMethod::PostWithForm,
            3 => RequestMethod::PostRaw,
            _ => RequestMethod::GetWithUri
        }
    }

    fn new(glade: &str, config: &config::Config, application: &gtk::Application) -> MainWindow {
        let builder = Builder::new_from_string(glade);

        let window: ApplicationWindow = builder.get_object("window1").expect("Couldn't get window1");
        let perform_btn: Button = builder.get_object("performBtn").expect("performBtn not found");
        let url_inp: Entry = builder.get_object("urlInp").expect("urlInp not found");
        let resp_mtx: TextView = builder.get_object("respMtx").expect("respMtx not found");
        let req_mtx: TextView = builder.get_object("reqMtx").expect("reqMtx not found");
        let headers_mtx: TextView = builder.get_object("headersMtx").expect("headersMtx not found");
        let method_sel: ComboBoxText = builder.get_object("methodSel").expect("methodSel not found");
        let mut window_rect = window.get_allocation();
        let search_bar: gtk::SearchBar = builder.get_object("searchBar").expect("searchBar not found");
        let search_inp: gtk::SearchEntry = builder.get_object("searchInp").expect("searchInp not found");
        let find_acm: gtk::ImageMenuItem = builder.get_object("findAcm").expect("findAcm not found");

        MainWindow::apply_css(&window);

        window_rect.width = config.width as i32;
        window_rect.height = config.height as i32;
        window.set_allocation(&window_rect);
        url_inp.set_text(&config.url);
        headers_mtx.replace_all_text(&config.headers);
        
        window.set_application(application);

        find_acm.connect_activate(gtk_clone!(search_bar => move |_| {
            search_bar.set_search_mode(!search_bar.get_search_mode());
        }));

        search_inp.connect_next_match(gtk_clone!(resp_mtx => move |search_inp| {
            let pattern = search_inp.upcast_ref::<gtk::Entry>().get_all_text();
        }));

        MainWindow {
            window: window, 
            perform_btn: perform_btn, 
            url_inp: url_inp, 
            resp_mtx: resp_mtx, 
            headers_mtx: headers_mtx,
            method_sel: method_sel,
            req_mtx: req_mtx
        }
    }
}

pub fn build_ui(application: &gtk::Application) {
    let config = config::get_current_config();
    let m_win = MainWindow::new(include_str!("main.glade"), &config, application);
    
    let ps = syntax_highlight::create_syntax_set();
    let ts = ThemeSet::load_defaults();
    
    m_win.window.connect_delete_event(gtk_clone!(m_win => move |_, _| {
        let new_config = config::Config {
            url: m_win.url_inp.get_all_text(), 
            height: m_win.window.get_allocated_height() as u32, 
            width: m_win.window.get_allocated_width() as u32,
            headers: m_win.headers_mtx.get_all_text(),
        };

        config::write_config(&new_config);
        m_win.window.destroy();
        Inhibit(false)
    }));

    m_win.perform_btn.connect_clicked(gtk_clone!(m_win => move |_| {
        let client = reqwest::Client::new();
        let headers = actions::populate_headers(&m_win.headers_mtx.get_all_text());
        
        let result = match m_win.get_request_method() {
            RequestMethod::GetWithUri => {
                client.get(&m_win.url_inp.get_all_text()).headers(headers).send()
            },
            RequestMethod::PostWithForm => {
                let text = m_win.req_mtx.get_all_text();
                
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
                
                client.post(&m_win.url_inp.get_all_text()).headers(headers).form(form.as_slice()).send()
            },
            RequestMethod::PostRaw => {
                client.post(&m_win.url_inp.get_all_text()).headers(headers).body(m_win.req_mtx.get_all_text()).send()
            }
        };
 
        match result {
            Ok(mut x) => {
                let response_text = x.text().unwrap_or(String::from(""));
                let mime: Mime = actions::detect_mime_type(x.headers());
                let extension: &'static str = actions::conv_mime_type_to_extension(&mime);
                syntax_highlight::output_with_syntax_highlight(
                    &m_win.resp_mtx, 
                    &actions::beautify_response_text(extension, &response_text), 
                    extension, &ps, &ts);
            },
            Err(err) => gtk_ext::show_message(&(String::from("Request failed: ") + err.description()), &m_win.window),
        }
    }));

    m_win.window.show_all();
}

pub fn main() {
    let application = gtk::Application::new("com.automatl.web_api_client", gio::ApplicationFlags::empty())
        .expect("Initialization failed...");

    application.connect_startup(move |app| {
        build_ui(app);
    });
    
    application.connect_activate(|_| {});
    application.run(&args().collect::<Vec<_>>());
}


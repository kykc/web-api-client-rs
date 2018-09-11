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
use gtk::{Builder, Button, Entry, ApplicationWindow, TextView};

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
    pub headers_mtx: TextView
}

/*struct AuWidget<'a, T: gtk::WidgetExt + 'a> {
    pub widget: &'a T
}

trait MaybeContainer {
    fn get_children_or_nothing(&self) -> Option<Vec<gtk::Widget>>;
}

impl<T: gtk::prelude::IsA<gtk::Widget>> MaybeContainer for T {
    fn get_children_or_nothing(&self) -> Option<Vec<gtk::Widget>> {
        None
    }
}

impl<T: gtk::prelude::IsA<gtk::Container> + gtk::ContainerExt - gtk::prelude::IsA<gtk::Widget>> MaybeContainer for T {
    fn get_children_or_nothing(&self) -> Option<Vec<gtk::Widget>> {
        Some(self.get_children())
    }
}

fn traverse_gtk_container<T: gtk::ContainerExt + gtk::WidgetExt, F: Fn(&AuWidget<T>)>(container: &T, worker: &F) {
    worker(&AuWidget{widget: container});
    for widget in container.get_children() {
        
    }
}*/

impl MainWindow {
    fn new(glade: &str, config: &config::Config, application: &gtk::Application) -> MainWindow {
        let builder = Builder::new_from_string(glade);

        let window: ApplicationWindow = builder.get_object("window1").expect("Couldn't get window1");
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

        let css = gtk::CssProvider::new();
        match css.load_from_data(include_str!("main.css").as_bytes()) {
            Ok(_) => (),
            Err(x) => panic!(x)
        };

        // TODO: will be nice if this could be applied to the whole window
        resp_mtx.get_style_context().unwrap().add_provider(&css, 600);

        MainWindow {window: window, perform_btn: perform_btn, url_inp: url_inp, resp_mtx: resp_mtx, headers_mtx: headers_mtx}
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
        let result = client.get(&m_win.url_inp.get_all_text()).headers(headers).send();

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


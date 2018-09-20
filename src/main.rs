#![windows_subsystem = "windows"]

extern crate gio;
extern crate gtk;
extern crate serde_json;
extern crate reqwest;
extern crate gdk;
extern crate libxml;
extern crate sourceview;
extern crate dirs;
extern crate glib;
extern crate mime;
#[macro_use] extern crate serde_derive;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::{Builder, Button, Entry, ApplicationWindow, Cast, StyleContextExt, ComboBoxText, MenuItemExt, SettingsExt};
use std::sync::mpsc::{channel, Receiver};
use std::cell::RefCell;

use std::env::args;
use mime::{Mime};


use gtk_ext::{TextWidget};
use sourceview::{StyleSchemeManagerExt, BufferExt, LanguageManagerExt};

mod config;
#[macro_use] mod gtk_ext;
mod actions;

#[derive(Clone)]
pub struct MainWindow {
    pub window: ApplicationWindow,
    pub perform_btn: Button,
    pub url_inp: Entry,
    pub resp_mtx: sourceview::View,
    pub headers_mtx: sourceview::View,
    pub method_sel: ComboBoxText,
    pub req_mtx: sourceview::View,
    pub lang_manager: sourceview::LanguageManager,
}

pub struct Response
{
    pub text: String,
    pub mime_type: Mime,
    pub extension: &'static str,
    pub highlight: Option<String>,
}

pub enum RequestMethod {
    GetWithUri = 1,
    PostWithForm = 2,
    PostRaw = 3,
}

impl<'a> From<&'a mut reqwest::Response> for Response {
    fn from(x: &'a mut reqwest::Response) -> Self {
        let response_text: String = x.text().unwrap_or(String::from(""));
        let mime: Mime = actions::detect_mime_type(x.headers());
        let extension: &'static str = actions::conv_mime_type_to_extension(&mime);

        Response {
            text: response_text, 
            mime_type: mime, 
            extension: extension, 
            highlight: None
        }
    }
}

impl Response {
    fn with_highlight_override(self, highlight: Option<String>) -> Self {
        Response {
            text: self.text,
            mime_type: self.mime_type,
            extension: self.extension,
            highlight: highlight
        }
    }
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
        let resp_mtx: sourceview::View = builder.get_object("respMtx").expect("respMtx not found");
        let req_mtx: sourceview::View = builder.get_object("reqMtx").expect("reqMtx not found");
        let headers_mtx: sourceview::View = builder.get_object("headersMtx").expect("headersMtx not found");
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

        match config.request {
            Some(ref text) => req_mtx.replace_all_text(&text),
            None => ()
        };
        
        window.set_application(application);

        find_acm.connect_activate(gtk_clone!(search_bar => move |_| {
            search_bar.set_search_mode(!search_bar.get_search_mode());
        }));

        search_inp.connect_activate(move |search_inp| {
            search_inp.emit_next_match();
        });

        search_inp.connect_next_match(gtk_clone!(resp_mtx => move |search_inp| {
            let pattern = search_inp.upcast_ref::<gtk::Entry>().get_all_text();
            let buffer = resp_mtx.get_buffer().unwrap();
            let cursor = buffer.get_insert().unwrap();
            let mut cursor_iter = buffer.get_iter_at_mark(&cursor);

            if cursor_iter == buffer.get_end_iter() {
                buffer.place_cursor(&buffer.get_start_iter());
                cursor_iter = buffer.get_iter_at_mark(&cursor);
            }

            let found = cursor_iter.forward_search(&pattern, gtk::TextSearchFlags::CASE_INSENSITIVE, None);

            match found {
                Some(pair) => {
                    buffer.select_range(&pair.1, &pair.0);
                    resp_mtx.scroll_mark_onscreen(&cursor);
                },
                None => buffer.place_cursor(&buffer.get_end_iter())
            };
        }));

        let style_manager = sourceview::StyleSchemeManager::new();
        let lang_manager = sourceview::LanguageManager::new();

        let executable_path = std::env::current_exe().ok().
            map(|x| x.with_file_name("")).as_ref().
            and_then(|x| x.to_str()).
            map(|x| String::from(x));

        match executable_path {
            Some(p) => style_manager.append_search_path(&p),
            None => ()
        };

        style_manager
            .get_scheme("tomorrownighteighties")
            .or(style_manager.get_scheme("Classic"))
            .map(|theme| {
                gtk_ext::apply_to_src_buf(&resp_mtx, &|x| x.set_style_scheme(&theme));
                gtk_ext::apply_to_src_buf(&headers_mtx, &|x| x.set_style_scheme(&theme));
                gtk_ext::apply_to_src_buf(&req_mtx, &|x| x.set_style_scheme(&theme));
            });

        lang_manager.
            guess_language(Some("headers.ini"), None).
            map(|lang| gtk_ext::apply_to_src_buf(&headers_mtx, &|x| x.set_language(&lang)));

        MainWindow {
            window: window, 
            perform_btn: perform_btn, 
            url_inp: url_inp, 
            resp_mtx: resp_mtx, 
            headers_mtx: headers_mtx,
            method_sel: method_sel,
            req_mtx: req_mtx,
            lang_manager: lang_manager,
        }
    }
}

pub fn build_ui(application: &gtk::Application) {

    gtk::Settings::get_default().unwrap().set_property_gtk_application_prefer_dark_theme(true);

    let config = config::get_current_config();
    let m_win = MainWindow::new(include_str!("main.glade"), &config, application);
    
    m_win.window.connect_delete_event(gtk_clone!(m_win => move |_, _| {
        let new_config = config::Config {
            url: m_win.url_inp.get_all_text(), 
            height: m_win.window.get_allocated_height() as u32, 
            width: m_win.window.get_allocated_width() as u32,
            headers: m_win.headers_mtx.get_all_text(),
            request: Some(m_win.req_mtx.get_all_text()),
        };

        config::write_config(&new_config);
        m_win.window.destroy();
        Inhibit(false)
    }));

    let (tx, rx) = channel();
    GLOBAL.with(|global| {
        *global.borrow_mut() = Some((m_win.clone(), rx))
    });

    m_win.perform_btn.connect_clicked(gtk_clone!(m_win => move |_| {
        m_win.perform_btn.set_sensitive(false);
        let headers = actions::populate_headers(&m_win.headers_mtx.get_all_text(), &m_win.window);
        let highlight_override = headers.
            get("X-AU-Syntax").
            map(|x| x.as_bytes()).
            and_then(|y| std::str::from_utf8(y).ok()).
            map(|x| String::from(x.trim()));

        let request_method = m_win.get_request_method();
        let url = m_win.url_inp.get_all_text();
        let req = m_win.req_mtx.get_all_text();
        let thread_tx = tx.clone();

        std::thread::spawn(move|| actions::http_worker(
            request_method, 
            &url, 
            req, 
            highlight_override, 
            headers, 
            thread_tx
        ));
    }));

    m_win.window.show_all();
}

thread_local!(
    static GLOBAL: RefCell<Option<(MainWindow, Receiver<Result<Response, String>>)>> = RefCell::new(None)
);

pub fn receive() -> glib::Continue {
    GLOBAL.with(|global| {
        if let Some((ref m_win, ref rx)) = *global.borrow() {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(resp) => actions::output_to_sourceview(&m_win, &resp),
                    Err(err) => gtk_ext::show_message(&err, &m_win.window),
                };
                m_win.perform_btn.set_sensitive(true);
            }
        }
    });
    glib::Continue(false)
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

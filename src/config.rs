use std::path::{PathBuf};
use dirs;
use std;
use rusqlite::Connection;
use std::collections::HashMap;
use gtk::{WidgetExt};
use actions;

fn get_state_path() -> PathBuf {
    let mut path: PathBuf = dirs::home_dir().expect("Cannot get user home directory location");
    path.push(".auweb.db");

    path
}

#[allow(unused_must_use)]
pub fn connect_to_state() -> Connection {
    let connection = Connection::open(get_state_path()).expect("Cannot connect to database");

    connection.execute("CREATE TABLE state_values (
        option_key TEXT NOT NULL UNIQUE,
        option_value TEXT NOT NULL
        )", &[]);

    connection
}

#[derive(Debug, Clone)]
pub struct WindowState {
    pub top_left_offset: i32,
    pub top_right_offset: i32,
    pub vertical_offset: i32,
    pub window_height: i32,
    pub window_width: i32,
    pub req_headers: String,
    pub req_body: String,
    pub rs_headers: String,
    pub rs_body: String,
    pub current_url: String,
    pub current_extension: Option<String>,
    pub current_mime: Option<String>,
    pub request_method: i32,
}

#[derive(Debug, Clone)]
struct StateOption {
    pub option_key: String,
    pub option_value: String,
}

pub const TOP_LEFT_OFFSET: &'static str = "top_left_offset";
pub const TOP_RIGHT_OFFSET: &'static str = "top_right_offset";
pub const VERTICAL_OFFSET: &'static str = "vertical_offset";
pub const WINDOW_HEIGHT: &'static str = "window_height";
pub const WINDOW_WIDTH: &'static str = "window_width";
pub const REQ_HEADERS: &'static str = "req_headers";
pub const REQ_BODY: &'static str = "req_body";
pub const RS_HEADERS: &'static str = "rs_headers";
pub const RS_BODY: &'static str = "rs_body";
pub const CURRENT_URL: &'static str = "current_url";
pub const CURRENT_EXTENSION: &'static str = "current_extension";
pub const CURRENT_MIME: &'static str = "current_mime";
pub const REQUEST_METHOD: &'static str = "request_method";

impl WindowState {
    pub fn read_from_db(connection: &Connection) -> Self {
        let mut stmt = connection.prepare("SELECT option_key, option_value FROM state_values").unwrap();
        let option_iter = stmt.query_map(&[], |row| {
            StateOption {
                option_key: row.get(0),
                option_value: row.get(1)
            }
        }).unwrap();

        let mut dict = HashMap::new();

        for option in option_iter {
            match option.ok() {
                Some(o) => dict.insert(o.option_key, o.option_value),
                _ => None
            };
        }

        WindowState {
            top_left_offset: WindowState::parse_option(&dict, TOP_LEFT_OFFSET, 200),
            top_right_offset: WindowState::parse_option(&dict, TOP_RIGHT_OFFSET, 200),
            vertical_offset: WindowState::parse_option(&dict, VERTICAL_OFFSET, 300),
            window_height: WindowState::parse_option(&dict, WINDOW_HEIGHT, 600),
            window_width: WindowState::parse_option(&dict, WINDOW_WIDTH, 1024),
            req_headers: WindowState::parse_str(&dict, REQ_HEADERS),
            req_body: WindowState::parse_str(&dict, REQ_BODY),
            rs_headers: WindowState::parse_str(&dict, RS_HEADERS),
            rs_body: WindowState::parse_str(&dict, RS_BODY),
            current_url: WindowState::parse_str_or(&dict, CURRENT_URL, "https://api.github.com/users/kykc/repos"),
            current_extension: WindowState::parse_option_str(&dict, CURRENT_EXTENSION),
            current_mime: WindowState::parse_option_str(&dict, CURRENT_MIME),
            request_method: WindowState::parse_option(&dict, REQUEST_METHOD, ::RequestMethod::GetWithUri as i32),
        }
    }

    pub fn update_from_window(&mut self, m_win: &::MainWindow) {
        self.top_left_offset = m_win.get_paned_top_left();
        self.top_right_offset = m_win.get_paned_top_right();
        self.vertical_offset = m_win.get_vertical_offset();
        self.window_height = m_win.window.get_allocated_height();
        self.window_width = m_win.window.get_allocated_width();
        self.req_headers = m_win.get_req_headers();
        self.req_body = m_win.get_req_body();
        self.rs_headers = m_win.get_rs_headers();
        self.rs_body = m_win.get_rs_body();
        self.current_url = m_win.get_url();
        self.request_method = m_win.get_request_method() as i32;
    }

    pub fn update_to_window(&self, m_win: &::MainWindow) {
        m_win.set_paned_top_left(self.top_left_offset);
        m_win.set_paned_top_right(self.top_right_offset);
        m_win.set_vertical_offset(self.vertical_offset);
        m_win.set_window_size(self.window_width, self.window_height);
        m_win.set_req_headers(&self.req_headers);
        m_win.set_req_body(&self.req_body);
        m_win.set_rs_headers(&self.rs_headers);
        m_win.set_rs_body(&self.rs_body);
        m_win.set_url(&self.current_url);
        m_win.set_request_method(WindowState::to_req_method(self.request_method));
        actions::update_resp_body_highlighting(&m_win);
    }

    pub fn to_req_method(i: i32) -> ::RequestMethod {
        match i {
            2 => ::RequestMethod::PostWithForm,
            3 => ::RequestMethod::PostRaw,
            _ => ::RequestMethod::GetWithUri,
        }
    }

    #[allow(unused_must_use)]
    pub fn write_to_db(&self, connection: &Connection) {

        let q = "INSERT OR REPLACE INTO state_values (option_key, option_value) VALUES (?1, ?2);";

        connection.execute(q, &[&TOP_LEFT_OFFSET, &self.top_left_offset]);
        connection.execute(q, &[&TOP_RIGHT_OFFSET, &self.top_right_offset]);
        connection.execute(q, &[&VERTICAL_OFFSET, &self.vertical_offset]);
        connection.execute(q, &[&WINDOW_HEIGHT, &self.window_height]);
        connection.execute(q, &[&WINDOW_WIDTH, &self.window_width]);
        connection.execute(q, &[&REQ_HEADERS, &self.req_headers.as_str()]);
        connection.execute(q, &[&REQ_BODY, &self.req_body.as_str()]);
        connection.execute(q, &[&RS_HEADERS, &self.rs_headers.as_str()]);
        connection.execute(q, &[&RS_BODY, &self.rs_body.as_str()]);
        connection.execute(q, &[&CURRENT_URL, &self.current_url.as_str()]);
        connection.execute(q, &[&CURRENT_EXTENSION, &WindowState::optional_string_to_db(&self.current_extension)]);
        connection.execute(q, &[&CURRENT_MIME, &WindowState::optional_string_to_db(&self.current_mime)]);
        connection.execute(q, &[&REQUEST_METHOD, &self.request_method]);
    }

    fn optional_string_to_db(opt: &Option<String>) -> String {
        opt.as_ref().unwrap_or(&String::new()).clone()
    }

    fn parse_option<T: std::str::FromStr>(hash: &HashMap<String, String>, key: &str, default: T) -> T {
        hash.get(key).and_then(|x| x.parse::<T>().ok()).unwrap_or(default)
    }

    fn parse_str(hash: &HashMap<String, String>, key: &str) -> String {
        WindowState::parse_str_or(hash, key, "")
    }

    fn parse_str_or(hash: &HashMap<String, String>, key: &str, def: &str) -> String {
        (hash.get(key).unwrap_or(&String::from(def))).clone()
    }

    fn parse_option_str(hash: &HashMap<String, String>, key: &str) -> Option<String> {
        hash.get(key).map(|x|String::from(x.as_str()))
    }
}
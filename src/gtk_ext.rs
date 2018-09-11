use text_out::{TextWidget, TextWidgetStyled};
use gtk::{TextView, Entry, TextViewExt, EntryExt, TextBufferExt, TextBuffer, TextTagTableExt, TextTag, TextTagExt, DialogExt, WidgetExt};
use gdk::RGBA;
use gtk;
use syntect::highlighting::{Style, Color};
use serde_json;

macro_rules! gtk_clone {
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
            move |$(gtk_clone!(@param $p),)+| $body
        }
    );
}

impl TextWidgetStyled for TextView {
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

impl TextWidget for Entry {
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

impl TextWidget for TextView {
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

fn from_parser_to_gdk(c: Color) -> RGBA {
    let mut result = RGBA::red();
    let norm = 255f64;
    result.alpha = (c.a as f64) / norm;
    result.blue = (c.b as f64) / norm;
    result.red = (c.r as f64) / norm;
    result.green = (c.g as f64) / norm;

    result
}

fn conv_to_tag(buffer: &TextBuffer, style: &Style) -> String {
    // TODO: more efficient hashing instead of JSON serialization
    let hash: String = serde_json::to_string(&style).expect("Cannot serialize style to JSON string");
    let table = buffer.get_tag_table().unwrap();
    
    if !table.lookup(&hash).is_some() {       
        let tag = TextTag::new(Some(hash.as_str()));
        tag.set_property_foreground_rgba(Some(&from_parser_to_gdk(style.foreground)));
        tag.set_property_background_rgba(Some(&from_parser_to_gdk(style.background)));
        table.add(&tag);
    }

    hash
}

pub fn show_message<T: gtk::prelude::IsA<gtk::Window>>(msg: &str, window: &T) {
    let dialog = gtk::MessageDialog::new(Some(window), gtk::DialogFlags::MODAL, gtk::MessageType::Warning, gtk::ButtonsType::Ok, msg);
    dialog.connect_response(|dialog, _| dialog.destroy());
    dialog.run();
}

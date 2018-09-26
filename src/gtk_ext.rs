use gtk::{TextView, Entry, TextViewExt, EntryExt, TextBufferExt, DialogExt, WidgetExt, ContainerExt, Cast};
use gtk;
use sourceview;

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

macro_rules! impl_text {
    ($($t:ty),+) => {
        $(impl TextWidget for $t {
            fn get_all_text(&self) -> String {
                self.get_buffer().
                    and_then(|buf| buf.get_text(&mut buf.get_start_iter(), &mut buf.get_end_iter(), true)).
                    unwrap_or(String::from(""))
            }

            fn replace_all_text(&self, new_text: &str) {
                self.get_buffer().map(|buf| {
                    buf.delete(&mut buf.get_start_iter(), &mut buf.get_end_iter());
                    buf.insert(&mut buf.get_start_iter(), new_text);
                });
            }

            fn clear_all_text(&self) {
                self.get_buffer().map(|buf| {
                    buf.delete(&mut buf.get_start_iter(), &mut buf.get_end_iter());
                });
            }

            fn append_text(&self, add_text: &str) {
                self.get_buffer().map(|buf| {
                    buf.insert(&mut buf.get_end_iter(), add_text);
                });
            }
        })+
    }
}

pub trait TextWidget {
    fn get_all_text(&self) -> String;
    fn replace_all_text(&self, text: &str);
    fn clear_all_text(&self);
    fn append_text(&self, text: &str);
}

pub fn apply_to_src_buf(view: &sourceview::View, worker: &Fn(&sourceview::Buffer)) -> bool {
    let result = match view.get_buffer().as_ref().and_then(|x| x.downcast_ref::<sourceview::Buffer>()) {
        Some(x) => { 
            worker(x); 
            true
        },
        None => false
    };

    result
}

pub fn get_gtk_obj_by_id<T: gtk::IsA<gtk::Object>>(builder: &gtk::Builder, id: &str) -> T {
    builder.get_object(id).expect(&(String::from("Couldn't get ") + id))
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

pub fn show_message<T: gtk::prelude::IsA<gtk::Window>>(msg: &str, window: &T) {
    let dialog = gtk::MessageDialog::new(
        Some(window), 
        gtk::DialogFlags::MODAL, 
        gtk::MessageType::Warning, 
        gtk::ButtonsType::Ok, msg
    );

    dialog.connect_response(|dialog, _| dialog.destroy());
    dialog.run();
}

pub fn traverse_gtk_container(container: &gtk::Container, worker: &Fn(&gtk::Container)) {
    worker(&container);
    
    for widget in container.get_children() {
        let sub_container: Option<&gtk::Container> = widget.downcast_ref();
        sub_container.map(|sub| traverse_gtk_container(&sub, worker));
    }
}

impl_text!(TextView, sourceview::View);
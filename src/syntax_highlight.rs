pub const CONTENT_TYPE_JSON: &'static str = "json";
pub const CONTENT_TYPE_DEFAULT: &'static str = "";
pub const CONTENT_TYPE_XML: &'static str = "xml";
pub const CONTENT_TYPE_HTML: &'static str = "html";

use sourceview;
use gtk::TextViewExt;
use gtk::Cast;
use sourceview::{BufferExt, StyleSchemeManager, StyleSchemeManagerExt, LanguageManagerExt};
use text_out::TextWidget;

pub fn output_to_sourceview(target: &sourceview::View, text: &str, extension: &str, content_type: Option<&str>) {
    sourceview::LanguageManager::new().
        guess_language(Some((String::from("dummy.") + extension).as_str()), content_type).
        map(|lang| target.get_buffer().unwrap().downcast_ref::<sourceview::Buffer>().unwrap().set_language(&lang));

    let manager = StyleSchemeManager::new();
    manager
        .get_scheme("tomorrownighteighties")
        .or(manager.get_scheme("Classic"))
        .map(|theme| target.get_buffer().unwrap().downcast_ref::<sourceview::Buffer>().unwrap().set_style_scheme(&theme));
    
    target.replace_all_text(text);
}

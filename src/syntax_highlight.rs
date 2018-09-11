pub const CONTENT_TYPE_JSON: &'static str = "json";
pub const CONTENT_TYPE_DEFAULT: &'static str = "";
pub const CONTENT_TYPE_XML: &'static str = "xml";
pub const CONTENT_TYPE_HTML: &'static str = "html";

use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::syntax_definition::SyntaxDefinition;
use syntect::easy::HighlightLines;
use static_map::Map;
use static_map;

static KNOWN_CONTENT_TYPES: Map<&'static str, ContentTypeRules> = static_map! {
    Default: ContentTypeRules{syntax_load_strategy: SyntaxLoadStrategy::None},
    "" => ContentTypeRules{syntax_load_strategy: SyntaxLoadStrategy::None},
    "json" => ContentTypeRules{syntax_load_strategy: SyntaxLoadStrategy::Custom(include_str!("json.sublime-syntax"))},
    "xml" => ContentTypeRules{syntax_load_strategy: SyntaxLoadStrategy::Default},
    "html" => ContentTypeRules{syntax_load_strategy: SyntaxLoadStrategy::Default},
};

#[derive(Clone)]
enum SyntaxLoadStrategy {
    Default,
    Custom(&'static str),
    None,
}

struct ContentTypeRules {
    pub syntax_load_strategy: SyntaxLoadStrategy,
}

impl ContentTypeRules {
    #[allow(dead_code)]
    pub fn new(strategy: &SyntaxLoadStrategy) -> ContentTypeRules {
        ContentTypeRules{syntax_load_strategy: strategy.clone()}
    }
}

pub fn create_syntax_set() -> SyntaxSet {
    let mut set = SyntaxSet::new();
    let default_set = SyntaxSet::load_defaults_nonewlines();
    
    for content_type in KNOWN_CONTENT_TYPES.keys() {
        let syntax = match KNOWN_CONTENT_TYPES.get_entry(content_type).unwrap().1.syntax_load_strategy {
            SyntaxLoadStrategy::Default => default_set.find_syntax_by_extension(content_type).expect("Cannot find syntax parser").clone(),
            SyntaxLoadStrategy::Custom(syntax_file) => SyntaxDefinition::load_from_str(syntax_file, true, None).expect("Failed to parse syntax def"),
            SyntaxLoadStrategy::None => default_set.find_syntax_plain_text().clone(),
        };

        set.add_syntax(syntax);
    }

    // Load all available default syntax definitions which were not defined explicitly
    for syntax in default_set.syntaxes() {
        if syntax.file_extensions.iter().all(|ext| !KNOWN_CONTENT_TYPES.contains_key::<str>(&ext.to_owned())) {
            set.add_syntax(syntax.clone());
        }
    }

    set.link_syntaxes();

    set
}

pub fn output_with_syntax_highlight<T: ::text_out::TextWidget + ::text_out::TextWidgetStyled>(target: &T, text: &str, extension: &str, 
    syntaxes: &SyntaxSet, themes: &ThemeSet) {

    let syntax = match extension {
        "" => syntaxes.find_syntax_plain_text(),
        _ => match syntaxes.find_syntax_by_extension(extension) {
            Some(s) => s,
            None => syntaxes.find_syntax_plain_text()
        }
    };

    let mut h = HighlightLines::new(syntax, &themes.themes["base16-ocean.dark"]);
    target.clear_all_text();

    for line in text.lines() {
        let unprocessed = String::from(line) + "\n";
        
        let ranges: Vec<(Style, &str)> = h.highlight(&unprocessed);
        
        for range in &ranges {
            target.append_styled_text(range.1, &range.0);
        }
        //target.append_text("\n");
    }
}
use syntect::highlighting::Style;

pub trait TextWidget {
    fn get_all_text(&self) -> String;
    fn replace_all_text(&self, &str);
    fn clear_all_text(&self);
    fn append_text(&self, &str);
}

pub trait TextWidgetStyled {
    fn append_styled_text(&self, &str, &Style);
}
use quick_xml::Writer;
use quick_xml::Reader;
use quick_xml::events::{Event, BytesEnd, BytesStart};
use std::io::Cursor;
use std;

pub fn beautify_xml(xml: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), 0x20, 4);
    let mut buf = Vec::new();
    let mut success = true;
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let mut elem = BytesStart::owned(e.name().to_vec(), e.name().len());
                elem.extend_attributes(e.attributes().map(|attr| attr.unwrap()));
                
                if !writer.write_event(Event::Start(elem)).is_ok() {
                    success = false;
                    break;
                }
            },
            Ok(Event::End(ref e)) => {
                if !writer.write_event(Event::End(BytesEnd::borrowed(e.name()))).is_ok() {
                    success = false;
                    break;
                }
            },
            Ok(Event::Eof) => break,
            Ok(e) => if !writer.write_event(&e).is_ok() {
                success = false;
                break;
            }
            Err(_) => {
                success = false;
                break;
            },
        }
        buf.clear();
    }

    if success {
        let result = writer.into_inner().into_inner();

        std::str::from_utf8(&result).map(|x| String::from(x)).ok()
    } else {
        None
    }
}
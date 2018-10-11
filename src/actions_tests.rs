use super::actions;

#[test]
pub fn it_works() {
    assert_eq!(4, 2 + 2);
}

#[test]
pub fn test_parse_headers() {
    let empty = actions::parse_headers("", &mut |_|{});

    assert_eq!(empty.len(), 0);

    let mut test_vec1: Vec<String> = Vec::new();

    let test1 = actions::parse_headers(include_str!("test_data/headers1"), &mut |msg|{
        test_vec1.push(String::from(msg));
    });

    assert_eq!(test1.len(), 3);
    assert!(test1.contains_key("content-type"));
    assert!(test1.contains_key("cookie"));
    assert!(!test1.contains_key("connection"));
    assert_eq!(test1["cookie"], "");
    assert_eq!(test1["soapaction"], " http://example.com/action");
    assert_eq!(test1["content-type"], " application/json");
    assert_eq!(test_vec1.len(), 0);
}
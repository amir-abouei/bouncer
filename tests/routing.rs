use bouncer::routing::extract_alias;

#[test]
fn path_based_when_no_header() {
    assert_eq!(
        extract_alias("/kucoin/api/v1/timestamp", None),
        Some(("kucoin", "api/v1/timestamp"))
    );
}

#[test]
fn path_based_alias_only_no_rest() {
    assert_eq!(extract_alias("/kucoin", None), Some(("kucoin", "")));
}

#[test]
fn path_based_empty_path_is_none() {
    assert_eq!(extract_alias("/", None), None);
    assert_eq!(extract_alias("", None), None);
}

#[test]
fn header_based_overrides_path_and_forwards_it_unprefixed() {
    assert_eq!(
        extract_alias("/bot12345/getMe", Some("telegram")),
        Some(("telegram", "bot12345/getMe"))
    );
}

#[test]
fn header_based_with_root_path() {
    assert_eq!(extract_alias("/", Some("telegram")), Some(("telegram", "")));
}

#[test]
fn header_value_is_trimmed() {
    assert_eq!(
        extract_alias("/anything", Some(" telegram ")),
        Some(("telegram", "anything"))
    );
}

#[test]
fn blank_header_falls_back_to_path() {
    assert_eq!(extract_alias("/kucoin/get", Some("   ")), Some(("kucoin", "get")));
}

#[test]
fn absent_header_falls_back_to_path() {
    assert_eq!(
        extract_alias("/kucoin/get", None),
        extract_alias("/kucoin/get", Some(""))
    );
}

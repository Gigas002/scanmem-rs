use super::*;

const BASIC: &str = include_str!("../../tests/fixtures/maps/basic.txt");
const MALFORMED: &str = include_str!("../../tests/fixtures/maps/malformed.txt");

#[test]
fn parses_every_region_in_the_fixture() {
    let regions = parse_maps(BASIC);
    assert_eq!(regions.len(), 13);
}

#[test]
fn parses_address_range_and_permissions() {
    let regions = parse_maps(BASIC);
    let exe = &regions[0];
    assert_eq!(exe.start, 0x00400000);
    assert_eq!(exe.end, 0x00452000);
    assert_eq!(exe.size(), 0x52000);
    assert_eq!(
        exe.perms,
        Perms {
            read: true,
            write: false,
            exec: true,
            shared: false,
        }
    );
    assert_eq!(exe.path.as_deref(), Some("/usr/bin/example"));
}

#[test]
fn parses_anonymous_region_with_no_path() {
    let regions = parse_maps(BASIC);
    let anon = regions.iter().find(|r| r.start == 0x00653000).unwrap();
    assert_eq!(anon.path, None);
    assert_eq!(anon.kind(), RegionKind::Anonymous);
}

#[test]
fn classifies_heap_and_stack() {
    let regions = parse_maps(BASIC);
    assert!(regions.iter().any(|r| r.kind() == RegionKind::Heap));
    assert!(regions.iter().any(|r| r.kind() == RegionKind::Stack));
    assert!(
        regions
            .iter()
            .any(|r| r.path.as_deref() == Some("/usr/bin/example") && r.kind() == RegionKind::File)
    );
}

#[test]
fn is_writable_reflects_the_w_permission_bit() {
    let regions = parse_maps(BASIC);
    let read_only = regions.iter().find(|r| r.start == 0x00651000).unwrap();
    let writable = regions.iter().find(|r| r.start == 0x00652000).unwrap();
    assert!(!read_only.is_writable());
    assert!(writable.is_writable());
}

#[test]
fn skips_malformed_and_blank_lines() {
    let regions = parse_maps(MALFORMED);
    assert_eq!(regions.len(), 3);
}

#[test]
fn returns_none_for_a_line_with_no_dash_in_the_range() {
    assert_eq!(parse_line("not-a-range-at-all"), None);
    assert_eq!(parse_line(""), None);
}

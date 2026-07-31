use super::greet;

#[test]
fn greet_works() {
    assert_eq!(greet("world"), "Hello, world!");
}

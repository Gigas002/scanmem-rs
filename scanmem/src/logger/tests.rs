use super::init;
use crate::settings::Settings;

#[test]
fn init_does_not_panic_when_called_repeatedly() {
    init(&Settings::default());
    init(&Settings::default());
}

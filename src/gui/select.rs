use crate::MyApp;

pub fn select_all(interface: &mut MyApp) {
    if interface.select_all {
        for core in interface.inner.iter_mut() {
            core.selected = true;
        }
    }
}

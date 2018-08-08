extern crate ifcontrol;

fn main() {
    ifcontrol::Iface::find_by_name("asd").unwrap();
}

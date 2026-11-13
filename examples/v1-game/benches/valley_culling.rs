//! Capture-off paired timing on the same moving route as the exact oracle.
#[path = "../tests/support/valley_route.rs"]
mod valley_route;

fn main() {
    valley_route::run();
}

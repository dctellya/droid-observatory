mod app;
mod backend;
mod board;
mod lanes;
mod library;
mod panel;
mod tray;

use leptos::prelude::*;

fn main() {
    mount_to_body(app::App);
}

mod agents;
mod app;
mod backend;
mod board;
mod ctx;
mod details;
mod lanes;
mod library;
mod panel;
mod raw;
mod tray;

use leptos::prelude::*;

fn main() {
    mount_to_body(app::App);
}

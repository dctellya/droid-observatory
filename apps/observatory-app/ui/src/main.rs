mod agents;
mod app;
mod attach;
mod backend;
mod board;
mod capture;
mod controls;
mod ctx;
mod details;
mod lanes;
mod launch;
mod library;
mod panel;
mod perf;
mod raw;
mod shell;
mod tray;

use leptos::prelude::*;

fn main() {
    mount_to_body(app::App);
}

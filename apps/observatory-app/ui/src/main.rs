mod agents;
mod app;
mod attach;
mod backend;
mod board;
mod capture;
mod controls;
mod ctx;
mod details;
mod gate;
mod lanes;
mod launch;
mod library;
mod pagehead;
mod panel;
mod panel_charts;
mod panel_usage;
mod perf;
mod raw;
mod shell;
mod statusbar;
mod tabs;
mod tray;

use leptos::prelude::*;

fn main() {
    mount_to_body(app::App);
}

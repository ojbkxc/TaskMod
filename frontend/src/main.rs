use dioxus::prelude::*;

mod app;
mod api;
mod components;
mod pages;

use app::App;

fn main() {
    dioxus::launch(App);
}

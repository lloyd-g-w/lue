mod app;
mod components;
mod models;
mod pages;
mod route;
mod storage;
mod view_helpers;
mod ws;

fn main() {
    dioxus::launch(app::App);
}

mod app;
mod data;
mod icon;
mod output;
mod stats;
mod syntax;
mod theme;

fn main() -> eframe::Result<()> {
    let initial = std::env::args().nth(1);
    app::run(initial)
}

mod app;
mod data;
mod output;
mod stats;
mod syntax;

fn main() -> eframe::Result<()> {
    let initial = std::env::args().nth(1);
    app::run(initial)
}

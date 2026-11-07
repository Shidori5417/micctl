use std::io::IsTerminal;

mod cli;
mod dsp;
mod sfx;
mod state;
mod tui;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        if std::io::stdin().is_terminal() {
            tui::run_tui();
        } else {
            let st = state::load_state();
            cli::print_status(&st);
        }
    } else {
        cli::handle_cli(args);
    }
}

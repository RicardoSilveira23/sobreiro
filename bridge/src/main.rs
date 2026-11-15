//! `sobreiro-bridge.exe`: the command Claude Code's status line runs. All it does is in the
//! library beside this file, where the tests can reach it.

fn main() {
    std::process::exit(sobreiro_bridge::run());
}

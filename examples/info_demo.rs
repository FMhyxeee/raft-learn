use slog::{o, info};

fn main() {
    let a = "nihao";
    let b = "nihao".to_string();
    let c = 1u8;

    let logger = slog::Logger::root(slog::Discard, o!());

    info!(logger,"a is {a}, b is {b} c is {c}", a = a, b = b.clone(), c = c;);
    println!("ok")
}
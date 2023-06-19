use std::{thread, time::Duration};

fn main() {
    let mut index = 0usize;
    loop {
        thread::sleep(Duration::from_secs(1));
        loop {
            match index {
                0 => {println!("ok"); index += 1},
                1 => { index += 1; break},
                2 => return,
                _ => println!("default"),
            }
        }
    }
}
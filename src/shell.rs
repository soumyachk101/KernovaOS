//! Interactive shell (M13) — an async task over the keyboard scancode stream.
//! Prompt, backspace line editing, tokenizer, builtins, and `run <prog>`.

use crate::fs::{Initrd, Vfs};
use crate::interrupts::TICKS;
use crate::usermode::{self, programs};
use crate::{print, println, vga_buffer};
use alloc::string::String;
use core::sync::atomic::Ordering;
use futures_util::stream::StreamExt;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

const PROMPT: &str = "kernova> ";

pub async fn run_shell() {
    let mut scancodes = crate::task::keyboard::ScancodeStream::new();
    let mut keyboard = Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::Ignore);
    let mut line = String::new();

    println!();
    println!("Kernova shell - type 'help'.");
    print!("{}", PROMPT);

    while let Some(scancode) = scancodes.next().await {
        let Ok(Some(event)) = keyboard.add_byte(scancode) else {
            continue;
        };
        let Some(key) = keyboard.process_keyevent(event) else {
            continue;
        };

        match key {
            DecodedKey::Unicode('\n') => {
                println!();
                execute(&line);
                line.clear();
                print!("{}", PROMPT);
            }
            // backspace (0x08) or delete (0x7f)
            DecodedKey::Unicode('\u{8}') | DecodedKey::Unicode('\u{7f}') => {
                if line.pop().is_some() {
                    vga_buffer::backspace();
                }
            }
            DecodedKey::Unicode(c) => {
                line.push(c);
                print!("{}", c);
            }
            DecodedKey::RawKey(_) => {} // ignore arrows, modifiers, etc.
        }
    }
}

fn execute(line: &str) {
    let mut tokens = line.split_whitespace();
    let Some(cmd) = tokens.next() else {
        return; // empty line
    };

    match cmd {
        "help" => {
            println!("builtins: help echo clear uptime ls cat run");
            println!("run <prog>: hello getpid fault priv");
        }
        "echo" => {
            let rest: String = tokens.collect::<alloc::vec::Vec<_>>().join(" ");
            println!("{}", rest);
        }
        "clear" => vga_buffer::clear_screen(),
        "uptime" => {
            // PIT default ~18.2065 Hz; tenths = ticks*10/182
            let ticks = TICKS.load(Ordering::Relaxed);
            let tenths = ticks * 10 / 182;
            println!("uptime: {}.{}s ({} ticks)", tenths / 10, tenths % 10, ticks);
        }
        "ls" => {
            for name in Initrd::new().list() {
                println!("{}", name);
            }
        }
        "cat" => match tokens.next() {
            Some(path) => match Initrd::new().read(path) {
                Some(data) => print!("{}", core::str::from_utf8(data).unwrap_or("<binary>")),
                None => println!("cat: {}: no such file", path),
            },
            None => println!("usage: cat <file>"),
        },
        "run" => match tokens.next() {
            Some(name) => match programs::by_name(name) {
                Some(blob) => {
                    let code = usermode::run(blob);
                    println!("[{} exited with code {}]", name, code);
                }
                None => println!("run: {}: unknown program (try: hello getpid fault priv)", name),
            },
            None => println!("usage: run <prog>"),
        },
        other => println!("{}: command not found", other),
    }
}

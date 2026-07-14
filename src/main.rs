mod _chunk;
mod _vm;
mod _scanner;
mod _parser;
mod _object;
mod _table;

use std::{env, fs, io::{self, Write}, process};

use crate::_vm::{InterpretResult, VM};

fn main() {
    let mut vm = VM::new();

    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => {
            repl(&mut vm);
        }

        2 => {
            run_file(&mut vm, &args[1]);
        }

        _ => {
            eprintln!("Usage: clox [path]");
            process::exit(64);
        }
    }
}

fn repl(vm: &mut VM) {
    let stdin = io::stdin();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();

        match stdin.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                vm.interpret(&line);
            }
            Err(_) => break,
        }
    }
}

fn run_file(vm: &mut VM, path: &str) {
    let source = fs::read_to_string(path)
        .expect("Could not read file");

    let result = vm.interpret(&source);

    match result {
        InterpretResult::InterpretOk => {}

        InterpretResult::InterpretCompileError => {
            process::exit(65);
        }

        InterpretResult::InterpretRuntimeError => {
            process::exit(70);
        }
    }
}
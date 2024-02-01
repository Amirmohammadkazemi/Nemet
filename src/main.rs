use std::error::Error;
use std::path::PathBuf;
use std::process::Command;
use std::{env::args, process::exit};

mod codegen;
mod command_line;
mod compiler;
mod error_handeling;
mod lexer;
mod macros;
mod parser;
#[cfg(test)]
mod tests;
mod utils;
use command_line::{help_command, CliArgs};
use compiler::compile;
use utils::get_output_path_from_input;

// --- Static Compiler Defenition
pub static VERSION: &str = "v0.0.1-Beta";
pub static COPYRIGHT: &str = "Mahan Farzaneh 2023-2024";
pub static DEBUG: bool = true;

fn assembler_target() -> &'static str {
    if cfg!(windows) {
        "win64"
    } else {
        "elf64"
    }
}

/// Compiles the given file into an executable
fn compile_command(arg: &mut CliArgs) {
    if arg.get().starts_with('-') {
        match arg.get().as_str() {
            "-elf" => {
                arg.next();
                let out_path = get_output_path_from_input(arg.get());
                compile(
                    arg.get(),
                    out_path.with_extension("o"),
                    compiler::OutputType::Elf,
                );
                link_to_exc(out_path.with_extension(""));
                exit(0);
            }
            _ => {
                eprintln!("Unknown option for compile command!");
                exit(-1);
            }
        }
    }
    let out_path = get_output_path_from_input(arg.get());
    compile(
        arg.get(),
        out_path.with_extension("asm"),
        compiler::OutputType::Asm,
    );
    assemble_with_nasm(out_path.with_extension("o"));
    link_to_exc(out_path.with_extension(""));
}

/// Runs External commands for generating the object files
pub fn assemble_with_nasm(path: PathBuf) {
    println!(
        "[info] Assembling for {} - generaiting output.o",
        assembler_target()
    );
    let nasm_output = Command::new("nasm")
        .arg(format!("-f{}", assembler_target()))
        .arg("-o")
        .arg(path.with_extension("o"))
        .arg(path.with_extension("asm"))
        .output()
        .expect("Can not run nasm command! do you have nasm installed?");
    if !nasm_output.status.success() {
        println!("[error] Failed to Assemble: Status code non zero");
        println!("{}", String::from_utf8(nasm_output.stderr).unwrap());
    }
}

/// Runs External commands for generating the executable
pub fn link_to_exc(path: PathBuf) {
    println!("[info] Linking object file...");
    let linker_output = Command::new("ld")
        .arg("-o")
        .arg(path.with_extension(""))
        .arg(path.with_extension("o"))
        .output()
        .expect("Can not link using ld command!");
    if !linker_output.status.success() {
        println!("[error] Failed to Link Exectable: Status code non zero");
        println!("{}", String::from_utf8(linker_output.stderr).unwrap());
    } else {
        println!("[sucsees] Executable File Has been Generated!");
    }
}

/// Run The Program Directly after generating the executable
pub fn run_program() {
    println!("+ Running The Generated Executable");
    let output = Command::new("./build/output")
        .output()
        .expect("Error Executing the program!");
    println!("{}", String::from_utf8(output.stdout).unwrap());
}

/// Executes commands resived by commandline
/// nemet [commands] <options> [path]
/// First level of command line argument parsing
fn commands(arg: &mut CliArgs) {
    match arg.get().as_str() {
        "--help" | "help" | "-h" => {
            help_command();
        }
        "--version" | "-v" => {
            println!("{VERSION}");
        }
        "--compile" | "-c" => {
            arg.next();
            compile_command(arg);
        }
        _ => {
            compile_command(arg);
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = CliArgs::new(args().collect());
    commands(&mut args);
    Ok(())
}

use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use naso_compiler::lexer::Lexer;
use naso_compiler::lowering::lower_program;
use naso_compiler::parser::parse_program;
use naso_compiler::typecheck::check_program;

#[cfg(feature = "llvm")]
use naso_compiler::codegen::{
    Backend, CodegenConfig, CodegenContext, CodegenPipeline, CodegenTarget, OptLevel,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "parse" | "tokens" | "check" => {
            if args.len() < 3 {
                eprintln!("Usage: naso {command} <file>");
                std::process::exit(1);
            }
            run_frontend_command(command, &args[2]);
        }
        "build" => {
            run_build_command(&args[2..]);
        }
        _ => {
            eprintln!("Unknown command: {command}");
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("Usage: naso <command> [args]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  parse <file>          Parse and print AST as JSON");
    eprintln!("  tokens <file>         Print token stream");
    eprintln!("  check <file>          Type check program");
    eprintln!("  build [options] <file>  Build program to target");
    eprintln!("Build options:");
    eprintln!("  --target <llvm|qir|cranelift>  Target backend (default: llvm)");
    eprintln!("  -o, --output <file>            Output file path");
    eprintln!("  --opt <0|1|2|3>                Optimization level (default: 2)");
    eprintln!("  --triple <target>              Target triple (host, nvptx64, wasm32, aarch64)");
    eprintln!("  --debug                        Emit debug information");
    eprintln!();
    eprintln!("Verify options:");
    eprintln!("  --mode <all|uncomputation|linearity>  Verification mode (default: all)");
    eprintln!("  --format <human|json|sarif>           Output format (default: human)");
    eprintln!("  --jobs <N>                              Parallel jobs (default: auto)");
    eprintln!("  --timeout <MS>                          Solver timeout in ms (default: 30000)");
    eprintln!("  --no-cache                              Disable incremental cache");
}

fn run_frontend_command(command: &str, file: &str) {
    let file_path = PathBuf::from(file);
    let source = fs::read_to_string(&file_path).unwrap_or_else(|e| {
        eprintln!("error: cannot read `{}`: {e}", file_path.display());
        std::process::exit(1);
    });

    match command {
        "parse" => match parse_program(&source) {
            Ok(program) => {
                let json = serde_json::to_string_pretty(&program).expect("failed to serialize AST");
                println!("{json}");
            }
            Err(e) => {
                eprintln!("parse error: {e}");
                std::process::exit(1);
            }
        },
        "tokens" => {
            let tokens = Lexer::lex(&source);
            for tok in &tokens {
                let kind_name = format!("{:?}", tok.kind);
                let short = match &tok.kind {
                    naso_compiler::lexer::TokenKind::Comment => "comment".into(),
                    naso_compiler::lexer::TokenKind::Newline => "newline".into(),
                    other => format!("{other}"),
                };
                println!(
                    "{:4}-{:4}  {:<14}  {}",
                    tok.span.start, tok.span.end, kind_name, short
                );
            }
        }
        "check" => {
            let mut program = match parse_program(&source) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("parse error: {e}");
                    std::process::exit(1);
                }
            };
            let result = check_program(&mut program);
            if result.errors.is_empty() {
                println!("OK");
                std::process::exit(0);
            } else {
                for e in &result.errors {
                    eprintln!("{e}");
                }
                std::process::exit(1);
            }
        }
        _ => unreachable!(),
    }
}

#[cfg(feature = "llvm")]
fn run_build_command(args: &[String]) {
    let mut config = CodegenConfig::default();
    let mut input_file = None;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--target" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("--target requires an argument");
                    std::process::exit(1);
                }
                config.backend = match args[i].as_str() {
                    "llvm" => Backend::Llvm,
                    "qir" => Backend::Qir,
                    "cranelift" => Backend::Cranelift,
                    other => {
                        eprintln!("Unknown target: {other}. Use llvm, qir, or cranelift");
                        std::process::exit(1);
                    }
                };
            }
            "-o" | "--output" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("--output requires an argument");
                    std::process::exit(1);
                }
                config.output_path = Some(PathBuf::from(&args[i]));
            }
            "--opt" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("--opt requires an argument");
                    std::process::exit(1);
                }
                config.opt_level = match args[i].as_str() {
                    "0" => OptLevel::None,
                    "1" => OptLevel::Less,
                    "2" => OptLevel::Default,
                    "3" => OptLevel::Aggressive,
                    other => {
                        eprintln!("Invalid optimization level: {other}. Use 0, 1, 2, or 3");
                        std::process::exit(1);
                    }
                };
            }
            "--triple" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("--triple requires an argument");
                    std::process::exit(1);
                }
                config.target = CodegenTarget::from_str(&args[i]).unwrap_or_else(|e| {
                    eprintln!("Invalid target triple: {e}");
                    std::process::exit(1);
                });
            }
            "--debug" => {
                config.emit_debug = true;
            }
            arg if arg.starts_with('-') => {
                eprintln!("Unknown option: {arg}");
                std::process::exit(1);
            }
            current_arg => {
                if input_file.is_none() {
                    input_file = Some(PathBuf::from(current_arg));
                } else {
                    eprintln!("Multiple input files not supported");
                    std::process::exit(1);
                }
            }
        }
        i += 1;
    }

    let input_file = input_file.unwrap_or_else(|| {
        eprintln!("Missing input file");
        print_usage();
        std::process::exit(1);
    });

    // Read and parse source
    let source = fs::read_to_string(&input_file).unwrap_or_else(|e| {
        eprintln!("error: cannot read `{}`: {e}", input_file.display());
        std::process::exit(1);
    });

    let mut program = match parse_program(&source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("parse error: {e}");
            std::process::exit(1);
        }
    };

    // Type check
    let result = check_program(&mut program);
    if !result.errors.is_empty() {
        for e in &result.errors {
            eprintln!("{e}");
        }
        std::process::exit(1);
    }

    // Lower to PIR
    let pir_module = match lower_program(&program) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("lowering error: {e}");
            std::process::exit(1);
        }
    };

    // Create codegen context
    let codegen_context =
        match CodegenContext::with_debug(config.target, config.opt_level, config.emit_debug) {
            Ok(ctx) => ctx,
            Err(e) => {
                eprintln!("codegen context error: {e}");
                std::process::exit(1);
            }
        };

    // Create pipeline and generate output
    let pipeline = CodegenPipeline::new(codegen_context);

    match config.backend {
        Backend::Llvm => {
            let ir = match pipeline.emit_llvm(&pir_module) {
                Ok(ir) => ir,
                Err(e) => {
                    eprintln!("LLVM codegen error: {e}");
                    std::process::exit(1);
                }
            };
            if let Some(path) = config.output_path {
                fs::write(&path, ir).unwrap_or_else(|e| {
                    eprintln!("Failed to write output: {e}");
                    std::process::exit(1);
                });
                println!("Written LLVM IR to {}", path.display());
            } else {
                print!("{ir}");
            }
        }
        Backend::Qir => {
            let ir = match pipeline.emit_qir(&pir_module) {
                Ok(ir) => ir,
                Err(e) => {
                    eprintln!("QIR codegen error: {e}");
                    std::process::exit(1);
                }
            };
            if let Some(path) = config.output_path {
                fs::write(&path, ir).unwrap_or_else(|e| {
                    eprintln!("Failed to write output: {e}");
                    std::process::exit(1);
                });
                println!("Written QIR to {}", path.display());
            } else {
                print!("{ir}");
            }
        }
        Backend::Cranelift => match pipeline.execute_cranelift_jit(&pir_module) {
            Ok(result) => {
                println!("JIT execution result: {result}");
            }
            Err(e) => {
                eprintln!("Cranelift JIT error: {e}");
                std::process::exit(1);
            }
        },
    }
}

#[cfg(not(feature = "llvm"))]
fn run_build_command(_args: &[String]) {
    eprintln!("Build command requires LLVM backend. Compile with 'llvm' feature.");
    std::process::exit(1);
}


use std::process::ExitCode;

const HELP: &str = "Limivra — base mínima do workspace

Uso: limivra [--help | --version]

Opções:
  -h, --help       Exibe esta ajuda
  -V, --version    Exibe a versão
";

fn main() -> ExitCode {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();

    if args.is_empty() {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    }

    if args.len() == 1 {
        match args[0].to_str() {
            Some("--help" | "-h") => {
                print!("{HELP}");
                return ExitCode::SUCCESS;
            }
            Some("--version" | "-V") => {
                println!("limivra {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            _ => {}
        }
    }

    eprintln!("Comando ou opção não disponível nesta versão. Use --help.");
    ExitCode::from(2)
}

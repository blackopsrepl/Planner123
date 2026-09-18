use clap::{error::ErrorKind, Parser};

fn main() {
    let cli = match planner123::cli::Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                print!("{}", err);
                std::process::exit(0);
            }
            _ => {
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&planner123::cli::error_value(
                        &planner123::cli::CliError::invalid_arguments(err.to_string()),
                    ))
                    .expect("serializable clap error")
                );
                std::process::exit(2);
            }
        },
    };

    match planner123::cli::execute(cli) {
        Ok(value) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).expect("serializable success payload")
            );
        }
        Err(err) => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&planner123::cli::error_value(&err))
                    .expect("serializable error payload")
            );
            std::process::exit(1);
        }
    }
}

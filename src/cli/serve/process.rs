// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use camino::Utf8PathBuf;
use eyre::eyre;

use crate::environment;

fn parse_command(command: &[String], output: Utf8PathBuf) -> eyre::Result<std::process::Command> {
    if command.is_empty() {
        return Err(eyre!(
            "invalid `serve.command`: command list cannot be empty"
        ));
    }

    let mut serve = std::process::Command::new(&command[0]);
    for arg in &command[1..] {
        if arg == "<output>" {
            serve.arg(&output);
            continue;
        }
        serve.arg(arg);
    }
    Ok(serve)
}

/// `spawn` reports a missing program as a bare "No such file or directory",
/// naming neither the program nor the setting that chose it. Since the default
/// server is a separate tool that many machines will not have, that error is
/// the most likely first thing a new user sees from `wanshi serve`.
fn explain_spawn_failure(program: &str, err: std::io::Error) -> eyre::Report {
    if err.kind() != std::io::ErrorKind::NotFound {
        return eyre!("failed to start the serve command `{program}`: {err}");
    }

    eyre!(
        "serve command `{program}` was not found on PATH.\n\n\
         `wanshi serve` delegates to a static file server, chosen by \
         `[serve].command` in \"Wanshi.toml\". Either:\n  \
         install it — `brew install {program}` or `cargo install {program}`\n  \
         point `[serve].command` at a server you already have, for example\n    \
         command = [\"python3\", \"-m\", \"http.server\", \"8080\", \"-d\", \"<output>\"]\n  \
         or run `wanshi serve --no-server` to build and watch without serving,\n    \
         and serve the output directory yourself."
    )
}

pub(super) fn spawn_serve_process() -> eyre::Result<std::process::Child> {
    let command = environment::serve_command();
    let program = command.first().cloned().unwrap_or_default();
    let mut serve = parse_command(&command, environment::output_dir())?
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|err| explain_spawn_failure(&program, err))?;

    if let Some(serve_stdout) = serve.stdout.take() {
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(serve_stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => println!("[serve] {line}"),
                    Err(err) => {
                        color_print::ceprintln!("<r>[serve] stdout read error: {err}</>");
                        break;
                    }
                }
            }
        });
    }

    if let Some(serve_stderr) = serve.stderr.take() {
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(serve_stderr);
            for line in reader.lines() {
                match line {
                    Ok(line) => color_print::ceprintln!("<r>[serve] Error: {line}</>"),
                    Err(err) => {
                        color_print::ceprintln!("<r>[serve] stderr read error: {err}</>");
                        break;
                    }
                }
            }
        });
    }

    Ok(serve)
}

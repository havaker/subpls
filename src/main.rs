mod user;
use user::*;

use clap::Arg;
use colored::*;
use std::process;

mod subs;

fn main() {
    let matches = clap::App::new("Subpls")
        .version("1.0")
        .author("Michał Sala <0havaker@gmail.com>")
        .about("Download subtitles from OpenSubtitles")
        .arg(
            Arg::with_name("username")
                .short("u")
                .long("username")
                .alias("login")
                .value_name("USERNAME")
                .help("Sets OpenSubtitles username")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("password")
                .short("p")
                .long("password")
                .value_name("PASSWORD")
                .help("Sets OpenSubtitles password used to login")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("language")
                .short("l")
                .long("language")
                .value_name("LANGUAGE")
                .help("ISO639 2 letter code, 'en' e.g.")
                .default_value("en")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("FILE")
                .required(true)
                .multiple(true)
                .index(1),
        )
        .get_matches();

    let files: Vec<&str> = matches.values_of("FILE").unwrap().collect();

    let mut username = String::new();
    let mut password = String::new();

    if let Some(u) = matches.value_of("username") {
        username = u.to_string();
    }
    if let Some(p) = matches.value_of("password") {
        password = p.to_string();
    }

    if !username.is_empty() && password.is_empty() {
        if let Ok(p) = rpassword::prompt_password_stdout("password: ") {
            password = p;
        }
    }

    let user = User::login(
        &username,
        &password,
        matches.value_of("language").unwrap_or("en"),
    );

    if let Err(s) = user {
        eprintln!("{} {}", "error: ".red(), s);
        process::exit(1);
    }
    let user = user.unwrap();

    let msg = "logged in successfully".green();
    println!(
        "{}{}",
        msg,
        if username.is_empty() {
            " (as an anonymous user)"
        } else {
            ""
        }
    );

    let subs = user.search(files.into_iter().map(|x| x.to_string()).collect());
    println!(
        "found {} subtitle{}",
        subs.len(),
        if subs.len() == 1 { "" } else { "s" }
    );

    if subs.is_empty() {
        process::exit(2);
    }

    match user.download(subs) {
        Ok(count) => {
            let msg = format!(
                "downloaded {} subtitle{}!",
                count,
                if count == 1 { "" } else { "s" }
            );
            println!("{}", msg.green())
        }
        Err(e) => println!("{} {}", "downloading failed:".red(), e.to_string()),
    }
}

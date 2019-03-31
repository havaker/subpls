use clap::Arg;
use colored::*;
use std::process;

mod subs;
use subs::*;

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
                .help("SubLanguageID, 'eng' e.g.")
                .default_value("eng")
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
        eprintln!("{} {:?}", "error: ".red(), s);
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

    let mut movies = Movie::collection(&files);

    for movie in &mut movies {
        if let Err(e) = movie.compute_os_hash() {
            eprintln!(
                "{} {} ({:?})",
                "could not compute hash for: ".red(),
                movie.filename,
                e
            );
        }
    }

    let mut search_result = user.search(movies);
    if let Err(e) = search_result {
        eprintln!("{} ({:?})", "could not search for subtitles ".red(), e);
        std::process::exit(1);
    }
    let mut movies = search_result.unwrap();

    for movie in &mut movies {
        println!(
            "found {} subtitles for {}",
            movie.subs.len(),
            movie.filename
        );
        movie.filter_subs();
        if let Some(rating) = movie.present_rating() {
            println!(
                "  choosing ones with rating: {}/10{}",
                rating,
                if rating > 0.0 { "" } else { " (unrated)" }
            );
        }
    }

    let download_result = user.download(movies);
    if let Err(e) = download_result {
        eprintln!("{} ({:?})", "could not download subtitles ".red(), e);
        std::process::exit(1);
    }
    let mut movies = download_result.unwrap();
    println!("{}", "download completed successfully".green());

    let mut ok = 0;
    for movie in &movies {
        if let Err(e) = movie.save_subs() {
            eprintln!(
                "{} {} {} ({:?})",
                "saving subtitles for".red(),
                movie.filename,
                "failed".red(),
                e
            );
        } else {
            ok += 1;
        }
    }
    if ok > 0 {
        println!(
            "{} {} {}{}",
            "saved".green(),
            ok,
            "subtitle".green(),
            (if ok == 1 { "" } else { "s" }).green()
        );
    }
}

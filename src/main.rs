#[macro_use]
extern crate nom;
extern crate clap;

mod parser;

use std::fs::File;
use std::io;
use std::io::{BufReader, BufRead};
use std::process;

use clap::{Arg, App, ArgMatches};
use nom::IResult::{Error, Done, Incomplete};

use parser::{CrontabParserOptions, parse_crontab, walk_errors};


fn parse_args<'a>() -> ArgMatches<'a> {
    App::new("crontabcheck")
        .about("Check a crontab file (read from stdin)")
        .arg(
            Arg::with_name("allowed-usernames")
                .short("u")
                .takes_value(true)
                .use_delimiter(true)
                .default_value("")
                .help("Comma-separated list of valid usernames (may be specified multiple times).")
        )
        .arg(
            Arg::with_name("passwd-usernames")
                .short("p")
                .help("Read valid usernames from /etc/passwd")
        ).get_matches()
}

fn main() {
    process::exit(run());
}

fn run() -> i32 {
    let matches = parse_args();
    let mut allowed_usernames: Vec<String> = matches.values_of("allowed-usernames").unwrap().map(|s| s.to_string()).collect();
    if matches.is_present("passwd-usernames") {
         match usernames_from_etc_passwd() {
            Ok(more_usernames) => allowed_usernames.extend(more_usernames),
            Err(e) => { println!("could not read usernames from /etc/passwd: {}", e); return 2; }
         }
    }
    let options = CrontabParserOptions {
        allowed_usernames: Some(&allowed_usernames[..])
    };
    let stdin = io::stdin();
    for input in stdin.lock().lines() {
        let line = match input {
            Ok(line) => line,
            Err(what) => { println!("could no read from stdin: {:?}", what); return 2; }
        };
        let out = parse_crontab(line.as_bytes(), &options);
        match out {
            Done(..) => (),
            Incomplete(_) => { println!("Invalid line: {} (incomplete crontab)", line); return 1; },
            Error(err) => { println!("Invalid line: {}\n{}", line, walk_errors(&[err])); return 1; }
        }
    }
    0
}


fn usernames_from_etc_passwd() -> Result<Vec<String>, io::Error> {
    let file = BufReader::new(File::open("/etc/passwd")?);
    let mut usernames: Vec<String> = vec![];
    for line in file.lines() {
        usernames.push(
            line?.split(':').nth(0)
            .ok_or(io::Error::new(io::ErrorKind::InvalidData, "invalid /etc/passwd format"))?
            .trim().to_string());
    }
    Ok(usernames)
}

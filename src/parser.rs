use std::str::from_utf8;
use std::fmt;

use nom;
use nom::{IResult, ErrorKind, digit, space, alphanumeric, is_space};
use nom::IResult::{Error, Done, Incomplete};


#[derive(PartialEq)]
#[derive(Debug)]
#[derive(Clone)]
pub enum CrontabSyntaxError {
    InvalidEnumField,
    ValueOutOfBounds { value: i32, min: i32, max: i32 },
    InvalidNumericValue,
    InvalidPeriodField,
    InvalidFieldSeparator,
    InvalidUsername,
    InvalidCommandLine { reason: String },
}

impl fmt::Display for CrontabSyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CrontabSyntaxError::ValueOutOfBounds { value, min, max } => write!(f, "value {} out of bounds (accepted: {} to {})", value, min, max),
            CrontabSyntaxError::InvalidEnumField | CrontabSyntaxError::InvalidPeriodField => write!(f, "could not parse the field"),
            CrontabSyntaxError::InvalidNumericValue => write!(f, "invalid numeric value"),
            CrontabSyntaxError::InvalidFieldSeparator => write!(f, "expected a field separator (space or tab)"),
            CrontabSyntaxError::InvalidUsername => write!(f, "invalid username"),
            CrontabSyntaxError::InvalidCommandLine { ref reason } => write!(f, "invalid command line: {}", reason),
        }
    }
}


fn parse_within_bounds(input: &[u8], min: i32, max: i32) -> IResult<&[u8], (), CrontabSyntaxError> {
    let digits = digit(input);
    match digits {
        Done(remaining, result)  => {
            from_utf8(result).ok().and_then(|x|
                x.parse::<i32>().ok()
            ).and_then(|int|
                if int < min || int > max {
                    Some(Error(error_position!(
                        ErrorKind::Custom(CrontabSyntaxError::ValueOutOfBounds { value: int, min: min, max: max }),
                        input
                    )))
                } else {
                    Some(Done(remaining, ()))
                }
            ).unwrap_or(
                Error(error_position!(ErrorKind::Custom(CrontabSyntaxError::InvalidNumericValue), input))
            )
        },
        Error(..) => {
            Error(error_position!(ErrorKind::Custom(CrontabSyntaxError::InvalidNumericValue), input))
        },
        Incomplete(n) => Incomplete(n)
    }
}

// Basic values parsers (a value is either a day or month name ("mon", "jun") or a bounded integer ("2"),
named!(minute_value_parser<&[u8], (), CrontabSyntaxError>, apply!(parse_within_bounds, 0, 59));
named!(hour_value_parser<&[u8], (), CrontabSyntaxError>, apply!(parse_within_bounds, 0, 24));
named!(day_of_month_value_parser<&[u8], (), CrontabSyntaxError>, apply!(parse_within_bounds, 0, 31));

fn month_value_parser(input: &[u8]) -> IResult<&[u8], (), CrontabSyntaxError> {
    let parsed =
        fix_error!(input, CrontabSyntaxError,
            alt_complete!(
                tag!("jan")
                | tag!("feb")
                | tag!("mar")
                | tag!("apr")
                | tag!("may")
                | tag!("jun")
                | tag!("jul")
                | tag!("aug")
                | tag!("sep")
                | tag!("oct")
                | tag!("nov")
                | tag!("dec")
            )
        );
    match parsed {
        Done(i, _) => Done(i, ()),
        Incomplete(inc) => Incomplete(inc),
        Error(..) => parse_within_bounds(input, 1, 12)
    }
}

fn day_of_week_value_parser(input: &[u8]) -> IResult<&[u8], (), CrontabSyntaxError> {
    let parsed =
        fix_error!(input, CrontabSyntaxError,
            alt_complete!(
                tag!("mon")
                | tag!("tue")
                | tag!("wed")
                | tag!("thu")
                | tag!("fri")
                | tag!("sat")
                | tag!("sun")
            )
        );
    match parsed {
        Done(i, _) => Done(i, ()),
        Incomplete(inc) => Incomplete(inc),
        Error(..) => parse_within_bounds(input, 0, 7)
    }
}

// parse '*/2'
fn parse_period(input: &[u8], value_parser: fn(&[u8]) -> IResult<&[u8], (), CrontabSyntaxError>) -> IResult<&[u8], (), CrontabSyntaxError> {
    let out = tag!(input, "*");
    match out {
       Done(i, _) => {
            let next = tag!(i, "/");
            match next {
                Done(ii, _) => add_return_error!(ii, ErrorKind::Custom(CrontabSyntaxError::InvalidPeriodField), value_parser),
                _ => Done(i, ()),
            }
        },
       _ => Error(error_position!(ErrorKind::Custom(CrontabSyntaxError::InvalidPeriodField), input))
    }
}

fn parse_range_or_value(input: &[u8], value_parser: fn(&[u8]) -> IResult<&[u8], (), CrontabSyntaxError>) -> IResult<&[u8], (), CrontabSyntaxError> {
    let parsed_value = value_parser(input);
    match parsed_value {
        Error(..) | Incomplete(..) => parsed_value,
        Done(i, _) => {
            let separator = fix_error!(i, CrontabSyntaxError, tag!("-"));
            match separator {
                Error(..) => parsed_value,
                Incomplete(inc) => Incomplete(inc),
                Done(ii, _) => value_parser(ii)
            }
        }
    }
}

// parse 2,12-23
fn parse_enum(input: &[u8], value_parser: fn(&[u8]) -> IResult<&[u8], (), CrontabSyntaxError>) -> IResult<&[u8], (), CrontabSyntaxError> {
    add_return_error!(input, ErrorKind::Custom(CrontabSyntaxError::InvalidEnumField),
        do_parse!(
            separated_nonempty_list!(tag!(","), apply!(parse_range_or_value, value_parser)) >>
            ()
        )
    )
}

// a field is either a frequency (*/2) or an enumeration (2-4,5)
fn parse_field(input: &[u8], value_parser: fn(&[u8]) -> IResult<&[u8], (), CrontabSyntaxError>) -> IResult<&[u8], (), CrontabSyntaxError> {
    match peek!(input, tag!("*")) {
        IResult::Error(..) => apply!(input, parse_enum, value_parser),
        IResult::Done(..) => apply!(input, parse_period, value_parser),
        Incomplete(e) => Incomplete(e)
    }
}


fn parse_field_separator(input: &[u8]) -> IResult<&[u8], (), CrontabSyntaxError> {
    let parsed = space(input);
    match parsed {
        Done(i, _) => Done(i, ()),
        Error(_) => Error(error_position!(ErrorKind::Custom(CrontabSyntaxError::InvalidFieldSeparator), input)),
        Incomplete(i) => Incomplete(i)
    }
}

fn is_valid_username<T: AsRef<str>>(name: &str, allowed_usernames: Option<&[T]>) -> bool {
    if allowed_usernames.is_none() {
        return true;
    }
    (*(allowed_usernames.unwrap())).iter().any(|el| el.as_ref() == name)
}


fn parse_user<'a, 'b, T: AsRef<str> + 'b>(input: &'a[u8], allowed_usernames: Option<&'b[T]>) -> IResult<&'a[u8], (), CrontabSyntaxError> {
    let parsed = alphanumeric(input);
    match parsed {
        Done(i, o) => {
            from_utf8(o).ok().map(|name| is_valid_username(name, allowed_usernames)).map(|valid|
            if valid {
                Done(i, ())
            } else {
                Error(error_position!(ErrorKind::Custom(CrontabSyntaxError::InvalidUsername), input))
            }).unwrap_or(Error(error_position!(ErrorKind::Custom(CrontabSyntaxError::InvalidUsername), input)))
        },
        Error(_) => Error(error_position!(ErrorKind::Custom(CrontabSyntaxError::InvalidUsername), input)),
        Incomplete(i) => Incomplete(i)
    }
}

pub struct CrontabParserOptions<'a, T: AsRef<str> + 'a> {
    pub allowed_usernames: Option<&'a [T]>
}

// consume all input, make sure there are not special characters in the command line
fn parse_command_line(input: &[u8]) -> IResult<&[u8], (), CrontabSyntaxError> {
    // cron limitation
    // see https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=686223
    if (*input).len() > 999 {
        return Error(error_position!(ErrorKind::Custom(
            CrontabSyntaxError::InvalidCommandLine { reason: "command line can not exceed 999 characters".to_string() }),
            input
        ))
    }
    for &c in input {
        if c == b'%' {
            let reason = format!("special char {} should not be used", c as char);
            return Error(error_position!(ErrorKind::Custom(
                CrontabSyntaxError::InvalidCommandLine { reason }),
                input
            ))
        }
    }
    Done(&[], ())
}

fn parse_comment(input: &[u8]) -> IResult<&[u8], (), CrontabSyntaxError> {
    let out = fix_error!(input, CrontabSyntaxError, do_parse!(
        take_while_s!(is_space) >>
        tag!("#") >>
        ()
    ));
    match out {
        Done(..) => Done(&[], ()),
        Error(e) => Error(e),
        Incomplete(e) => Incomplete(e)
    }
}

// TODO: more checks for this parser
fn parse_environnment_variable(input: &[u8]) -> IResult<&[u8], (), CrontabSyntaxError> {
    let out = fix_error!(input, CrontabSyntaxError,
            do_parse!(
                alphanumeric >>
                tag!("=") >>
                ()
            ));
    match out {
        Done(..) => Done(&[], ()),
        Error(e) => Error(e),
        Incomplete(e) => Incomplete(e)
    }

}

fn parse_empty_line(input: &[u8]) -> IResult<&[u8], (), CrontabSyntaxError> {
    for c in input {
        if !is_space(*c) {
            return Error(error_position!(ErrorKind::Space, input));
        }
    }
    return Done(&[], ());
}

// TODO: the caller should not have to depend on symbols exported by nom
pub fn parse_crontab<'a, T: AsRef<str>>(input: &'a[u8], options: &CrontabParserOptions<T>) -> IResult<&'a[u8], (), CrontabSyntaxError> {
    // We do not use the alt_complete! combinator because we want to have nice error codes
    // Try to parse the line as an empty line, then if it fails as a comment, then as an
    // environment variable assignation, then as an actual crontab line
    let mut result = parse_empty_line(input);
    if let Done(..) = result {
        return result;
    }
    result = parse_comment(input);
    if let Done(..) = result {
        return result;
    }
    result = parse_environnment_variable(input);
    if let Done(..) = result {
        return result;
    }

    // actual crontab line
    return do_parse!(input,
        apply!(parse_field, minute_value_parser) >>
        parse_field_separator >>
        apply!(parse_field, hour_value_parser) >>
        parse_field_separator >>
        apply!(parse_field, day_of_month_value_parser) >>
        parse_field_separator >>
        apply!(parse_field, month_value_parser) >>
        parse_field_separator >>
        apply!(parse_field, day_of_week_value_parser) >>
        parse_field_separator >>
        apply!(parse_user, options.allowed_usernames) >>
        parse_field_separator >>
        parse_command_line >>
        ()
    )
}


fn format_error(error: &ErrorKind<CrontabSyntaxError>) -> String {
    match *error {
        ErrorKind::Custom(ref e) => e.to_string(),
        ref e => format!("error: {:?}", e).to_string()  // this should not happen
    }
}

pub fn walk_errors(errs: &[nom::Err<&[u8], CrontabSyntaxError>]) -> String {
    let mut strings: Vec<String> = vec![];
    for err in errs {
        let formatted = match *err {
            nom::Err::Code(ref kind) => format_error(kind),
            nom::Err::Node(ref kind, ref next_error) => format_error(kind) + "\nCaused by: " + &walk_errors(next_error),
            nom::Err::Position(ref kind, position) => format_error(kind) + " (at '" + format_position(position).as_str() + "')",
            nom::Err::NodePosition(ref  kind, position, ref next_error) => format_error(kind) + " (at '" + format_position(position).as_str() + "')\nCaused by: " + &walk_errors(next_error),
        };
        strings.push(formatted);
    }
    strings.join("\n\n")
}

fn format_position(pos: &[u8]) -> String {
    let mut s = from_utf8(pos).unwrap_or("(invalid UTF-8)").to_string();
    s.truncate(15);
    s
}

#[cfg(test)]
mod tests {

    use nom::IResult::{Error, Done};
    use parser::*;

    #[test]
    fn test_format_errors() {
        let usernames = ["root"];
        let options = &CrontabParserOptions { allowed_usernames: Some(&usernames) };
        let parsed = parse_crontab("2-10 * */4 * mon  root /usr/local/bin yay".as_bytes(), options);
        match parsed {
            Error(e) => {
                let errors = [e];
                println!("{}", walk_errors(&errors));
            },
            _ =>  ()
        };
    }

    #[test]
    fn test_parse_valid_crontab() {
        let usernames = ["root"];
        let options = &CrontabParserOptions { allowed_usernames: Some(&usernames) };
        let out = parse_crontab("* * * * * root /usr/local/bin yay".as_bytes(), options);
        assert_eq!(out, Done("".as_bytes(), ()));

        let out = parse_crontab("8 * * * * root /usr/local/bin yay".as_bytes(), options);
        assert_eq!(out, Done("".as_bytes(), ()));

        let out = parse_crontab("*/3 2 * * * root /usr/local/bin yay".as_bytes(), options);
        assert_eq!(out, Done("".as_bytes(), ()));

        let out = parse_crontab("1-2 * * * * root /usr/local/bin yay".as_bytes(), options);
        assert_eq!(out, Done("".as_bytes(), ()));

        let out = parse_crontab("1-2 * * * mon,tue root /usr/local/bin yay".as_bytes(), options);
        assert_eq!(out, Done("".as_bytes(), ()));

        let out = parse_crontab("#This is a comment".as_bytes(), options);
        assert_eq!(out, Done("".as_bytes(), ()));

        let out = parse_crontab("VARIABLE=VALUE".as_bytes(), options);
        assert_eq!(out, Done("".as_bytes(), ()));

        let out = parse_crontab("   ".as_bytes(), options);
        assert_eq!(out, Done("".as_bytes(), ()));
    }

    #[test]
    fn test_parse_user() {
        assert_eq!(parse_user("whatever".as_bytes(), None as Option<&[String]>), Done("".as_bytes(), ()));
        let users = ["root"];
        match  parse_user("whatever".as_bytes(), Some(&users)) {
            Error(_) => (),
            _ => assert!(false)
        };
        assert_eq!(parse_user("root /usr/bin/local".as_bytes(), None as Option<&[String]>), Done(" /usr/bin/local".as_bytes(), ()));
    }

    #[test]
    fn test_is_valid_username() {
        assert_eq!(true, is_valid_username("whatever", None as Option<&[String]>));
        assert_eq!(true, is_valid_username("root", Some(&["root", "notroot"])));
        assert_eq!(false, is_valid_username("bfaucon", Some(&["root", "notroot"])));
    }

    #[test]
    fn test_day_of_week_value_parser() {
        assert_eq!(day_of_week_value_parser("mon".as_bytes()), Done("".as_bytes(), ()));
        assert_eq!(day_of_week_value_parser("mon ".as_bytes()), Done(" ".as_bytes(), ()));
        assert_eq!(day_of_week_value_parser("0 ".as_bytes()), Done(" ".as_bytes(), ()));
        assert_eq!(day_of_week_value_parser("1 ".as_bytes()), Done(" ".as_bytes(), ()));
    }

    #[test]
    fn test_parse_period() {
        assert_eq!(parse_period("* ".as_bytes(), minute_value_parser), Done(" ".as_bytes(), ()));
        assert_eq!(parse_period("*/2 ".as_bytes(), minute_value_parser), Done(" ".as_bytes(), ()));
    }

    #[test]
    fn test_parse_range_or_value() {
        assert_eq!(parse_range_or_value("1-2".as_bytes(), minute_value_parser), Done("".as_bytes(), ()));
    }

    #[test]
    fn test_parse_enum() {
        assert_eq!(parse_enum("1-2,3,4-5 *".as_bytes(), minute_value_parser), Done(" *".as_bytes(), ()));
        assert_eq!(parse_enum("mon-tue ".as_bytes(), day_of_week_value_parser), Done(" ".as_bytes(), ()));
    }

    #[test]
    fn test_parse_field() {
        assert_eq!(parse_field("mon-tue ".as_bytes(), day_of_week_value_parser), Done(" ".as_bytes(), ()));
    }

}

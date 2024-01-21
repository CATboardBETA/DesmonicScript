use crate::parser::Expr;
use ariadne::{Report, ReportKind, Source, Span};
use chumsky::error::Simple;
use chumsky::prelude::Parser;
use rocket::serde::json::serde_json::json;
use rocket::serde::json::Value;
use rocket::{get, routes, Error, Ignite, Rocket};
use std::ops::Range;
use std::process::exit;
use std::sync::Mutex;
use crate::compile::compile;

static DATA_OUT: Mutex<Value> = Mutex::new(json! {[]});

mod compile;
mod parser;

fn main() {
    // start_server().unwrap();

    let src_f = std::env::args().nth(1).unwrap();
    let src_f = src_f.as_str();
    let src = std::fs::read_to_string(src_f).unwrap();
    let src = src.as_str();

    let mut tok = Expr::Num(0.);
    match parser::parser().parse(src) {
        Ok(tk) => { tok=tk.clone(); println!("{tk:?}") },
        Err(e) => {
            for err in e {
                Report::<(&str, Range<usize>)>::build(ReportKind::Error, src_f, err.span().start())
                    .with_message(&format!(
                        "Expected {:?}, but fount {:?}!",
                        err.expected().collect::<Vec<_>>(),
                        err.found().unwrap_or(&'Ø')
                    ))
                    .finish()
                    .print((src_f, Source::from(src)))
                    .unwrap();
            }
            exit(1)
        }
    }

    println!("'{}'", compile(&tok, &vec![]).unwrap());
}

#[allow(dead_code)]
#[get("/data")]
fn data() -> Value {
    DATA_OUT.lock().unwrap().clone()
}

fn start_server() -> Result<Rocket<Ignite>, Error> {
    rocket::execute(rocket::build().mount("/", routes![data]).launch())
}

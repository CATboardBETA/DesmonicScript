use crate::compile::compile;
use crate::parser::Expr;
use ariadne::{Report, ReportKind, Source, Span};
use chumsky::error::Simple;
use chumsky::prelude::Parser;
use lazy_static::lazy_static;
use rocket::serde::json::serde_json::json;
use rocket::serde::json::Value;
use rocket::yansi::Paint;
use rocket::{get, routes, Error, Ignite, Rocket};
use std::ops::Range;
use std::process::exit;
use std::sync::Mutex;

static DATA_OUT: Mutex<Value> = Mutex::new(json! {[]});

mod compile;
mod parser;

lazy_static! {
    static ref SRC_F: String = std::env::args().nth(1).unwrap();
}

fn main() {
    let src_f = SRC_F.as_str();
    let src = std::fs::read_to_string(src_f).unwrap();
    let src = src.as_str();

    let mut tok = Expr::Num(0.);
    match parser::parser().parse(src) {
        Ok(tk) => {
            tok = tk.clone();
            println!("{tk:?}")
        }
        Err(e) => {
            for err in e {
                Report::<(&str, Range<usize>)>::build(ReportKind::Error, src_f, err.span().start())
                    .with_message(&format!(
                        "Expected {:?}, but fount {:?}!",
                        err.expected().collect::<Vec<_>>(),
                        err.found().unwrap_or(&'Ø')
                    ))
                    .finish()
                    .eprint((src_f, Source::from(src)))
                    .unwrap();
            }
            exit(1)
        }
    }

    println!("{:#?}", compile(&tok, &vec![]).unwrap());

    // start_server().unwrap();
}

#[allow(dead_code)]
#[get("/data")]
fn data() -> Value {
    DATA_OUT.lock().unwrap().clone()
}

fn start_server() -> Result<Rocket<Ignite>, Error> {
    rocket::execute(rocket::build().mount("/", routes![data]).launch())
}

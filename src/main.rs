#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]

use crate::compile::compile;
use crate::compile::graph_state::ToGraphStateJson;
use ariadne::{Label, Report, ReportKind, Source, Span};
use chumsky::prelude::Parser;
use lazy_static::lazy_static;
use rocket::response::Responder;
use rocket::serde::json::serde_json::json;
use rocket::serde::json::Value;
use rocket::{get, routes, Error, Ignite, Request, Response, Rocket};
use std::process::exit;
use std::sync::Mutex;

static DATA_OUT: Mutex<Value> = Mutex::new(json! {[]});

mod compile;
mod parser;

lazy_static! {
    static ref SRC_F: String = std::env::args().nth(1).unwrap();
}

fn  main() {
    let src_f = SRC_F.as_str();
    let src = std::fs::read_to_string(src_f).unwrap();
    let src = src.as_str();

    let mut tok;
    match parser::parser().parse_recovery(src) {
        (Some(tk), e) => {
            for err in e {
                Report::build(ReportKind::Error, src_f, err.span().start())
                    .with_message(&format!(
                        "Expected {:?}, but found {:?}!",
                        err.expected().collect::<Vec<_>>(),
                        err.found().unwrap_or(&'Ø')
                    ))
                    .with_label(Label::new((src_f, err.span())).with_message("Found here"))
                    .finish()
                    .eprint((src_f, Source::from(src)))
                    .unwrap();
            }
            tok = tk.clone();
        }
        (None, e) => {
            for err in e {
                Report::build(ReportKind::Error, src_f, err.span().start())
                    .with_message(&format!(
                        "Expected {:?}, but found {:?}!",
                        err.expected().collect::<Vec<_>>(),
                        err.found().unwrap_or(&'Ø')
                    ))
                    .with_label(Label::new((src_f, err.span())).with_message("Found here"))
                    .finish()
                    .eprint((src_f, Source::from(src)))
                    .unwrap();
            }
            println!("Cannot recover! Exiting...");
            exit(1)
        }
    }

    println!("{tok:#?}");

    let compiled = compile(
        &mut tok,
        &mut ["x", "y", "t"].iter().map(ToString::to_string).collect(),
        &mut vec![],
        None,
    )
    .unwrap();
    *DATA_OUT.lock().unwrap() = compiled.into_graph_state();
    start_server().unwrap();
}

#[allow(dead_code)]
#[get("/data")]
fn data() -> GraphStateResponse<Value> {
    GraphStateResponse(DATA_OUT.lock().unwrap().to_owned())
}

struct GraphStateResponse<R>(pub R);

impl<'r, 'o: 'r, R: Responder<'r, 'o>> Responder<'r, 'o> for GraphStateResponse<R> {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'o> {
        Response::build_from(self.0.respond_to(req)?)
            .raw_header("Access-Control-Allow-Origin", "*")
            .ok()
    }
}

fn start_server() -> Result<Rocket<Ignite>, Error> {
    rocket::execute(rocket::build().mount("/", routes![data]).launch())
}

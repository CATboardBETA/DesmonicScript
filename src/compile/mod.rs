use crate::parser::Expr;
use crate::SRC_F;
use ariadne::{Report, ReportKind, Source};
use rocket::form::validate::Contains;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::ops::{Deref, Range};
use std::sync::Mutex;

pub mod graph_state;

static CURRENT_ID: Mutex<u32> = Mutex::new(0);

static BUILTINS: &[&str] = &["sin"];

#[derive(Clone, Debug)]
pub struct Latex {
    pub inner: String,
    pub folder_id: Option<String>,
    pub id: String,
}

pub fn compile(
    expr: &Expr,
    vars: &mut Vec<String>,
    funcs: &mut HashMap<String, Expr>,
    mut fold_id: Option<u32>,
) -> Result<Vec<Latex>, String> {
    let mut all_latex = vec![];

    let mut latex = "".to_owned();

    match expr {
        Expr::Num(val) => latex.push_str(&val.to_string()),
        Expr::Call(name, params) => latex.push_str(&format!(
            r"{}\left({}\right)",
            if funcs.keys().collect::<Vec<_>>().contains(name) {
                subscriptify(name)
            } else if BUILTINS.contains(&name.as_str()) {
                operatorname(name)
            } else {
                return Err(format!("Function '{name}' does not exist!"));
            },
            display_params(params, vars, funcs, fold_id)?
        )),
        Expr::Var(name) => {
            latex.push_str(&subscriptify(name));
            if !vars.contains(name) {
                Report::<(&str, Range<usize>)>::build(ReportKind::Error, SRC_F.as_str(), 0)
                    .with_message(format!("Variable '{name}' is undefined."))
                    .finish()
                    .eprint((
                        SRC_F.as_str(),
                        Source::from(read_to_string(SRC_F.clone()).unwrap()),
                    ))
                    .unwrap();
            }
        }
        Expr::Neg(ex) => latex.push_str(&format!("-{}", &compile1(ex, vars, funcs, fold_id)?)),
        Expr::Mul(ex1, ex2) => latex.push_str(&format!(
            r"{}\cdot {}",
            compile1(ex1, vars, funcs, fold_id)?,
            compile1(ex2, vars, funcs, fold_id)?
        )),
        Expr::Div(ex1, ex2) => latex.push_str(&format!(
            r"\frac{{{}}}{{{}}}",
            compile1(ex1, vars, funcs, fold_id)?,
            compile1(ex2, vars, funcs, fold_id)?
        )),
        Expr::Add(ex1, ex2) => latex.push_str(&format!(
            r"\left({}+{}\right)",
            compile1(ex1, vars, funcs, fold_id)?,
            compile1(ex2, vars, funcs, fold_id)?
        )),
        Expr::Sub(ex1, ex2) => latex.push_str(&format!(
            r"\left({}-{}\right)",
            compile1(ex1, vars, funcs, fold_id)?,
            compile1(ex2, vars, funcs, fold_id)?
        )),
        Expr::Def { left, right, then } => {
            latex.push_str(&format!(
                r"{}={}",
                compile1(left, vars, funcs, fold_id)?,
                compile1(right, vars, funcs, fold_id)?
            ));
            // If it is just a variable on the lhs, set that var as defined.
            if let Expr::Var(name) = right.deref() {
                vars.push(name.to_string());
            }
            if let Some(then) = then {
                all_latex.push(Latex {
                    inner: latex.clone(),
                    folder_id: None,
                    id: gen_id().to_string(),
                });
                for expr in compile(then, vars, funcs, fold_id)? {
                    all_latex.push(expr);
                }
            }
        }

        Expr::Fol { title, body, then } => {
            if fold_id.is_some() {
                return Err("Cannot create a folder inside a folder!".to_owned());
            }

            fold_id = Some(gen_id());

            all_latex.push(Latex {
                inner: format!("\\folder {title}"),
                folder_id: None,
                id: fold_id.unwrap().to_string(),
            });

            for expr in body {
                for exp in compile(expr, vars, funcs, fold_id)? {
                    all_latex.push(Latex {
                        inner: exp.inner,
                        folder_id: fold_id.map(|x| x.to_string()),
                        id: gen_id().to_string(),
                    });
                }
            }

            if let Some(then) = then {
                fold_id = None;

                for expr in compile(then, vars, funcs, fold_id)? {
                    all_latex.push(expr);
                }
            }
        }
    }

    if !latex.is_empty() {
        all_latex.push(Latex {
            inner: latex.clone(),
            folder_id: None,
            id: gen_id().to_string(),
        });
        latex.clear();
    }
    Ok(all_latex)
}

fn operatorname(op: &String) -> String {
    format!("\\operatorname{{{op}}}")
}

// Compile, and if theres more than one latex output, output an error.
fn compile1(
    expr: &Expr,
    vars: &mut Vec<String>,
    funcs: &mut HashMap<String, Expr>,
    fold_id: Option<u32>,
) -> Result<String, String> {
    let comp = compile(expr, vars, funcs, fold_id)?;

    let len = comp.len();
    if len != 1 {
        Err(format!("Expected only one expression. Got {len}"))
    } else {
        // Unwrap is safe here since we checked above than the length is 1
        Ok(comp.first().unwrap().clone().inner)
    }
}

fn display_params(
    params: &Vec<Expr>,
    vars: &mut Vec<String>,
    funcs: &mut HashMap<String, Expr>,
    fold_id: Option<u32>,
) -> Result<String, String> {
    let mut out = "".to_owned();

    for (i, param) in params.iter().enumerate() {
        let param = compile1(param, vars, funcs, fold_id)?;
        if i < params.len() - 1 {
            out.push_str(&format!("{param},"))
        } else {
            out.push_str(&param)
        }
    }

    Ok(out)
}

fn subscriptify(ident: &str) -> String {
    let (first, rest) = ident.split_at(1);

    if rest.is_empty() {
        first.to_owned()
    } else {
        format!("{first}_{{{rest}}}")
    }
}

/// Generate a new id in `CURRENT_ID`, and output it.
fn gen_id() -> u32 {
    *CURRENT_ID.lock().unwrap() += 1;
    *CURRENT_ID.lock().unwrap()
}

use crate::parser::Expr;
use crate::SRC_F;
use ariadne::{Report, ReportKind, Source};
use std::collections::HashMap;
use std::fs::read_to_string;
use std::ops::{Deref, Range};
use std::sync::Mutex;
use chumsky::chain::Chain;

pub mod graph_state;

static CURRENT_ID: Mutex<u32> = Mutex::new(0);

static BUILTINS: &[&str] = &["sin"];

#[derive(Clone, Debug)]
pub struct Latex {
    pub inner: String,
    pub folder_id: Option<String>,
    pub id: String,
}

//noinspection ALL
pub fn compile(
    expr: &mut Expr,
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
            if funcs.keys().collect::<Vec<_>>().contains(&name.deref()) {
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
            // If it is just a variable on the lhs, set that var as defined.
            if let Expr::Var(name) = *left.clone() {
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
            latex.push_str(&format!(
                r"{}={}",
                compile1(left, vars, funcs, fold_id)?,
                compile1(right, vars, funcs, fold_id)?
            ));
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
        Expr::Fun {
            name,
            args,
            body,
            then,
        } => {
            let fold_id = gen_id();

            all_latex.push(Latex {
                inner: format!("\\folder {name}"),
                folder_id: None,
                id: fold_id.to_string(),
            });

            let mut our_vars = vars.clone();
            our_vars.append(args);

            let iter = ExprIterator(*body.clone(), 0);
            let count = iter.clone().count();
            for (i, mut body_item) in iter.enumerate() {
                match body_item {
                    Expr::Def { .. } => all_latex.push(Latex {
                        inner: compile1(&mut body_item, &mut our_vars, funcs, Some(fold_id))?,
                        folder_id: Some(fold_id.to_string()),
                        id: gen_id().to_string()
                    }),
                    Expr::Fol { .. } => panic!("Cannot have a folder inside of a function!"),
                    Expr::Fun { .. } => panic!("Cannot have a function inside of a function (for now)!"),
                    _ if i == count => all_latex.push(Latex {
                        inner: compile1(&mut body_item, &mut our_vars, funcs, Some(fold_id))?,
                        folder_id: Some(fold_id.to_string()),
                        id: gen_id().to_string()
                    }),
                    _ => panic!("Only final expression may be a non-def.")
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

#[derive(Debug, Clone)]
pub struct ExprIterator(Expr, usize);

impl Iterator for ExprIterator {
    type Item = Expr;

    fn next(&mut self) -> Option<Self::Item> {

        if self.1 == 0 { return Some(self.0.clone()) }

        let mut n = 1;
        let mut current = match self.0.clone() {
            Expr::Def { then, .. } => then.clone(),
            Expr::Fol { then, .. } => then.clone(),
            Expr::Fun { then, .. } => then.clone(),
            _ => None,
        }?;
        while n <= self.1 {
            current = match *current {
                Expr::Def { then, .. } => then.clone(),
                Expr::Fol { then, .. } => then.clone(),
                Expr::Fun { then, .. } => then.clone(),
                _ => None,
            }?;
            n += 1;
            dbg!(n);
        }

        *self = ExprIterator(*current.clone(), self.1);

        Some(*current)
    }
}

#[inline]
fn operatorname(op: &String) -> String {
    format!("\\operatorname{{{op}}}")
}

// Compile, and if there's more than one latex output, output an error.
fn compile1(
    expr: &mut Expr,
    vars: &mut Vec<String>,
    funcs: &mut HashMap<String, Expr>,
    fold_id: Option<u32>,
) -> Result<String, String> {
    let comp = compile(expr, vars, funcs, fold_id)?;
    Ok(comp.first().unwrap().clone().inner)
}

fn display_params(
    params: &mut [Expr],
    vars: &mut Vec<String>,
    funcs: &mut HashMap<String, Expr>,
    fold_id: Option<u32>,
) -> Result<String, String> {
    let mut out = "".to_owned();

    let ln = params.len();
    for (i, param) in params.iter_mut().enumerate() {
        let param = compile1(param, vars, funcs, fold_id)?;
        if i < ln - 1 {
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

use crate::parser::Expr;
use rocket::serde::json::{json, Value};
use std::sync::Mutex;

static ID: Mutex<u32> = Mutex::new(1);

pub fn compile(expr: &Expr, vars: &Vec<(String, f64)>) -> Result<String, String> {
    let mut latex = "".to_owned();

    match expr {
        Expr::Num(val) => latex.push_str(&val.to_string()),
        Expr::Call(name, params) => latex.push_str(&format!(
            "{}\\left({}\\right)",
            subscriptify(name),
            display_params(params)?
        )),
        Expr::Var(name) => latex.push_str(&subscriptify(name)),
        Expr::Neg(ex) => latex.push_str(&format!("-{}", &compile(ex, vars)?)),
        Expr::Mul(ex1, ex2) => latex.push_str(&format!(
            "{}\\cdot{}",
            compile(ex1, vars)?,
            compile(ex2, vars)?
        )),
        Expr::Div(ex1, ex2) => latex.push_str(&format!(
            "\\frac{{{}}}{{{}}}",
            compile(ex1, vars)?,
            compile(ex2, vars)?
        )),
        Expr::Add(ex1, ex2) => latex.push_str(&format!(
            "\\left({}+{}\\right)",
            compile(ex1, vars)?,
            compile(ex2, vars)?
        )),
        Expr::Sub(ex1, ex2) => latex.push_str(&format!(
            "\\left({}-{}\\right)",
            compile(ex1, vars)?,
            compile(ex2, vars)?
        )),
    }

    Ok(latex)
}

fn display_params(params: &Vec<Expr>) -> Result<String, String> {
    let mut out = "".to_owned();

    for (i, param) in params.iter().enumerate() {
        let param = compile(param, &vec![])?;
        if i < params.len() -1 {
            out.push_str(&format!("{param},"))
        } else {
            out.push_str(&param)
        }
    }

    Ok(out)
}

fn subscriptify(ident: &str) -> String {
    let (first, rest) = ident.split_at(1);

    format!("{first}_{{{rest}}}")
}

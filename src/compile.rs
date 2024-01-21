use crate::parser::Expr;

pub fn compile(expr: &Expr, vars: &Vec<(String, f64)>) -> Result<Vec<String>, String> {
    let mut all_latex = vec![];

    let mut latex = "".to_owned();

    match expr {
        Expr::Num(val) => latex.push_str(&val.to_string()),
        Expr::Call(name, params) => latex.push_str(&format!(
            r"{}\left({}\right)",
            subscriptify(name),
            display_params(params)?
        )),
        Expr::Var(name) => latex.push_str(&subscriptify(name)),
        Expr::Neg(ex) => latex.push_str(&format!("-{}", &compile1(ex, vars)?)),
        Expr::Mul(ex1, ex2) => latex.push_str(&format!(
            r"{}\cdot{}",
            compile1(ex1, vars)?,
            compile1(ex2, vars)?
        )),
        Expr::Div(ex1, ex2) => latex.push_str(&format!(
            r"\frac{{{}}}{{{}}}",
            compile1(ex1, vars)?,
            compile1(ex2, vars)?
        )),
        Expr::Add(ex1, ex2) => latex.push_str(&format!(
            r"\left({}+{}\right)",
            compile1(ex1, vars)?,
            compile1(ex2, vars)?
        )),
        Expr::Sub(ex1, ex2) => latex.push_str(&format!(
            r"\left({}-{}\right)",
            compile1(ex1, vars)?,
            compile1(ex2, vars)?
        )),
        Expr::Def { left, right, then } => {
            latex.push_str(&format!(
                r"{}={}",
                compile1(left, vars)?,
                compile1(right,vars)?
            ));
            if let Some(then) = then {
                all_latex.push(latex.clone());
                latex.clear();
                latex.push_str(&compile1(then, vars)?);
            }
        }
    }

    if !latex.is_empty() {
        // For whatever reason just taking a reference here doesn't work;
        // Frankly, as_str is more verbose, so idk why I don't use it
        all_latex.push(latex.clone());
        latex.clear();
    }
    Ok(all_latex)
}

// Compile, and if theres more than one latex output, output an error.
fn compile1(expr: &Expr, vars: &Vec<(String, f64)>) -> Result<String, String> {
    let comp = compile(expr, vars)?;

    let len = comp.len();
    if len != 1 {
        Err(format!("Expected only one expression. Got {len}"))
    } else {
        // Unwrap is safe here since we checked above than the length is 1
        Ok(comp.first().unwrap().to_owned())
    }
}

fn display_params(params: &Vec<Expr>) -> Result<String, String> {
    let mut out = "".to_owned();

    for (i, param) in params.iter().enumerate() {
        let param = compile1(param, &vec![])?;
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

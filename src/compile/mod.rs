use crate::parser::Expr;
use crate::SRC_F;
use ariadne::{Report, ReportKind, Source};
use chumsky::chain::Chain;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::ops::Range;
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

//noinspection ALL
pub fn compile(
    expr: &mut Expr,
    vars: &mut Vec<String>,
    funcs: &mut Vec<String>,
    mut fold_id: Option<u32>,
) -> Result<Vec<Latex>, String> {
    let mut all_latex = vec![];

    let mut latex = String::new();

    match expr {
        Expr::Num(int, frac) => latex.push_str(&format!("{int}.{frac}")),
        Expr::Call(name, params) => latex.push_str(&format!(
            r"{}\left({}\right)",
            if funcs.contains(name) {
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
            then: _,
        } => {
            let fold_id = gen_id();

            all_latex.push(Latex {
                inner: format!("\\folder {name}"),
                folder_id: None,
                id: fold_id.to_string(),
            });

            let mut new_funcs: Vec<String> = vec![];

            let mut our_vars = vars.clone();
            our_vars.append(&mut args.clone());

            let iter = ExprIterator(*body.clone(), 0);
            let count = iter.clone().count();
            for (i, mut body_item) in iter.enumerate() {
                body_item.replace_all(&{
                    let mut map = HashMap::<Expr, Expr>::new();
                    for func in &new_funcs {
                        map.insert(
                            Expr::Var(func.to_owned()),
                            Expr::Call(
                                name.to_owned() + { 
                                    let (first, rest) = func.split_at(1);
                                    &format!("{}{rest}", first.to_uppercase())
                                },
                                args.clone()
                                    .iter()
                                    .map(|x| Expr::Var(subscriptify(x)))
                                    .collect(),
                            ),
                        );
                    }
                    map
                });
                match body_item {
                    Expr::Def { left, mut right, then: _ } if ExprIterator(*left.clone(), 0).count() == 1 => {
                        // We need to output a helper function for each variable
                        match *left {
                            Expr::Var(v_name) => {
                                all_latex.push(Latex {
                                    inner: format!("{}\\left({}\\right)={}", { 
                                        new_funcs.push(v_name.clone());
                                        funcs.push(name.to_owned() + {
                                            let (first, rest) = v_name.split_at(1);
                                            &format!("{}{rest}", first.to_uppercase())
                                        });
                                        subscriptify_with(name, &v_name)
                                    }, display_params_str(args.as_mut_slice()), compile1(right.as_mut(), &mut our_vars, funcs, Some(fold_id))?),
                                    folder_id: Some(fold_id.to_string()),
                                    id: gen_id().to_string(),
                                });
                            }
                            _ => unimplemented!()
                        }
                    }
                    Expr::Def { .. } => panic!("Definition's left side must consist of a single element within a function."),
                    Expr::Fol { .. } => panic!("Cannot have a folder inside of a function!"),
                    Expr::Fun { .. } => panic!("Cannot have a function inside of a function (for now)!"),
                    _ if i == count - 1 => {
                        all_latex.push(Latex {
                            inner: format!("{}\\left({}\\right)={}", subscriptify(name), display_params_str(args), compile1(&mut body_item, &mut our_vars, funcs, Some(fold_id))?),
                            folder_id: Some(fold_id.to_string()),
                            id: gen_id().to_string()
                        });
                    },
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
        if self.1 == 0 {
            self.1 += 1;
            return Some(self.0.clone());
        }

        let mut n = 2;
        let mut current = match self.0.clone() {
            Expr::Def { then, .. } | Expr::Fol { then, .. } | Expr::Fun { then, .. } => {
                then.clone()
            }
            _ => None,
        }?;
        while n <= self.1 {
            current = match *current {
                Expr::Def { then, .. } | Expr::Fol { then, .. } | Expr::Fun { then, .. } => {
                    then.clone()
                }
                _ => None,
            }?;
            n += 1;
        }

        self.1 += 1;

        Some(*current)
    }
}

#[inline]
fn operatorname(op: &String) -> String {
    format!("\\operatorname{{{op}}}")
}

pub trait ReplaceAll<F, T> {
    fn replace_all(&mut self, from_to: &HashMap<F, T>);
}

impl ReplaceAll<Expr, Expr> for Expr {
    fn replace_all(&mut self, from_to: &HashMap<Expr, Expr>) {
        for (k, v) in from_to {
            if let Expr::Var(new_name) = k {
                match self {
                    Expr::Num(_, _) => {}
                    Expr::Var(found_name) => {
                        if found_name == new_name {
                            *self = v.clone();
                        }
                    }
                    Expr::Def { left, right, then } => {
                        left.replace_all(from_to);
                        right.replace_all(from_to);
                        if let Some(then) = then {
                            then.replace_all(from_to);
                        }
                    }
                    Expr::Fol {
                        title: _,
                        body,
                        then,
                    } => {
                        for expr in body {
                            expr.replace_all(from_to);
                        }
                        if let Some(then) = then {
                            then.replace_all(from_to);
                        }
                    }
                    Expr::Neg(expr) => expr.replace_all(from_to),
                    Expr::Mul(left, right)
                    | Expr::Div(left, right)
                    | Expr::Add(left, right)
                    | Expr::Sub(left, right) => {
                        left.replace_all(from_to);
                        right.replace_all(from_to);
                    }
                    Expr::Call(_name, exprs) => {
                        for expr in exprs {
                            expr.replace_all(from_to);
                        }
                    }

                    Expr::Fun {
                        name: _,
                        args: _,
                        body,
                        then,
                    } => {
                        body.replace_all(from_to);
                        if let Some(then) = then {
                            then.replace_all(from_to);
                        }
                    }
                }
            } else {
                unimplemented!("expected a var in replace_all keys.")
            }
        }
    }
}

/// Compile, and if there's more than one latex output, output an error.
fn compile1(
    expr: &mut Expr,
    vars: &mut Vec<String>,
    funcs: &mut Vec<String>,
    fold_id: Option<u32>,
) -> Result<String, String> {
    let comp = compile(expr, vars, funcs, fold_id)?;
    Ok(comp.first().unwrap().clone().inner)
}

fn display_params(
    params: &mut [Expr],
    vars: &mut Vec<String>,
    funcs: &mut Vec<String>,
    fold_id: Option<u32>,
) -> Result<String, String> {
    let mut out = String::new();

    let ln = params.len();
    for (i, param) in params.iter_mut().enumerate() {
        let param = compile1(param, vars, funcs, fold_id)?;
        if i < ln - 1 {
            out.push_str(&format!("{param},"));
        } else {
            out.push_str(&param);
        }
    }

    Ok(out)
}

fn display_params_str(params: &mut [String]) -> String {
    let mut out = String::new();

    let mut itr = params.iter().peekable();

    while let Some(param) = itr.next() {
        if itr.peek().is_some() {
            out.push_str(&format!("{param},"));
        } else {
            out.push_str(param);
        }
    }

    out
}

fn subscriptify(ident: &str) -> String {
    let (first, rest) = ident.split_at(1);

    if rest.is_empty() {
        first.to_owned()
    } else {
        format!("{first}_{{{rest}}}")
    }
}

fn subscriptify_with(ident: &str, plus: &str) -> String {
    let (first, rest) = ident.split_at(1);

    if rest.is_empty() {
        first.to_owned()
    } else {
        format!("{first}_{{{rest}{}}}", {
            
            let (first, rest) = plus.split_at(1);
            
            format!("{}{rest}", first.to_uppercase())
        })
    }
}

/// Generate a new id in `CURRENT_ID`, and output it.
fn gen_id() -> u32 {
    *CURRENT_ID.lock().unwrap() += 1;
    *CURRENT_ID.lock().unwrap()
}

#[cfg(test)]
mod tests {
    use crate::compile::ReplaceAll;
    use crate::parser::Expr;
    use crate::parser::Expr::Num;
    use std::collections::HashMap;

    #[test]
    fn test_replace_1() {
        let mut a = Expr::Var("a".to_owned());
        let b = Expr::Call("mapped".to_owned(), vec![Num(0, 0)]);
        let mut map = HashMap::new();
        map.insert(a.clone(), b.clone());
        a.replace_all(&map);
        assert_eq!(a, b);
    }
}

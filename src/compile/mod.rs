use crate::parser::{CompOp, Expr, FormattedExpr};
use crate::SRC_F;
use ariadne::{Report, ReportKind, Source};
use std::collections::HashMap;
use std::fs::read_to_string;
use std::ops::Range;
use std::sync::Mutex;

pub mod graph_state;

static CURRENT_ID: Mutex<u32> = Mutex::new(0);

static BUILTINS: &[&str] = &[
    "exp",
    "ln",
    "log",
    "total",
    "length",
    "count",
    "mean",
    "median",
    "quantile",
    "quartile",
    "nCr",
    "nPr",
    "stats",
    "stdev",
    "stddev",
    "stdevp",
    "stddevp",
    "mad",
    "var",
    "varp",
    "variance",
    "cov",
    "covp",
    "corr",
    "spearman",
    "lcm",
    "mcm",
    "gcd",
    "mcd",
    "gcf",
    "mod",
    "ceil",
    "floor",
    "round",
    "abs",
    "min",
    "max",
    "sign",
    "signum",
    "sgn",
    "sin",
    "cos",
    "tan",
    "csc",
    "sec",
    "cot",
    "sinh",
    "cosh",
    "tanh",
    "csch",
    "sech",
    "coth",
    "arcsin",
    "arccos",
    "arctan",
    "arccsc",
    "arcsec",
    "arccot",
    "arcsinh",
    "arccosh",
    "arctanh",
    "arccsch",
    "arcsech",
    "arccoth",
    "arsinh",
    "arcosh",
    "artanh",
    "arcsch",
    "arsech",
    "arcoth",
    "polygon",
    "distance",
    "midpoint",
    "sort",
    "shuffle",
    "join",
    "unique",
    "erf",
    "TTest",
    "ttest",
    "TScore",
    "tscore",
    "iTTest",
    "ittest",
    "IndependentTTest",
    "TScore",
    "Tscore",
    "tscore",
    "normaldist",
    "tdist",
    "poissondist",
    "binomialdist",
    "uniformdist",
    "pdf",
    "cdf",
    "random",
    "inverseCdf",
    "inversecdf",
    "histogram",
    "dotplot",
    "boxplot",
    "pdf",
    "cdf",
    "rgb",
    "hsv",
    "for",
    "width",
    "height",
    "with",
    "det",
    "inv",
    "transpose",
    "rref",
    "trace",
    "tone",
];

#[derive(Clone, Debug)]
pub struct Latex {
    pub inner: String,
    pub folder_id: Option<String>,
    pub id: String,
}

//noinspection ALL
pub fn compile(
    f_expr: &mut FormattedExpr,
    vars: &mut Vec<String>,
    funcs: &mut Vec<String>,
    mut fold_id: Option<u32>,
) -> Result<Vec<Latex>, String> {
    let FormattedExpr { expr, format } = f_expr;
    let builtins = BUILTINS
        .iter()
        .map(|x: &&str| x.to_lowercase())
        .collect::<Vec<_>>();

    let mut all_latex = vec![];

    let mut latex = String::new();

    match expr {
        Expr::Num(int, frac) => latex.push_str(&format!("{int}.{frac}")),
        Expr::Cal(name, params) => latex.push_str(&format!(
            r"{}\left({}\right)",
            if funcs.contains(name) {
                subscriptify(name)
            } else if builtins.contains(name) {
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
            if let Expr::Var(name) = left.clone().expr {
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
                            Expr::Cal(
                                name.to_owned() + {
                                    let (first, rest) = func.split_at(1);
                                    &format!("{}{rest}", first.to_uppercase())
                                },
                                args.clone()
                                    .iter()
                                    .map(|x| Expr::Var(subscriptify(x)).into())
                                    .collect(),
                            ),
                        );
                    }
                    map
                });
                match body_item.expr {
                    Expr::Def { left, mut right, then: _ } if ExprIterator(*left.clone(), 0).count() == 1 => {
                        // We need to output a helper function for each variable
                        match left.expr {
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
        Expr::Pnt(x, y) => {
            all_latex.push(Latex {
                inner: format!(
                    "\\left({},{}\\right)",
                    compile1(x, vars, funcs, fold_id)?,
                    compile1(y, vars, funcs, fold_id)?
                ),
                folder_id: fold_id.map(|x| x.to_string()),
                id: gen_id().to_string(),
            });
        }
        Expr::Lst(items) => {
            let mut inner = "\\left[".to_owned();

            let items_len = dbg!(items.len());
            for (i, item) in items.iter_mut().enumerate() {
                let item = compile1(item, vars, funcs, fold_id)?;
                if i < items_len - 1 {
                    inner.push_str(&item);
                    inner.push(',');
                } else {
                    inner.push_str(&item);
                    inner.push_str("\\right]");
                }
            }

            all_latex.push(Latex {
                inner,
                folder_id: fold_id.map(|x| x.to_string()),
                id: gen_id().to_string(),
            });
        }
        Expr::Exp(bottom, top) => latex.push_str(&format!(
            "{}^{{{}}}",
            compile1(bottom, vars, funcs, fold_id)?,
            compile1(top, vars, funcs, fold_id)?
        )),
        Expr::Neq {
            left,
            op1,
            middle,
            op2,
            right,
        } => {
            let inner = format!(
                "{}{}{}{}{}",
                compile1(left, vars, funcs, fold_id)?,
                match op1 {
                    CompOp::Eq => "=",
                    CompOp::Lt => "<",
                    CompOp::Gt => ">",
                    CompOp::Leq => "\\le",
                    CompOp::Geq => "\\ge",
                },
                compile1(middle, vars, funcs, fold_id)?,
                if let Some(op) = op2 {
                    match op {
                        CompOp::Eq => "=",
                        CompOp::Lt => "<",
                        CompOp::Gt => ">",
                        CompOp::Leq => "\\le",
                        CompOp::Geq => "\\ge",
                    }
                } else {
                    ""
                },
                if let Some(right) = right {
                    compile1(right, vars, funcs, fold_id)?
                } else {
                    String::new()
                }
            );

            all_latex.push(Latex {
                inner,
                folder_id: fold_id.map(|x| x.to_string()),
                id: gen_id().to_string(),
            });
        }
        Expr::If {
            cond,
            body,
            ref mut elif_conds,
            ref mut elif_bodies,
            else_body,
        } => {
            let mut elif_string = String::new();
            for (i, elif_cond) in elif_conds.iter_mut().enumerate() {
                elif_string.push_str(&format!(
                    ",{}:{}",
                    compile1(elif_cond, vars, funcs, fold_id)?,
                    compile1(&mut elif_bodies[i], vars, funcs, fold_id)?
                ));
            }

            let else_string;
            if let Some(else_body) = else_body {
                else_string = format!(",{}", compile1(else_body, vars, funcs, fold_id)?);
            } else {
                else_string = String::new();
            }

            latex.push_str(&format!(
                "\\left\\{{{}:{}{}{}\\right\\}}",
                compile1(cond, vars, funcs, fold_id)?,
                compile1(body, vars, funcs, fold_id)?,
                elif_string,
                else_string
            ));
        }
        Expr::Cond {
            left,
            op1,
            middle,
            op2,
            right,
        } => {
            return Ok(vec![Latex {
                inner: format!(
                    "{}{}{}{}{}",
                    compile1(left, vars, funcs, fold_id)?,
                    Into::<&'static str>::into(*op1),
                    compile1(middle, vars, funcs, fold_id)?,
                    if let Some(op2) = op2 {
                        (*op2).into()
                    } else {
                        ""
                    },
                    if let Some(right) = right {
                        compile1(right, vars, funcs, fold_id)?
                    } else {
                        String::new()
                    }
                ),

                folder_id: fold_id.map(|x| x.to_string()),
                id: gen_id().to_string(),
            }]);
        }
    }

    if !latex.is_empty() {
        all_latex.push(Latex {
            inner: latex.clone(),
            folder_id: fold_id.map(|x| x.to_string()),
            id: gen_id().to_string(),
        });
        latex.clear();
    }
    Ok(all_latex)
}

#[derive(Debug, Clone)]
pub struct ExprIterator(FormattedExpr, usize);

impl Iterator for ExprIterator {
    type Item = FormattedExpr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.1 == 0 {
            self.1 += 1;
            return Some(self.0.clone());
        }

        let mut n = 2;
        let mut current = match &self.0.expr {
            Expr::Def { then, .. } | Expr::Fol { then, .. } | Expr::Fun { then, .. } => {
                Some(FormattedExpr {
                    expr: self.0.expr.clone(),
                    format: self.0.format.clone(),
                })
            }
            _ => None,
        }?;
        while n <= self.1 {
            current = match &current.expr {
                Expr::Def { then, .. } | Expr::Fol { then, .. } | Expr::Fun { then, .. } => {
                    Some(FormattedExpr {
                        expr: current.expr,
                        format: current.format,
                    })
                }
                _ => None,
            }?;
            n += 1;
        }

        self.1 += 1;

        Some(current)
    }
}

#[inline]
fn operatorname(op: &String) -> String {
    format!("\\operatorname{{{op}}}")
}

pub trait ReplaceAll<F, T> {
    fn replace_all(&mut self, from_to: &HashMap<F, T>);
}

impl ReplaceAll<Expr, Expr> for FormattedExpr {
    fn replace_all(&mut self, from_to: &HashMap<Expr, Expr>) {
        for (k, v) in from_to {
            if let Expr::Var(new_name) = k {
                match self.expr.clone() {
                    Expr::Num(_, _) => {}
                    Expr::Var(found_name) => {
                        if found_name == *new_name {
                            *self = v.clone().into();
                        }
                    }
                    Expr::Def {
                        mut left,
                        mut right,
                        then,
                    } => {
                        left.replace_all(from_to);
                        right.replace_all(from_to);
                        if let Some(mut then) = then {
                            then.replace_all(from_to);
                        }
                    }
                    Expr::Fol {
                        title: _,
                        body,
                        then,
                    } => {
                        for mut expr in body {
                            expr.replace_all(from_to);
                        }
                        if let Some(mut then) = then {
                            then.replace_all(from_to);
                        }
                    }
                    Expr::Neg(mut expr) => expr.replace_all(from_to),
                    Expr::Mul(mut left, mut right)
                    | Expr::Div(mut left, mut right)
                    | Expr::Add(mut left, mut right)
                    | Expr::Sub(mut left, mut right) => {
                        left.replace_all(from_to);
                        right.replace_all(from_to);
                    }
                    Expr::Cal(_name, exprs) => {
                        for mut expr in exprs {
                            expr.replace_all(from_to);
                        }
                    }

                    Expr::Fun {
                        name: _,
                        args: _,
                        mut body,
                        then,
                    } => {
                        body.replace_all(from_to);
                        if let Some(mut then) = then {
                            then.replace_all(from_to);
                        }
                    }
                    Expr::Pnt(mut x, mut y) => {
                        x.replace_all(from_to);
                        y.replace_all(from_to);
                    }
                    Expr::Lst(exprs) => {
                        for mut expr in exprs {
                            expr.replace_all(from_to);
                        }
                    }
                    Expr::Exp(mut bottom, mut top) => {
                        bottom.replace_all(from_to);
                        top.replace_all(from_to);
                    }
                    Expr::Neq {
                        mut left,
                        op1: _,
                        mut middle,
                        op2: _,
                        right,
                    }
                    | Expr::Cond {
                        mut left,
                        op1: _,
                        mut middle,
                        op2: _,
                        right,
                    } => {
                        left.replace_all(from_to);
                        middle.replace_all(from_to);
                        if let Some(mut right) = right {
                            right.replace_all(from_to);
                        }
                    }
                    Expr::If {
                        mut cond,
                        body,
                        elif_conds,
                        elif_bodies,
                        else_body,
                    } => {
                        cond.replace_all(from_to);

                        for mut cond in elif_conds {
                            cond.replace_all(from_to);
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
    expr: &mut FormattedExpr,
    vars: &mut Vec<String>,
    funcs: &mut Vec<String>,
    fold_id: Option<u32>,
) -> Result<String, String> {
    let comp = compile(expr, vars, funcs, fold_id)?;
    Ok(comp.first().unwrap().clone().inner)
}

fn display_params(
    params: &mut [FormattedExpr],
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

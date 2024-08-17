use chumsky::prelude::*;
use strum_macros::IntoStaticStr;

#[derive(Debug, Copy, Clone, IntoStaticStr, PartialEq, Hash, Eq)]
#[repr(u8)]
pub enum CompOp {
    #[strum(serialize = "=")]
    Eq,
    #[strum(serialize = "<")]
    Lt,
    #[strum(serialize = ">")]
    Gt,
    #[strum(serialize = "\\le ")]
    Leq,
    #[strum(serialize = "\\ge ")]
    Geq,
}

impl CompOp {
    pub fn from_str(s: &str) -> Self {
        match s {
            "=" | "==" => Self::Eq,
            "<" => Self::Lt,
            ">" => Self::Gt,
            "<=" => Self::Leq,
            ">=" => Self::Geq,
            _ => panic!("unknown comparison operator: '{s}'"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct FormattedExpr {
    pub expr: Expr,
    pub format: Option<String>,
}

impl From<Expr> for FormattedExpr {
    fn from(e: Expr) -> FormattedExpr {
        FormattedExpr {
            expr: e,
            format: None,
        }
    }
}

#[derive(Debug, Clone, IntoStaticStr, PartialEq, Hash, Eq)]
pub enum Expr {
    Num(u32, u32),
    Var(String),
    Pnt(Box<FormattedExpr>, Box<FormattedExpr>),
    Lst(Vec<FormattedExpr>),
    Def {
        left: Box<FormattedExpr>,
        right: Box<FormattedExpr>,
        then: Option<Box<FormattedExpr>>,
    },
    Neq {
        left: Box<FormattedExpr>,
        op1: CompOp,
        middle: Box<FormattedExpr>,
        op2: Option<CompOp>,
        right: Option<Box<FormattedExpr>>,
    },
    Fol {
        title: String,
        body: Vec<FormattedExpr>,
        then: Option<Box<FormattedExpr>>,
    },
    If {
        cond: Box<FormattedExpr>,
        body: Box<FormattedExpr>,
        elif_conds: Vec<FormattedExpr>,
        elif_bodies: Vec<FormattedExpr>,
        else_body: Option<Box<FormattedExpr>>,
    },
    Cond {
        left: Box<FormattedExpr>,
        op1: CompOp,
        middle: Box<FormattedExpr>,
        op2: Option<CompOp>,
        right: Option<Box<FormattedExpr>>,
    },

    // - Operations -

    // Unary
    Neg(Box<FormattedExpr>),

    // Binary
    Mul(Box<FormattedExpr>, Box<FormattedExpr>),
    Div(Box<FormattedExpr>, Box<FormattedExpr>),
    Add(Box<FormattedExpr>, Box<FormattedExpr>),
    Sub(Box<FormattedExpr>, Box<FormattedExpr>),
    Exp(Box<FormattedExpr>, Box<FormattedExpr>),

    // Function
    Cal(String, Vec<FormattedExpr>),
    Fun {
        name: String,
        args: Vec<String>,
        body: Box<FormattedExpr>,
        then: Option<Box<FormattedExpr>>,
    },
}
pub fn parser() -> impl Parser<char, FormattedExpr, Error = Simple<char>> {
    let ident = text::ident().padded();

    let format = text::keyword("formatted")
        .ignore_then(filter(|c: &char| {
            c.is_alphanumeric() || *c == '{' || *c == '}'
        }))
        .repeated()
        .boxed();

    let expr = recursive(|expr| {
        let ineq = |is_conditional: bool| {
            expr.clone()
                .then(choice((
                    if is_conditional {
                        just("==").map(CompOp::from_str)
                    } else {
                        just("=").map(CompOp::from_str)
                    },
                    just(">=").ignore_then(just(r"\\ge")).map(CompOp::from_str),
                    just("<=").ignore_then(just(r"\\le")).map(CompOp::from_str),
                    just("<").map(CompOp::from_str),
                    just(">").map(CompOp::from_str),
                )))
                .then(expr.clone())
                .then(
                    choice((
                        if is_conditional {
                            just("==").map(CompOp::from_str)
                        } else {
                            just("=").map(CompOp::from_str)
                        },
                        just(">=").map(CompOp::from_str),
                        just("<=").map(CompOp::from_str),
                        just("<").map(CompOp::from_str),
                        just(">").map(CompOp::from_str),
                    ))
                    .or_not(),
                )
                .then(expr.clone().or_not())
                .padded()
                .boxed()
        };

        let iff = text::keyword("if")
            .ignore_then(ineq(true))
            .then(expr.clone().delimited_by(just('{'), just('}')))
            .padded()
            .then(
                text::keyword("elif")
                    .ignore_then(ineq(true))
                    .then(expr.clone().delimited_by(just('{'), just('}')))
                    .repeated(),
            )
            .padded()
            .then(
                text::keyword("else")
                    .padded()
                    .ignore_then(expr.clone().delimited_by(just('{'), just('}')))
                    .or_not(),
            )
            .map(|(((cond, body), elif), else_body)| Expr::If {
                cond: Box::new(FormattedExpr::from(Expr::Cond {
                    left: Box::new(cond.0 .0 .0 .0),
                    op1: cond.0 .0 .0 .1,
                    middle: Box::new(cond.0 .0 .1),
                    op2: cond.0 .1,
                    right: cond.1.map(Box::new),
                })),
                body: Box::new(body),
                elif_conds: elif
                    .iter()
                    .map(|x| {
                        let x = x.clone();
                        FormattedExpr::from(Expr::Cond {
                            left: Box::new(x.0 .0 .0 .0 .0),
                            op1: x.0 .0 .0 .0 .1,
                            middle: Box::new(x.0 .0 .0 .1),
                            op2: x.0 .0 .1,
                            right: x.0 .1.map(Box::new),
                        })
                    })
                    .collect(),
                elif_bodies: elif.iter().map(|x| x.1.clone()).collect(),
                else_body: else_body.map(Box::new),
            });

        let int = filter(move |c: &char| c.is_ascii_digit())
            .repeated()
            .at_least(1)
            .collect()
            .then_ignore(filter(|c: &char| *c == '.').repeated().at_most(1))
            .then(
                filter(move |c: &char| c.is_ascii_digit())
                    .repeated()
                    .collect(),
            )
            .map(|(int, frac): (String, String)| {
                Expr::Num(int.parse().unwrap(), frac.parse().unwrap_or(0))
            })
            .map(|num| FormattedExpr {
                expr: num,
                format: None,
            })
            .padded();

        let atom = int
            .or(expr.clone().delimited_by(just('('), just(')')).padded())
            .or(ident
                .then(
                    expr.clone()
                        .separated_by(just(','))
                        .allow_trailing()
                        .delimited_by(just('('), just(')')),
                )
                .map(|(f, args)| Expr::Cal(f, args))
                .map(|expr| FormattedExpr { expr, format: None })
                .padded())
            .or(iff.map(|expr| FormattedExpr { expr, format: None }))
            .or(ident.map(|ident| FormattedExpr { expr: Expr::Var(ident), format: None}));

        let atom = atom
            .or(expr
                .clone()
                .then_ignore(just(','))
                .then(expr.clone())
                .delimited_by(just('('), just(')'))
                .map(|(x, y)| Expr::Pnt(Box::new(x), Box::new(y)))
                .map(|expr| FormattedExpr { expr, format: None }))
            .or(expr
                .clone()
                .separated_by(just(','))
                .allow_trailing()
                .delimited_by(just('['), just(']'))
                .map(Expr::Lst)
                .map(|expr| FormattedExpr { expr, format: None }));

        let op = |c: char| just(c).padded();

        let neg = op('-')
            .repeated()
            .then(atom)
            .foldr(|_op, rhs| Expr::Neg(Box::new(rhs)).into())
            .map(Into::into);

        let exp = neg
            .clone()
            .then(op('^').ignore_then(neg.clone()).repeated())
            .foldl(|lhs, rhs| Expr::Exp(Box::new(lhs), Box::new(rhs)).into());

        let product = choice((
            exp.clone()
                .then(op('*').ignore_then(exp.clone()).repeated())
                .foldl(|lhs, rhs| Expr::Mul(Box::new(lhs), Box::new(rhs)).into()),
            exp.clone()
                .then(op('/').ignore_then(exp.clone()).repeated())
                .foldl(|lhs, rhs| Expr::Div(Box::new(lhs), Box::new(rhs)).into())
        ));

        let sum = choice((
            product.clone()
                .then(op('+').ignore_then(product.clone()).repeated())
                .foldl(|lhs, rhs| Expr::Add(Box::new(lhs), Box::new(rhs)).into()),
            product.clone()
                .then(op('-').ignore_then(product.clone()).repeated())
                .foldl(|lhs, rhs| Expr::Sub(Box::new(lhs), Box::new(rhs)).into())
        ));

        sum
    })
    .boxed();
    
    let decl = recursive(|decl: Recursive<char, FormattedExpr, Simple<char>>| {
        let ineq = |is_conditional: bool| {
            expr.clone()
                .then(choice((
                    if is_conditional {
                        just("==").map(CompOp::from_str)
                    } else {
                        just("=").map(CompOp::from_str)
                    },
                    just(">=").map(CompOp::from_str),
                    just("<=").map(CompOp::from_str),
                    just("<").map(CompOp::from_str),
                    just(">").map(CompOp::from_str),
                )))
                .then(expr.clone())
                .then(
                    choice((
                        if is_conditional {
                            just("==").map(CompOp::from_str)
                        } else {
                            just("=").map(CompOp::from_str)
                        },
                        just(">=").map(CompOp::from_str),
                        just("<=").map(CompOp::from_str),
                        just("<").map(CompOp::from_str),
                        just(">").map(CompOp::from_str),
                    ))
                    .repeated()
                    .at_most(1),
                )
                .then(expr.clone().repeated().at_most(1))
                .boxed()
        };

        let function = text::keyword("fn")
            .padded()
            .ignore_then(text::ident())
            .padded()
            .then(
                text::ident()
                    .separated_by(just(','))
                    .allow_trailing()
                    .delimited_by(just('('), just(')')),
            )
            .padded()
            .then(decl.clone().delimited_by(just('{'), just('}')))
            .padded()
            .then(decl.clone().or_not())
            .map(|(((name, args), body), then)| Expr::Fun {
                name,
                args,
                body: Box::new(body),
                then: then.map(Box::new),
            })
            .boxed();

        let folder = text::keyword("fold")
            .ignore_then(
                none_of("\"")
                    .repeated()
                    .delimited_by(just("\""), just("\""))
                    .collect::<String>()
                    .padded(),
            )
            .then(decl.clone().repeated().delimited_by(just('{'), just('}')))
            .then(decl.clone().or_not())
            .padded()
            .map(|((title, body), then)| Expr::Fol {
                title,
                body,
                then: then.map(Box::new),
            })
            .boxed();

        let def_or_implicit = expr
            .clone()
            .then_ignore(just('='))
            .then(expr.clone())
            .then_ignore(just(';'))
            .then(decl.clone().or_not())
            .map(|((left, right), then)| Expr::Def {
                left: Box::new(left),
                right: Box::new(right),
                then: then.map(Box::new),
            });

        let ineq =
            ineq(false)
                .then_ignore(just(';'))
                .map(|((((left, op1), middle), op2), right)| Expr::Neq {
                    left: Box::new(left),
                    op1,
                    middle: Box::new(middle),
                    op2: op2.first().copied(),
                    right: right.first().map(|x| Box::new(x.clone())),
                });

        choice((
            folder.map(Into::into),
            function.map(Into::into),
            ineq.map(Into::into),
            def_or_implicit.map(Into::into),
            // expr,
        ))
        .then(format)
        .map(|(expr, format): (FormattedExpr, Vec<char>)| FormattedExpr {
            expr: expr.expr,
            format: {
                let format_str = String::from_iter(format);
                if format_str.is_empty() {
                    Some(format_str)
                } else {
                    None
                }
            },
        })
        .padded()
        .boxed()
    });

    decl.then_ignore(end())
}

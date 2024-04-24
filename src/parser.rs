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
    #[strum(serialize = "\\le")]
    Leq,
    #[strum(serialize = "\\ge")]
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

#[derive(Debug, Clone, IntoStaticStr, PartialEq, Hash, Eq)]
pub enum Expr {
    Num(u32, u32),
    Var(String),
    Pnt(Box<Expr>, Box<Expr>),
    Lst(Vec<Expr>),
    Def {
        left: Box<Expr>,
        right: Box<Expr>,
        then: Option<Box<Expr>>,
    },
    Neq {
        left: Box<Expr>,
        op1: CompOp,
        middle: Box<Expr>,
        op2: Option<CompOp>,
        right: Option<Box<Expr>>,
    },
    Fol {
        title: String,
        body: Vec<Expr>,
        then: Option<Box<Expr>>,
    },
    If {
        cond: Box<Expr>,
        body: Box<Expr>,
        elif_conds: Vec<Expr>,
        elif_bodies: Vec<Expr>,
        else_body: Option<Box<Expr>>,
    },
    Cond {
        left: Box<Expr>,
        op1: CompOp,
        middle: Box<Expr>,
        op2: Option<CompOp>,
        right: Option<Box<Expr>>,
    },

    // - Operations -

    // Unary
    Neg(Box<Expr>),

    // Binary
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Exp(Box<Expr>, Box<Expr>),

    // Function
    Cal(String, Vec<Expr>),
    Fun {
        name: String,
        args: Vec<String>,
        body: Box<Expr>,
        then: Option<Box<Expr>>,
    },
}
pub fn parser() -> impl Parser<char, Expr, Error = Simple<char>> {
    let ident = text::ident().padded();
    let expr = recursive(|expr| {

        let expr2 = expr.clone();
        let ineq = |is_conditional: bool| {
            expr2
                .clone()
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
                .then(expr2.clone())
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
                        .or_not()
                )
                .then(expr2.clone().or_not())
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
                cond: Box::new(Expr::Cond {
                    left: Box::new(cond.0 .0 .0 .0),
                    op1: cond.0 .0 .0 .1,
                    middle: Box::new(cond.0 .0 .1),
                    op2: cond.0 .1,
                    right: cond.1.map(Box::new),
                }),
                body: Box::new(body),
                elif_conds: elif
                    .iter()
                    .map(|x| {
                        let x = x.clone();
                        Expr::Cond {
                            left: Box::new(x.0 .0 .0 .0 .0),
                            op1: x.0 .0 .0 .0 .1,
                            middle: Box::new(x.0 .0 .0 .1),
                            op2: x.0 .0 .1,
                            right: x.0 .1.map(Box::new),
                        }
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
                .padded())
            .or(iff)
            .or(ident.map(Expr::Var).padded());
        let atom = atom
            .clone()
            .or(expr
                .clone()
                .then_ignore(just(','))
                .then(expr.clone())
                .delimited_by(just('('), just(')'))
                .map(|(x, y)| Expr::Pnt(Box::new(x), Box::new(y))))
            .or(expr
                .clone()
                .separated_by(just(','))
                .allow_trailing()
                .delimited_by(just('['), just(']'))
                .map(Expr::Lst));

        let op = |c: char| just(c).padded();

        let neg = op('-')
            .repeated()
            .then(atom)
            .foldr(|_op, rhs| Expr::Neg(Box::new(rhs)));

        let exp = neg
            .clone()
            .then(op('^').to(Expr::Exp as fn(_, _) -> _).then(neg).repeated())
            .foldl(|lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)));

        let product = exp
            .clone()
            .then(
                op('*')
                    .to(Expr::Mul as fn(_, _) -> _)
                    .or(op('/').to(Expr::Div as fn(_, _) -> _))
                    .then(exp)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)));

        let sum = product
            .clone()
            .then(
                op('+')
                    .to(Expr::Add as fn(_, _) -> _)
                    .or(op('-').to(Expr::Sub as fn(_, _) -> _))
                    .then(product)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)));

        sum.boxed()
    })
    .boxed();

    let expr2 = expr.clone();
    let ineq = |is_conditional: bool| {
        expr2
            .clone()
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
            .then(expr2.clone())
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
            .then(expr2.clone().repeated().at_most(1))
            .boxed()
    };
    let decl = recursive(|decl: Recursive<char, Expr, Simple<char>>| {
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

        
        choice((folder, function, ineq, def_or_implicit, expr))
            .padded()
            .boxed()
    });

    decl.then_ignore(end())
}

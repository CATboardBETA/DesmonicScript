use chumsky::prelude::*;
use strum_macros::IntoStaticStr;

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
    Fol {
        title: String,
        body: Vec<Expr>,
        then: Option<Box<Expr>>,
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
    Call(String, Vec<Expr>),
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
                .map(|(f, args)| Expr::Call(f, args))
                .padded())
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
            .then(
                op('^')
                    .to(Expr::Exp as fn(_, _) -> _)
                    .then(neg)
                    .repeated(),
            )
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
    });

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
            .then(decl.or_not())
            .map(|((left, right), then)| Expr::Def {
                left: Box::new(left),
                right: Box::new(right),
                then: then.map(Box::new),
            });

        choice((folder, function, def_or_implicit, expr))
            .padded()
            .boxed()
    });

    decl.then_ignore(end())
}

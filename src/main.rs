use log::error;
use pest::Parser;
use pest::iterators::{Pair, Pairs};
use pest_derive::Parser;
use std::cmp::Ordering;
use std::fs;

#[derive(Parser)]
#[grammar = "desmonic.pest"]
pub struct DesmonicParser;

fn main() {
    env_logger::init();
    let arg = std::env::args()
        .nth(1)
        .expect("Must pass one argument to desmonicscript!");
    let file_unparsed = fs::read_to_string(arg).expect("Cannot read file!");
    // Unwrap is okay here because this rule is infallible assuming valid code
    let file = DesmonicParser::parse(Rule::file, &file_unparsed)
        .expect("Failed to parse")
        .next()
        .unwrap();

    let out = parse(
        file.into_inner(),
        &mut vec!["x".to_owned(), "y".to_string()],
    );
    for x in out {
        println!("{x}");
    }
}

fn parse(rule_in: Pairs<Rule>, vars: &mut Vec<String>) -> Vec<String> {
    let lines = &mut vec![];
    for rule in rule_in {
        // This is the only time that the result of [`parse_one`] should not be used
        let _ = parse_one(rule, vars, lines);
    }
    lines.clone()
}

#[must_use]
fn parse_one(rule: Pair<Rule>, vars: &mut Vec<String>, lines: &mut Vec<String>) -> String {
    let mut line = String::new();
    match rule.as_rule() {
        Rule::EOI => {
            // This will match at the end, but we don't need to do anything.
        }
        Rule::num => {
            line.push_str(rule.as_str());
        }
        Rule::var => {
            let var = rule.as_str();

            if !vars.contains(&var.to_owned()) {
                let (line, col) = rule.line_col();
                error!("{line}:{col}: Using a nonexistant variable '{var}'!")
            }
            match var.len().cmp(&1) {
                Ordering::Less => {
                    unreachable!()
                }
                Ordering::Equal => line.push_str(var),
                Ordering::Greater => {
                    let (first_c, rest) = var.split_at(1);
                    // need to insert `_{` and `}` around the remaining bits
                    line.push_str(&format!("{first_c}_{{{rest}}}"))
                }
            }
        }
        Rule::parens => {
            let expr = parse_one(rule.into_inner().next().unwrap(), vars, lines);
            line += &format!("\\left({expr}\\right)")
        }
        Rule::p2 => {}
        Rule::p3 => {}
        Rule::list => {}
        Rule::list_expr => {}
        Rule::conditional => {}
        Rule::iff => {}
        Rule::elsee => {}
        Rule::unary_op => {}
        Rule::exp => {
            let mut inner_rules = rule.into_inner();
            let left = parse_one(inner_rules.next().unwrap(), vars, lines);
            line.push_str(&left);
            while let Some(op) = inner_rules.next() {
                line.push_str(op.as_str());
                line.push_str(&format!(
                    "{{{}}}",
                    parse_one(inner_rules.next().unwrap(), vars, lines)
                ));
            }
        }
        Rule::prod => {
            let mut inner_rules = rule.into_inner();
            let mut left = parse_one(inner_rules.next().unwrap(), vars, lines);
            while let Some(op) = inner_rules.next() {
                let right = parse_one(inner_rules.next().unwrap(), vars, lines);
                match op.as_str() {
                    "*" => left = format!("{left}\\cdot{right}"),
                    "/" => left = format!("\\frac{{{left}}}{{{right}}}"),
                    "%" => left = format!("\\operatorname{{mod}}\\left({left},{right}\right)"),
                    _ => unreachable!(),
                }
            }
            line += &left;
        }
        Rule::sum => {
            let mut inner_rules = rule.into_inner();
            let left = parse_one(inner_rules.next().unwrap(), vars, lines);
            line.push_str(&left);
            while let Some(op) = inner_rules.next() {
                line.push_str(op.as_str());
                line.push_str(&parse_one(inner_rules.next().unwrap(), vars, lines));
            }
        }
        Rule::expr_out => {
            let expr = parse_one(rule.into_inner().next().unwrap(), vars, lines);
            lines.push(expr);
            println!("{lines:?}");
        }
        Rule::expr_in => {
            line += &parse_one(rule.into_inner().next().unwrap(), vars, lines);
        }
        Rule::implicit_explicit => {
            let mut inner_rules = rule.into_inner();
            let left = inner_rules.next().unwrap();
            let cmp = inner_rules.next().unwrap();
            let right = inner_rules.next().unwrap();
            // TODO: Find a cleaner way to find out if it's a single var on the left side
            if cmp.as_str() == "="
                && left
                    .clone()
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_rule()
                    == Rule::var
            {
                // Explicit, add variable to list.
                let left = left
                    .clone()
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap();
                vars.push(left.as_str().to_owned());
            }
            let left = parse_one(left, vars, lines);
            let right = parse_one(right, vars, lines);
            lines.push(format!("{}{}{}", left, cmp.as_str(), right,));
        }
        Rule::sqrt => {
            let val = parse_one(rule.into_inner().next().unwrap(), vars, lines);
            line += &format!("\\sqrt{{{val}}}")
        }
        Rule::cbrt => {
            let val = parse_one(rule.into_inner().next().unwrap(), vars, lines);
            line += &format!("\\sqrt[3]{{{val}}}")
        }
        Rule::nthrt => {
            let mut inner_rules = rule.into_inner();
            let val = parse_one(inner_rules.next().unwrap(), vars, lines);
            let n = parse_one(inner_rules.next().unwrap(), vars, lines);
            line += &format!("\\sqrt[{n}]{{{val}}}")
        }
        Rule::folder => {}
        Rule::note => {}
        _ => unreachable!(),
    }
    line
}

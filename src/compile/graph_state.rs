use crate::compile::Latex;
use rand::Rng;
use rocket::serde::json::{to_value, Value};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

pub trait ToGraphStateJson {
    fn into_graph_state(self) -> Value;
}

impl ToGraphStateJson for Vec<Latex> {
    //noinspection SpellCheckingInspection
    fn into_graph_state(mut self) -> Value {
        let expressions = self
            .iter()
            .map(|l: &Latex| {
                if l.inner.starts_with("\\folder ") {
                    Expression::Folder {
                        id: l.id.to_string(),
                        title: {
                            let s = l.inner.trim_start_matches("\\folder ");
                            if s.is_empty() {
                                None
                            } else {
                                Some(s.to_owned())
                            }
                        },
                        other: Default::default(),
                    }
                } else {
                    Expression::Expression {
                        id: l.id.parse().expect("Failed to parse id"),
                        latex: Some(l.clone().inner),
                        color: None,
                        folder_id: l.clone().folder_id,
                        other: Default::default(),
                    }
                }
            })
            .collect::<Vec<_>>();

        to_value(GraphState {
            version: 11,
            random_seed: rand::thread_rng().gen_range(0..u64::MAX).to_string(),
            graph: GraphMeta {
                viewport: ViewportMeta {
                    xmin: -10.,
                    ymin: -10.,
                    xmax: 10.,
                    ymax: 10.,
                },
                show_grid: true,
                show_x_axis: true,
                show_y_axis: true,
                x_axis_numbers: true,
                y_axis_numbers: true,
                polar_numbers: false,
            },
            expressions: Expressions { list: expressions },
        })
        .unwrap()
    }
}

// Everything from here down is credited to

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct GraphState {
    pub version: u32,
    pub random_seed: String,
    pub graph: GraphMeta,
    pub expressions: Expressions,
}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Expressions {
    list: Vec<Expression>,
}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct GraphMeta {
    viewport: ViewportMeta,
    show_grid: bool,
    show_x_axis: bool,
    show_y_axis: bool,
    x_axis_numbers: bool,
    y_axis_numbers: bool,
    polar_numbers: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ViewportMeta {
    xmin: f64,
    ymin: f64,
    xmax: f64,
    ymax: f64,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Expression {
    Expression {
        id: String,
        latex: Option<String>,
        color: Option<Color>,
        #[serde(rename = "folderId")]
        folder_id: Option<String>,
        #[serde(flatten)]
        other: HashMap<String, Value>,
    },
    Folder {
        id: String,
        title: Option<String>,
        #[serde(flatten)]
        other: HashMap<String, Value>,
    },
    #[serde(rename = "text")]
    Comment { id: String, text: String },
}

pub struct StrIntVisitor;
impl<'de> Visitor<'de> for StrIntVisitor {
    type Value = u32;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an unsigned integer")
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        v.parse()
            .map_err(|_| E::custom(format!("failed to parse unsigned integer from {}", v)))
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Color(u32);
impl<'a> Deserialize<'a> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_str(ColorVisitor {})
    }
}
impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("#{:x}", self.0))
    }
}
pub struct ColorVisitor {}
impl<'de> Visitor<'de> for ColorVisitor {
    type Value = Color;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a hex color literal")?;
        Ok(())
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if !v.starts_with('#') {
            return Err(E::custom("first character of color literal not \"#\""));
        }
        u32::from_str_radix(&v[1..], 16)
            .map_err(|_| E::custom("failed to parse hex literal"))
            .map(Color)
    }
}

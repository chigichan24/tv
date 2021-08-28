use std::{collections::HashMap, fmt::Display};

use crate::utils::Transpose;
use anyhow::{Context, Result};

use unicode_width::UnicodeWidthStr;

type Json<'a> = HashMap<&'a str, serde_json::Value>;

#[derive(Debug)]
pub struct Data<'a> {
    data: Vec<Json<'a>>,
    sort_key: Option<&'a str>,
}

impl<'a> Data<'a> {
    pub fn from(s: &'a str) -> Result<Self> {
        let data = serde_json::from_str::<Vec<HashMap<&str, serde_json::Value>>>(s);

        match data {
            Ok(vec) => Ok(Self {
                data: vec,
                sort_key: None,
            }),
            Err(_) => Self::csv(s),
        }
    }

    pub fn set_sort_key(&mut self, s: &'a str) -> &mut Self {
        self.sort_key = Some(s);
        self
    }

    fn csv(s: &'a str) -> Result<Self> {
        let mut lines = s.split("\n");
        let sub = lines
            .next()
            .with_context(|| "error")
            .map(|x| x.split(","))?;
        let maps = lines
            .map(|x| sub.clone().zip(x.split(",")))
            .map(|xs| {
                xs.fold(HashMap::new() as Json<'a>, |mut hash, (k, v)| {
                    hash.insert(k, serde_json::Value::String(String::from(v)));
                    hash
                })
            })
            .collect::<Vec<_>>();

        Ok(Self {
            data: maps,
            sort_key: None,
        })
    }

    fn keys(&self) -> Vec<String> {
        self.data
            .get(0)
            .map(|x| {
                let mut keys = x.keys().map(|&x| String::from(x)).collect::<Vec<_>>();
                keys.sort();
                keys
            })
            .unwrap_or_default()
    }

    fn values(&self) -> Vec<Vec<String>> {
        let keys = self.keys();

        let data = if let Some(key) = self.sort_key {
            let mut data = self.data.clone();
            data.sort_by_cached_key(|x| {
                x.get(key)
                    .as_deref()
                    .unwrap_or(&serde_json::Value::default())
                    .to_string()
            });
            data
        } else {
            self.data.clone()
        };

        data.iter()
            .map(|x| {
                keys.clone()
                    .iter()
                    .map(|k| x.get(k.as_str()))
                    .collect::<Vec<_>>()
            })
            .map(|x| {
                x.into_iter()
                    .map(|x| match x {
                        Some(serde_json::Value::String(s)) => String::from(s),
                        Some(serde_json::Value::Bool(b)) => b.to_string(),
                        Some(serde_json::Value::Number(n)) => n.to_string(),
                        None => String::from("undefined"),
                        _ => String::from("..."),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }

    fn pads(&self) -> Vec<usize> {
        let keys = self.keys();
        self.values()
            .iter()
            .transpose()
            .zip(keys)
            .map(|(mut xs, k)| {
                xs.push(&k);
                xs.into_iter().map(|x| x.width()).max().unwrap_or_default()
            })
            .collect::<Vec<_>>()
    }
}

impl<'a> Display for Data<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let keys = self.keys();
        let values = self.values();
        let pads = self.pads();

        let title_str = keys
            .iter()
            .zip(pads.clone())
            .map(|(x, pad)| format!("{:>width$}", x, width = pad))
            .collect::<Vec<_>>()
            .join("|");
        write!(f, "|{}|\n", title_str)?;

        let border = pads
            .clone()
            .iter()
            .map(|&x| "-".repeat(x))
            .collect::<Vec<_>>()
            .join("|");
        write!(f, "|{}|\n", border)?;

        values
            .into_iter()
            .map(|xs| {
                xs.iter()
                    .zip(pads.clone())
                    .map(|(x, pad)| format!("{}{}", " ".repeat(pad - x.width()), x))
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .try_for_each(|x| write!(f, "|{}|\n", x))
    }
}

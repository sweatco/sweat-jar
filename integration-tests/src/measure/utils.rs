use std::{env::var, fs::OpenOptions, future::Future, io::Write, iter::once};

use anyhow::{bail, Result};
use near_sdk::serde::Serialize;
use num_format::{Buffer, CustomFormat};
use serde_json::{to_string_pretty, to_value, Map, Value};
use workspaces::{types::Gas, Account};

use crate::{context::Context, product::RegisterProductCommand};

const MEASURE_JARS_COUNT: usize = 5;
const MEASURE_JARS_MULTIPLIER: usize = 10;

fn number_of_jars_to_measure() -> usize {
    var("MEASURE_JARS_COUNT")
        .map(|val| val.parse().unwrap_or(MEASURE_JARS_COUNT))
        .unwrap_or(MEASURE_JARS_COUNT)
}

fn measure_jars_multiplier() -> usize {
    var("MEASURE_JARS_MULTIPLIER")
        .map(|val| val.parse().unwrap_or(MEASURE_JARS_MULTIPLIER))
        .unwrap_or(MEASURE_JARS_MULTIPLIER)
}

pub fn measure_jars_range() -> Vec<usize> {
    generate_measure_jars_range(number_of_jars_to_measure(), measure_jars_multiplier())
}

pub fn generate_measure_jars_range(max: usize, multiplier: usize) -> Vec<usize> {
    let vals = (1..=max).map(|i| i * multiplier + 1);
    once(1).chain(vals).collect()
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MeasureData {
    total: u64,
    cost: Vec<String>,
    diff: Vec<String>,
}

impl MeasureData {
    pub fn new(cost: Vec<(Gas, usize)>, diff: Vec<i128>) -> Self {
        MeasureData {
            total: cost.iter().map(|a| a.0).sum(),
            cost: cost
                .into_iter()
                .map(|a| format!("  {} - number of jars: {}  ", format_number(a.0), a.1))
                .collect(),
            diff: diff
                .into_iter()
                .map(|a| {
                    format!(
                        "  {} - jar cost: {}  ",
                        format_number(a),
                        format_number(a / measure_jars_multiplier() as i128)
                    )
                })
                .collect(),
        }
    }
}

pub(crate) fn generate_permutations<One, Two>(one: &[One], two: &[Two]) -> Vec<(One, Two)>
where
    One: Copy,
    Two: Copy,
{
    one.into_iter()
        .flat_map(|&item1| two.iter().map(move |&item2| (item1, item2)))
        .collect()
}

pub(crate) fn generate_permutations_3<One, Two, Three>(
    one: &[One],
    two: &[Two],
    three: &[Three],
) -> Vec<(One, Two, Three)>
where
    One: Copy,
    Two: Copy,
    Three: Copy,
{
    one.into_iter()
        .flat_map(|&item1| {
            two.iter()
                .flat_map(move |&item2| three.iter().map(move |&item3| (item1, item2, item3)))
        })
        .collect()
}

fn format_numbers(json_obj: &Value) -> Value {
    match json_obj {
        Value::Number(n) => {
            if let Some(n) = n.as_i64() {
                Value::String(format_number(n))
            } else if let Some(n) = n.as_u64() {
                Value::String(format_number(n))
            } else {
                json_obj.clone()
            }
        }
        Value::Object(obj) => {
            let mut new_obj = Map::new();
            for (key, value) in obj {
                new_obj.insert(key.clone(), format_numbers(value));
            }
            Value::Object(new_obj)
        }
        Value::Array(arr) => {
            let new_arr: Vec<Value> = arr.iter().map(|v| format_numbers(v)).collect();
            Value::Array(new_arr)
        }
        _ => json_obj.clone(),
    }
}

fn format_number<T: num_format::ToFormattedStr>(number: T) -> String {
    let format = CustomFormat::builder().separator(" ").build().unwrap();

    let mut buf = Buffer::new();
    buf.write_formatted(&number, &format);

    buf.to_string()
}

pub fn append_measure<T: Serialize>(label: &str, data: T) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("../measured.txt")?;

    let value = to_value(data)?;

    let value = format_numbers(&value);

    let json = to_string_pretty(&value)?;

    let data = format!("{label}: \n{json}\n");

    file.write_all(data.as_bytes())?;

    Ok(())
}

/// Measure tests have too many different concurrent operations and may be flaky
pub async fn retry_until_ok<Res: Future<Output = Result<()>>>(mut job: impl FnMut() -> Res) -> Result<()> {
    let mut limit = 10;

    while let Err(err) = job().await {
        dbg!(&err);
        limit -= 1;
        if limit == 0 {
            bail!("Too many retries");
        }
    }

    return Ok(());
}

#[cfg(test)]
mod test {
    use crate::measure::utils::{generate_permutations, generate_permutations_3};

    #[test]
    fn permutations() -> anyhow::Result<()> {
        assert_eq!(
            generate_permutations(&['a', 'b'], &[10, 20]),
            vec![('a', 10,), ('a', 20,), ('b', 10,), ('b', 20,),]
        );

        assert_eq!(
            generate_permutations_3(&['a', 'b'], &[10, 20], &[true, false]),
            vec![
                ('a', 10, true,),
                ('a', 10, false,),
                ('a', 20, true,),
                ('a', 20, false,),
                ('b', 10, true,),
                ('b', 10, false,),
                ('b', 20, true,),
                ('b', 20, false,),
            ]
        );

        Ok(())
    }
}

pub(crate) async fn add_jar(
    context: &Context,
    account: &Account,
    product: RegisterProductCommand,
    amount: u128,
) -> anyhow::Result<()> {
    context
        .jar_contract
        .create_jar(account, product.id(), amount, context.ft_contract.account().id())
        .await?;

    Ok(())
}

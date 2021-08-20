use serde::{Deserialize, Deserializer};

pub fn option_str_to_vec<'de, D>(deserializer: D) -> Result<Vec<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    if s == "" {
        return Ok(Vec::new());
    }
    let values = s.split(",").collect::<Vec<&str>>();
    let mut values2 = Vec::new();
    for v in values {
        values2.push(
            v.parse::<i32>()
                .map_err(|_| serde::de::Error::custom("输入参数必须为数字"))?,
        )
    }
    Ok(values2)
}

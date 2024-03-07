use serde_json::Value;

pub fn parse_input_values(s: &str) -> Vec<Vec<String>> {
    s.split(';')
        .map(|s| s.split(',').map(|s| s.to_string()).collect::<Vec<String>>())
        .collect()
}

pub fn parse_input_fields(s: &str) -> Vec<String> {
    s.split(',').map(|s| s.to_string()).collect::<Vec<String>>()
}

pub fn filter_by_input_values(
    input: Value,
    values: &[Vec<String>],
    fields: &[String],
) -> anyhow::Result<Vec<Value>> {
    if let Value::Array(input) = input {
        return Ok(input
            .into_iter()
            .filter(|i| {
                if i.is_object() {
                    return values.iter().any(|v| {
                        v.iter().zip(fields).all(|(value, field)| {
                            i.pointer(field).and_then(|i| i.as_str()) == Some(value.as_str())
                        })
                    });
                }
                false
            })
            .collect());
    }
    anyhow::bail!("input is not an array");
}

pub fn get_ids(objects: &[Value]) -> Vec<String> {
    objects
        .iter()
        .filter_map(|obj| obj.get("id").and_then(Value::as_str).map(|s| s.to_string()))
        .collect()
}

pub fn get_raw_ids(objects: &[Value]) -> Vec<String> {
    objects
        .iter()
        .filter_map(|obj| {
            obj.get("id")
                .and_then(Value::as_str)
                .map(|s| s[72..96].to_string())
        })
        .collect()
}

pub fn null_str_to_option(value: &str) -> Option<&str> {
    if value.trim().eq("null") {
        None
    } else {
        Some(value)
    }
}

/// Get a object by name from the variables vector.
///
/// Assumes a object with the name exists in the variables.
///
/// # Returns
///
/// The value of the key with provided name in the first found object in the variables.
pub fn get_object_by_name_from_variables_vec(variables: &[Value], name: String) -> Option<Value> {
    variables
        .iter()
        .filter_map(|v| v.as_object())
        .find_map(|o| o.get(name.as_str()).cloned())
}

use rf::prelude::*;
use rf_validation::{
    rules::{MaxLengthRule, RequiredRule, StringRule},
    Rule, ValidatedData, Validator,
};
use std::collections::HashMap;

pub async fn validate_post(partial: bool) -> AppResult<ValidatedData> {
    let payload = all();
    let mut rules: HashMap<&str, Vec<Box<dyn Rule>>> = HashMap::from([
        (
            "title",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(StringRule),
                Box::new(MaxLengthRule::new(200)),
            ],
        ),
        (
            "body",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(StringRule),
            ],
        ),
    ]);
    if partial {
        rules.retain(|field, _| payload.contains_key(*field));
    }
    Ok(Validator::quick_validate(payload, rules).await?)
}

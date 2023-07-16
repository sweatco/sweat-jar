use serde_json::{json, Value};

pub(crate) enum Products {
    Locked12Months12Percents,
    Locked6Months6Percents,
}

impl Products {
    pub(crate) fn json(&self) -> Value {
        match self {
            Products::Locked12Months12Percents => json!({
                "id": "locked_12_months_12_percents",
                "lockup_term": 31_556_952_000_u64,
                "maturity_term": 31_556_952_000_u64,
                "is_refillable": false,
                "apy": {
                    "Constant": 0.12,
                },
                "cap": {
                    "min": 100_000u64,
                    "max": 100_000_000_000u64,
                },
                "is_restakable": false,
            }),
            Products::Locked6Months6Percents => json!({
                "id": "locked_6_months_6_percents",
                "lockup_term": 15_778_476_000_u64,
                "maturity_term": 15_778_476_000_u64,
                "is_refillable": false,
                "apy": {
                    "Constant": 0.12,
                },
                "cap": {
                    "min": 100_000u64,
                    "max": 100_000_000_000u64,
                },
                "is_restakable": false,
            }),
        }
    }

    pub(crate) fn id(&self) -> String {
        self.json().as_object().unwrap().get("id").unwrap().as_str().unwrap().to_string()
    }
}
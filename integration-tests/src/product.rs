use serde_json::{json, Value};

pub(crate) enum Products {
    Locked12Months12Percents,
    Locked6Months6Percents,
    Locked6Months6PercentsWithWithdrawFee,
    Locked10Minutes6PercentsWithWithdrawFee,
}

impl Products {
    pub(crate) fn json(&self) -> Value {
        match self {
            Products::Locked12Months12Percents => json!({
                "id": "locked_12_months_12_percents",
                "lockup_term": "31556952000",
                "is_refillable": false,
                "apy": {
                    "Constant": 0.12,
                },
                "cap": {
                    "min": "100000",
                    "max": "100000000000",
                },
                "is_restakable": false,
            }),
            Products::Locked6Months6Percents => json!({
                "id": "locked_6_months_6_percents",
                "lockup_term": "15778476000",
                "is_refillable": false,
                "apy": {
                    "Constant": 0.06,
                },
                "cap": {
                    "min": "100000",
                    "max": "100000000000",
                },
                "is_restakable": false,
            }),
            Products::Locked6Months6PercentsWithWithdrawFee => json!({
                "id": "locked_6_months_6_percents_with_withdraw_fee",
                "lockup_term": "15778476000",
                "is_refillable": false,
                "apy": {
                    "Constant": 0.06,
                },
                "cap": {
                    "min": "100000",
                    "max": "100000000000",
                },
                "is_restakable": false,
                "withdrawal_fee": {
                    "Fix": "1000",
                }
            }),
            Products::Locked10Minutes6PercentsWithWithdrawFee => json!({
                "id": "locked_10_minutes_6_percents_with_withdraw_fee",
                "lockup_term": "600000",
                "is_refillable": false,
                "apy": {
                    "Constant": 0.06,
                },
                "cap": {
                    "min": "100000",
                    "max": "100000000000",
                },
                "is_restakable": false,
                "withdrawal_fee": {
                    "Fix": "1000",
                }
            }),
        }
    }

    pub(crate) fn id(&self) -> String {
        self.json().as_object().unwrap().get("id").unwrap().as_str().unwrap().to_string()
    }
}
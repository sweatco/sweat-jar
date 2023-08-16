use serde_json::{json, Value};

pub(crate) enum RegisterProductCommand {
    Locked12Months12Percents,
    Locked6Months6Percents,
    Locked6Months6PercentsWithWithdrawFee,
    Locked10Minutes6PercentsWithWithdrawFee,
}

impl RegisterProductCommand {
    pub(crate) fn json(&self) -> Value {
        match self {
            RegisterProductCommand::Locked12Months12Percents => json!({
                "id": "locked_12_months_12_percents",
                "apy_default": ["12", 2],
                "cap_min": "100000",
                "cap_max": "100000000000",
                "terms": {
                    "type": "Fixed",
                    "data": {
                        "lockup_term": "31556952000",
                        "allows_top_up": false,
                        "allows_restaking": false,
                    }
                },
            }),
            RegisterProductCommand::Locked6Months6Percents => json!({
                "id": "locked_6_months_6_percents",
                "apy_default": ["6", 2],
                "cap_min": "100000",
                "cap_max": "100000000000",
                "terms": {
                    "type": "Fixed",
                    "data": {
                        "lockup_term": "15778476000",
                        "allows_top_up": false,
                        "allows_restaking": false,
                    }
                },
            }),
            RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee => json!({
                "id": "locked_6_months_6_percents_with_withdraw_fee",
                "apy_default": ["6", 2],
                "cap_min": "100000",
                "cap_max": "100000000000",
                "terms": {
                    "type": "Fixed",
                    "data": {
                        "lockup_term": "15778476000",
                        "allows_top_up": false,
                        "allows_restaking": false,
                    }
                },
                "withdrawal_fee": {
                    "type": "Fix",
                    "data": "1000",
                }
            }),
            RegisterProductCommand::Locked10Minutes6PercentsWithWithdrawFee => json!({
                "id": "locked_10_minutes_6_percents_with_withdraw_fee",
                "apy_default": ["6", 2],
                "cap_min": "100000",
                "cap_max": "100000000000",
                "terms": {
                    "type": "Fixed",
                    "data": {
                        "lockup_term": "600000",
                        "allows_top_up": false,
                        "allows_restaking": false,
                    }
                },
                "withdrawal_fee": {
                    "type": "Fix",
                    "data": "1000",
                }
            }),
        }
    }

    pub(crate) fn id(&self) -> String {
        self.json().as_object().unwrap().get("id").unwrap().as_str().unwrap().to_string()
    }
}
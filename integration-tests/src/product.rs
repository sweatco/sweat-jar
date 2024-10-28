use nitka::near_sdk::serde_json::{from_value, json, Value};
use serde::Serialize;
use sweat_jar_model::product::ProductDto;

#[derive(Serialize, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) enum RegisterProductCommand {
    Locked12Months12Percents,
    Locked6Months6Percents,
    Flexible6Months6Percents,
    Locked6Months6PercentsWithWithdrawFee,
    Locked10Minutes6Percents,
    Locked5Minutes60000Percents,
    Locked10Minutes60000Percents,
    Locked10Minutes6PercentsTopUp,
    Locked10Minutes6PercentsWithFixedWithdrawFee,
    Locked10Minutes6PercentsWithPercentWithdrawFee,
    Locked10Minutes20000ScoreCap,
}

impl RegisterProductCommand {
    pub(crate) fn all() -> [Self; 10] {
        [
            Self::Locked12Months12Percents,
            Self::Locked6Months6Percents,
            Self::Flexible6Months6Percents,
            Self::Locked6Months6PercentsWithWithdrawFee,
            Self::Locked10Minutes6Percents,
            Self::Locked5Minutes60000Percents,
            Self::Locked10Minutes60000Percents,
            Self::Locked10Minutes6PercentsTopUp,
            Self::Locked10Minutes6PercentsWithFixedWithdrawFee,
            Self::Locked10Minutes6PercentsWithPercentWithdrawFee,
        ]
    }
}

impl RegisterProductCommand {
    pub(crate) fn json_for_premium(&self, public_key: String) -> Value {
        let mut json = self.json();
        if let Value::Object(obj) = &mut json {
            obj.insert("public_key".to_string(), Value::String(public_key));
        }
        json
    }

    pub(crate) fn get(self) -> ProductDto {
        from_value(self.json()).unwrap()
    }

    fn json(&self) -> Value {
        match self {
            RegisterProductCommand::Locked12Months12Percents => json!({
                "id": "locked_12_months_12_percents",
                "cap": ["100000", "100000000000"],
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "31556952000",
                        "apy": {
                            "default": ["12", 2],
                        },
                    }
                },
                "is_enabled": true,
            }),
            RegisterProductCommand::Locked6Months6Percents => json!({
                "id": "locked_6_months_6_percents",
                "cap": ["100000", "100000000000"],
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "15778476000",
                        "apy": {
                            "default": ["6", 2],
                        },
                    }
                },
                "is_enabled": true,
            }),
            RegisterProductCommand::Flexible6Months6Percents => json!({
                "id": "flexible_6_months_6_percents",
                "apy_default": ["12", 2],
                "apy_fallback": ["6", 2],
                "cap": ["100000", "100000000000"],
                "terms": {
                    "type": "flexible",
                    "data": {
                        "apy": {
                            "default": ["12", 2],
                            "fallback": ["6", 2],
                        },
                    },
                },
                "is_enabled": true,
                "score_cap": 0,
            }),

            RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee => json!({
                "id": "locked_6_months_6_percents_with_withdraw_fee",
                "cap": ["100000", "100000000000"],
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "15778476000",
                        "apy": {
                            "default": ["6", 2],
                        },
                    }
                },
                "withdrawal_fee": {
                    "type": "fix",
                    "data": "1000",
                },
                "is_enabled": true,
                "score_cap": 0,
            }),
            RegisterProductCommand::Locked10Minutes6Percents => json!({
                "id": "locked_10_minutes_6_percents",
                "cap": ["100000", "100000000000"],
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "600000",
                        "apy": {
                            "default": ["6", 2],
                        },
                    }
                },
                "is_enabled": true,
                "score_cap": 0,
            }),
            RegisterProductCommand::Locked5Minutes60000Percents => json!({
                "id": "flexible_5_minutes_60000_percents",
                "cap": ["10000", "100000000000"],
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "300000",
                        "apy": {
                            "default": ["60000", 2],
                        },
                    }
                },
                "is_enabled": true,
                "score_cap": 0,
            }),
            RegisterProductCommand::Locked10Minutes60000Percents => json!({
                "id": "flexible_10_minutes_60000_percents",
                "cap": ["100000", "100000000000"],
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "600000",
                        "apy": {
                            "default": ["60000", 2],
                        },
                    }
                },
                "is_enabled": true,
                "score_cap": 0,
            }),
            RegisterProductCommand::Locked10Minutes6PercentsTopUp => json!({
                "id": "locked_10_minutes_6_percents_top_up",
                "cap": ["100000", "100000000000"],
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "600000",
                        "apy": {
                            "default": ["6", 2],
                        },
                    }
                },
                "is_enabled": true,
                "score_cap": 0,
            }),
            RegisterProductCommand::Locked10Minutes6PercentsWithFixedWithdrawFee => json!({
                "id": "locked_10_minutes_6_percents_with_fixed_withdraw_fee",
                "cap": ["100000", "100000000000"],
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "600000",
                        "apy": {
                            "default": ["6", 2],
                        },
                    }
                },
                "withdrawal_fee": {
                    "type": "fix",
                    "data": "1000",
                },
                "is_enabled": true,
                "score_cap": 0,
            }),
            RegisterProductCommand::Locked10Minutes6PercentsWithPercentWithdrawFee => json!({
                "id": "locked_10_minutes_6_percents_with_percent_withdraw_fee",
                "cap": ["100000", "100000000000"],
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "600000",
                        "apy": {
                            "default": ["6", 2],
                        },
                    }
                },
                "withdrawal_fee": {
                    "type": "percent",
                    "data": ["1", 2],
                },
                "is_enabled": true,
                "score_cap": 0,
            }),
            RegisterProductCommand::Locked10Minutes20000ScoreCap => json!({
                "id": "locked_10_minutes_20000_score_cap",
                "cap": ["100000", "100000000000"],
                "terms": {
                    "type": "score_based",
                    "data": {
                        "lockup_term": "600000",
                        "base_apy": {
                            "default": ["0", 0],
                        },
                        "score_cap": 20000,
                    }
                },
                "is_enabled": true,
            }),
        }
    }

    pub(crate) fn id(&self) -> String {
        self.json()
            .as_object()
            .unwrap()
            .get("id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    }
}

use nitka::near_sdk::serde_json::{json, Value};

use crate::product::RegisterProductCommand;

impl RegisterProductCommand {
    pub(crate) fn json_legacy_for_premium(&self, public_key: String) -> Value {
        let mut json = self.json_legacy();
        if let Value::Object(obj) = &mut json {
            obj.insert("public_key".to_string(), Value::String(public_key));
        }
        json
    }

    pub(crate) fn json_legacy(&self) -> Value {
        match self {
            RegisterProductCommand::Locked12Months12Percents => json!({
                "id": "locked_12_months_12_percents",
                "apy_default": ["12", 2],
                "cap_min": "100000",
                "cap_max": "100000000000",
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "31556952000",
                        "allows_top_up": false,
                        "allows_restaking": false,
                    }
                },
                "is_enabled": true,
                "score_cap": 0,
            }),
            RegisterProductCommand::Locked6Months6Percents => json!({
                "id": "locked_6_months_6_percents",
                "apy_default": ["6", 2],
                "cap_min": "100000",
                "cap_max": "100000000000",
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "15778476000",
                        "allows_top_up": false,
                        "allows_restaking": false,
                    }
                },
                "is_enabled": true,
                "score_cap": 0,
            }),
            RegisterProductCommand::Flexible6Months6Percents => json!({
                "id": "flexible_6_months_6_percents",
                "apy_default": ["12", 2],
                "apy_fallback": ["6", 2],
                "cap_min": "100000",
                "cap_max": "100000000000",
                "terms": {
                    "type": "flexible",
                },
                "is_enabled": true,
                "score_cap": 0,
            }),

            RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee => json!({
                "id": "locked_6_months_6_percents_with_withdraw_fee",
                "apy_default": ["6", 2],
                "cap_min": "100000",
                "cap_max": "100000000000",
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "15778476000",
                        "allows_top_up": false,
                        "allows_restaking": false,
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
                "apy_default": ["6", 2],
                "cap_min": "100000",
                "cap_max": "100000000000",
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "600000",
                        "allows_top_up": false,
                        "allows_restaking": true,
                    }
                },
                "is_enabled": true,
                "score_cap": 0,
            }),
            RegisterProductCommand::Locked5Minutes60000Percents => json!({
                "id": "flexible_5_minutes_60000_percents",
                "apy_default": ["60000", 2],
                "cap_min": "10000",
                "cap_max": "100000000000",
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "300000",
                        "allows_top_up": false,
                        "allows_restaking": true,
                    }
                },
                "is_enabled": true,
                "score_cap": 0,
            }),
            RegisterProductCommand::Locked10Minutes60000Percents => json!({
                "id": "flexible_10_minutes_60000_percents",
                "apy_default": ["60000", 2],
                "cap_min": "100000",
                "cap_max": "100000000000",
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "600000",
                        "allows_top_up": false,
                        "allows_restaking": true,
                    }
                },
                "is_enabled": true,
                "score_cap": 0,
            }),
            RegisterProductCommand::Locked10Minutes6PercentsTopUp => json!({
                "id": "locked_10_minutes_6_percents_top_up",
                "apy_default": ["6", 2],
                "cap_min": "100000",
                "cap_max": "100000000000",
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "600000",
                        "allows_top_up": true,
                        "allows_restaking": true,
                    }
                },
                "is_enabled": true,
                "score_cap": 0,
            }),
            RegisterProductCommand::Locked10Minutes6PercentsWithFixedWithdrawFee => json!({
                "id": "locked_10_minutes_6_percents_with_fixed_withdraw_fee",
                "apy_default": ["6", 2],
                "cap_min": "100000",
                "cap_max": "100000000000",
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "600000",
                        "allows_top_up": false,
                        "allows_restaking": false,
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
                "apy_default": ["6", 2],
                "cap_min": "100000",
                "cap_max": "100000000000",
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "600000",
                        "allows_top_up": false,
                        "allows_restaking": false,
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
                "apy_default": ["0", 0],
                "cap_min": "100000",
                "cap_max": "100000000000",
                "terms": {
                    "type": "fixed",
                    "data": {
                        "lockup_term": "600000",
                        "allows_top_up": false,
                        "allows_restaking": false,
                    }
                },
                "is_enabled": true,
                "score_cap": 20000,
            }),
        }
    }
}

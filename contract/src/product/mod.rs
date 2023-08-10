pub mod api;
pub mod command;
pub mod model;
pub mod view;

#[cfg(test)]
pub(crate) mod tests {
    use near_sdk::json_types::{Base64VecU8, U128, U64};

    use crate::common::UDecimal;
    use crate::product::command::RegisterProductCommand;
    use crate::product::model::{Apy, Cap, DowngradableApy, Product};

    fn get_premium_product_public_key() -> Vec<u8> {
        vec![
            33, 80, 163, 149, 64, 30, 150, 45, 68, 212, 97, 122, 213, 118, 189, 174, 239, 109,
            48, 82, 50, 35, 197, 176, 50, 211, 183, 128, 207, 1, 8, 68,
        ]
    }

    pub(crate) fn get_product() -> Product {
        Product {
            id: "product".to_string(),
            lockup_term: 365 * 24 * 60 * 60 * 1000,
            is_refillable: false,
            apy: Apy::Constant(UDecimal::new(12, 2)),
            cap: Cap {
                min: 100,
                max: 100_000_000_000,
            },
            is_restakable: false,
            withdrawal_fee: None,
            public_key: None,
        }
    }

    pub(crate) fn get_register_product_command() -> RegisterProductCommand {
        RegisterProductCommand {
            id: "product".to_string(),
            lockup_term: U64(365 * 24 * 60 * 60 * 1000),
            apy_default: (U128(12), 2),
            apy_fallback: None,
            cap_min: U128(100),
            cap_max: U128(100_000_000_000),
            is_restakable: false,
            is_refillable: false,
            withdrawal_fee: None,
            public_key: None,
        }
    }

    pub(crate) fn get_premium_product() -> Product {
        Product {
            id: "product_premium".to_string(),
            lockup_term: 365 * 24 * 60 * 60 * 1000,
            is_refillable: false,
            apy: Apy::Downgradable(DowngradableApy {
                default: UDecimal::new(20, 2),
                fallback: UDecimal::new(10, 2),
            }),
            cap: Cap {
                min: 100,
                max: 100_000_000_000,
            },
            is_restakable: false,
            withdrawal_fee: None,
            public_key: Some(get_premium_product_public_key()),
        }
    }

    pub(crate) fn get_register_premium_product_command() -> RegisterProductCommand {
        RegisterProductCommand {
            id: "product_premium".to_string(),
            lockup_term: U64(365 * 24 * 60 * 60 * 1000),
            apy_default: (U128(20), 2),
            apy_fallback: Some((U128(10), 2)),
            cap_min: U128(100),
            cap_max: U128(100_000_000_000),
            is_restakable: false,
            is_refillable: false,
            withdrawal_fee: None,
            public_key: Some(Base64VecU8(get_premium_product_public_key())),
        }
    }

    #[test]
    fn assert_cap_in_bounds() {
        get_product().assert_cap(200);
    }

    #[test]
    #[should_panic(expected = "Total amount is out of product bounds: [100..100000000000]")]
    fn assert_cap_less_than_min() {
        get_product().assert_cap(10);
    }

    #[test]
    #[should_panic(expected = "Total amount is out of product bounds: [100..100000000000]")]
    fn assert_cap_more_than_max() {
        get_product().assert_cap(500_000_000_000);
    }
}

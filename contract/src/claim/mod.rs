pub mod api;

#[cfg(test)]
mod tests {
    use near_sdk::json_types::{U128, U64};
    use near_sdk::PromiseOrValue;
    use near_sdk::test_utils::accounts;
    use crate::claim::api::ClaimApi;
    use crate::common::tests::Context;
    use crate::common::U32;
    use crate::jar::api::JarApi;
    use crate::jar::model::JarTicket;
    use crate::product::api::ProductApi;
    use crate::product::tests::get_register_product_command;

    #[test]
    fn claim_total_when_nothing_to_claim() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(
            1,
            |context| context.contract.register_product(get_register_product_command()),
        );

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: get_register_product_command().id,
                valid_until: U64(0),
            },
            U128(100_000_000),
            None,
        );

        context.switch_account(&alice.clone());
        let result = context.contract.claim_total();

        if let PromiseOrValue::Value(value) = result {
            assert_eq!(0, value.0);
        } else {
            panic!();
        }
    }

    #[test]
    fn claim_partially_when_having_tokens_to_claim() {
        let alice = accounts(0);
        let admin = accounts(1);

        let mut context = Context::new(admin.clone());

        context.switch_account(&admin);
        context.with_deposit_yocto(
            1,
            |context| context.contract.register_product(get_register_product_command()),
        );

        context.switch_account_to_owner();
        context.contract.create_jar(
            alice.clone(),
            JarTicket {
                product_id: get_register_product_command().id,
                valid_until: U64(0),
            },
            U128(100_000_000_000),
            None,
        );

        context.set_block_timestamp_in_days(365);

        context.switch_account(&alice.clone());
        context.contract.claim_jars(vec![0], Some(U128(100)));

        let jar = context.contract.get_jar(U32(0));
        assert_eq!(100, jar.claimed_balance.0);
    }
}
pub mod api;

#[cfg(test)]
mod tests {
    use near_sdk::{json_types::U128, test_utils::accounts, PromiseOrValue};

    use crate::{
        claim::api::ClaimApi,
        common::{tests::Context, UDecimal, U32},
        jar::{api::JarApi, model::Jar},
        product::{
            model::{Apy, Product},
            tests::YEAR_IN_MS,
        },
    };

    #[test]
    fn claim_total_when_nothing_to_claim() {
        let alice = accounts(0);
        let admin = accounts(1);

        let product = generate_product();
        let jar = Jar::generate(0, &alice, &product.id).principal(100_000_000);
        let mut context = Context::new(admin).with_products(&[product]).with_jars(&[jar]);

        context.switch_account(&alice);
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

        let product = generate_product();
        let reference_jar = Jar::generate(0, &alice, &product.id).principal(100_000_000_000);
        let mut context = Context::new(admin)
            .with_products(&[product])
            .with_jars(&[reference_jar.clone()]);

        context.set_block_timestamp_in_days(365);

        context.switch_account(&alice);
        context.contract.claim_jars(vec![reference_jar.index], Some(U128(100)));

        let jar = context.contract.get_jar(U32(reference_jar.index));
        assert_eq!(100, jar.claimed_balance.0);
    }

    fn generate_product() -> Product {
        Product::generate("product")
            .enabled(true)
            .lockup_term(YEAR_IN_MS)
            .apy(Apy::Constant(UDecimal::new(12, 2)))
    }
}

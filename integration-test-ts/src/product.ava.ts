import { createTest } from "./setup";
import { Preset } from "./state";
import { Product } from "./types";
import { hasError, hasNoErrors, sweat } from "./utils";

const test = createTest(Preset.Default);

test('Register product by authorized account', async t => {
  const { manager, jars } = t.context.accounts;

  const result = await manager.callRaw(jars, 'register_product', {
    product: {
      id: "test_product",
      cap: [sweat(0).toString(), sweat(100_000).toString()],
      terms: {
        type: "tiered_score_based",
        data: {
          score_cap: {
            default: 20_000,
            fallback: 10_000
          },
          lockup_term: "31536000000"
        }
      },
      withdrawal_fee: null,
      public_key: null,
      is_enabled: true
    }
  }, {
    attachedDeposit: 1n,
  });
  t.assert(hasNoErrors(result));

  const products: Product[] = await jars.view('get_products');
  t.is(products.length, 1);
});

test('Register product by unauthorized account', async t => {
  const { alice, jars } = t.context.accounts;

  const result = await alice.callRaw(jars, 'register_product', {
    product: {
      id: "test_product",
      cap: [sweat(10).toString(), sweat(200).toString],
      terms: {
        type: "tiered_score_based",
        data: {
          score_cap: {
            default: 30_000,
            fallback: 5_000
          },
          lockup_term: "31536000000"
        }
      },
      withdrawal_fee: null,
      public_key: null,
      is_enabled: true
    }
  }, {
    attachedDeposit: 1n,
  });
  t.assert(hasError(result, 'Can be performed only by admin'));
});

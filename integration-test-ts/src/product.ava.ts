import { NearAccount, TransactionResult } from "near-workspaces";
import { createTest } from "./setup";
import { Product } from "./types";
import { hasError, hasNoErrors } from "./utils";

const test = createTest();

test('Register product by authorized account', async t => {
  const { manager, jars } = t.context.accounts;

  const result = await register_product(jars, manager);
  t.assert(hasNoErrors(result));

  const products: Product[] = await jars.view('get_products');
  t.is(products.length, 1);
});

test('Register product by unauthorized account', async t => {
  const { alice, jars } = t.context.accounts;

  const result = await register_product(jars, alice);
  t.assert(hasError(result, 'Can be performed only by admin'));
});

async function register_product(jars: NearAccount, user: NearAccount): Promise<TransactionResult> {
  return await user.callRaw(jars, 'register_product', {
    product: {
      id: "test_product",
      cap: ["0", "1000000000000000000000000000"],
      terms: {
        type: "tiered_score_based",
        data: {
          score_cap: {
            default: 20000,
            fallback: 10000
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
}

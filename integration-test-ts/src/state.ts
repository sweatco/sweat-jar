import { readFile, writeFile } from "fs/promises";
import { NearAccount, StateItem } from "near-workspaces";
import { sweat } from "./utils";
import { createBaseContext } from "./setup";
import { Data } from "near-workspaces/dist/record";

export async function init() {
  const context = await createBaseContext();
  const { root, token, manager, treasury, jarsLegacy } = context.accounts;
  await setupJarsContract(root, token, manager, treasury, jarsLegacy);
}

export enum Preset {
  Default = 'default',
  WithTieredScoreBasedProduct = 'with_tiered_score_based_product',
}

function getStateFileName(preset: Preset): string {
  return `jars_${preset}`;
}

async function setupJarsContract(
  root: NearAccount,
  token: NearAccount,
  manager: NearAccount,
  treasury: NearAccount,
  jarsLegacy: NearAccount,
) {
  const contract = await root.devDeploy(
    '../res/sweat_jar.wasm',
    {
      method: 'init',
      args: {
        token_account_id: token.accountId,
        fee_account_id: treasury.accountId,
        manager: manager.accountId,
        previous_version_account_id: jarsLegacy.accountId,
      }
    }
  );

  await storeState(contract, Preset.Default);

  await manager.callRaw(contract, 'register_product', {
    product: {
      id: "test_product",
      cap: [sweat(0).toString(), sweat(1_000_000_000).toString()],
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

  await storeState(contract, Preset.WithTieredScoreBasedProduct);
}

async function storeState(contract: NearAccount, preset: Preset) {
  const state = await contract.viewStateRaw();
  const filePath = getStateFileName(preset);

  await writeFile(filePath, JSON.stringify(state), 'utf-8');
}

export async function patchState(account: NearAccount, preset: Preset) {
  const filePath = getStateFileName(preset);
  const state: StateItem[] = JSON.parse(await readFile(filePath, 'utf-8'));

  const dataRecords: Array<Data> = state.map(item => {
    return {
      Data: {
        account_id: account.accountId,
        data_key: item.key,
        value: item.value,
      }
    };
  })

  await account.patchStateRecords({
    records: dataRecords
  });
}

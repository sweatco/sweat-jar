import { type NearAccount, Worker } from 'near-workspaces';
import anyTest, { type TestFn } from 'ava'

export type Context = {
  worker: Worker;
  accounts: Record<string, NearAccount>;
}

export function createTest(): TestFn<Context> {
  const test = anyTest as TestFn<Context>;

  test.before(async t => {
    t.context = await prepareContext();
  })

  test.after.always(async t => {
    await t.context.worker.tearDown();
  })

  return test;
}

async function prepareContext(): Promise<Context> {
  const worker = await Worker.init();
  const root = worker.rootAccount;

  const alice = await root.createSubAccount('alice');
  const manager = await root.createSubAccount('manager');
  const treasury = await root.createSubAccount('treasury');
  const token = await root.devCreateAccount();
  const jarsLegacy = await root.createSubAccount('legacy_jars');

  const jars = await root.devDeploy('../res/sweat_jar.wasm', {
    method: 'init',
    args: {
      token_account_id: token.accountId,
      fee_account_id: treasury.accountId,
      manager: manager.accountId,
      previous_version_account_id: jarsLegacy.accountId,
    }
  });

  const accounts = { root, alice, manager, jars, treasury, token, jarsLegacy } as const;

  return {
    worker,
    accounts
  }
}

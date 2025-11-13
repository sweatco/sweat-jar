import { type NearAccount, Worker } from 'near-workspaces';
import anyTest, { type TestFn } from 'ava'
import { patchState, Preset } from './state';

export type Context = {
  worker: Worker;
  accounts: Record<string, NearAccount>;
}

export function createTest(preset: Preset = Preset.Default): TestFn<Context> {
  const test = anyTest as TestFn<Context>;

  test.before(async t => {
    t.context = await prepareContext(preset);
  })

  test.after.always(async t => {
    await t.context.worker.tearDown();
  })

  return test;
}

async function prepareContext(preset: Preset): Promise<Context> {
  const context = await createBaseContext();
  const { root } = context.accounts;

  const jars = await root.devDeploy('../res/sweat_jar.wasm');
  await patchState(jars, preset);

  return {
    worker: context.worker,
    accounts: { jars, ...context.accounts }
  }
}

export async function createBaseContext(): Promise<Context> {
  const worker = await Worker.init();
  const root = worker.rootAccount;

  const alice = await root.createSubAccount('alice');
  const manager = await root.createSubAccount('manager');
  const treasury = await root.createSubAccount('treasury');
  const token = await root.createSubAccount('token');
  const jarsLegacy = await root.createSubAccount('legacy_jars');

  const accounts = { root, alice, manager, treasury, token, jarsLegacy } as const;

  return {
    worker,
    accounts
  }
}

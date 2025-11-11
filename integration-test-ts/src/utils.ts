import { TransactionResult } from "near-workspaces";
import { Error } from "./types";

export function sweat(n: number): bigint {
  return BigInt(n) * (10n ** 18n);
}

export function hasError(result: TransactionResult, error: string): boolean {
  return result.failures.findIndex(item => {
    const errorKind = JSON.stringify((item as unknown as Error).ActionError.kind);

    return errorKind.includes(error);
  }) >= 0;
}

export function hasNoErrors(result: TransactionResult): boolean {
  return result.failures.length === 0;
}

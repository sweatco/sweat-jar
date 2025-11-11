export type Product = {
  id: string;
  cap: [string, string];
  terms: {
    type: 'fixed' | 'flexible' | 'score_based' | 'tiered_score_based';
    data: {
      score_cap: {
        default: number;
        fallback: number;
      };
      lockup_term: string;
    };
  };
  withdrawal_fee: string | null;
  public_key: string | null;
  is_enabled: boolean;
};

export type Error = {
  ActionError: {
    index: number,
    kind: object
  }
}

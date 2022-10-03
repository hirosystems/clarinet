json.extract! vault, :id, :id, :owner, :collateral, :collateral_type, :collateral_token, :stacked_tokens, :stacker_name, :revoked_stacking, :auto_payoff, :debt, :created_at_block_height, :updated_at_block_height, :stability_fee_accrued, :stability_fee_last_accrued, :is_liquidated, :auction_ended, :leftover_collateral, :created_at, :updated_at
json.url vault_url(vault, format: :json)

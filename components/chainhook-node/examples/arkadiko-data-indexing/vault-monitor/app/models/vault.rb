class Vault < ApplicationRecord

    def Vault.create_from_onchain_event(params)
        Vault.create({
            :onchain_id => params["id"],
            :owner => params["owner"],
            :collateral => params["collateral"],
            :collateral_type => params["collateral-type"],
            :collateral_token => params["collateral-token"],
            :stacked_tokens => params["stacked-tokens"],
            :stacker_name => params["stacker-name"],
            :revoked_stacking => params["revoked-stacking"],
            :auto_payoff => params["auto-payoff"],
            :debt => params["debt"],
            :created_at_block_height => params["created-at-block-height"],
            :updated_at_block_height => params["updated-at-block-height"],
            :stability_fee_accrued => params["stability-fee-accrued"],
            :stability_fee_last_accrued => params["stability-fee-last-accrued"],
            :is_liquidated => params["is-liquidated"],
            :auction_ended => params["auction-ended"],
            :leftover_collateral => params["leftover-collateral"],
        })
    end

    def Vault.update_attributes_from_onchain_event(params)
        Vault
            .where(:onchain_id => params["id"])
            .update_all(
                :owner => params["owner"],
                :collateral => params["collateral"],
                :collateral_type => params["collateral-type"],
                :collateral_token => params["collateral-token"],
                :stacked_tokens => params["stacked-tokens"],
                :stacker_name => params["stacker-name"],
                :revoked_stacking => params["revoked-stacking"],
                :auto_payoff => params["auto-payoff"],
                :debt => params["debt"],
                :created_at_block_height => params["created-at-block-height"],
                :updated_at_block_height => params["updated-at-block-height"],
                :stability_fee_accrued => params["stability-fee-accrued"],
                :stability_fee_last_accrued => params["stability-fee-last-accrued"],
                :is_liquidated => params["is-liquidated"],
                :auction_ended => params["auction-ended"],
                :leftover_collateral => params["leftover-collateral"],
            )
    end

end

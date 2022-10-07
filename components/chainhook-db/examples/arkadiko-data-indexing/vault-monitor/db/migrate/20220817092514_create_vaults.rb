class CreateVaults < ActiveRecord::Migration[7.0]
  def change
    create_table :vaults do |t|
      t.integer :onchain_id
      t.string :owner
      t.integer :collateral
      t.string :collateral_type
      t.string :collateral_token
      t.integer :stacked_tokens
      t.string :stacker_name
      t.boolean :revoked_stacking
      t.boolean :auto_payoff
      t.integer :debt
      t.integer :created_at_block_height
      t.integer :updated_at_block_height
      t.integer :stability_fee_accrued
      t.integer :stability_fee_last_accrued
      t.boolean :is_liquidated
      t.boolean :auction_ended
      t.integer :leftover_collateral

      t.timestamps
    end
  end
end

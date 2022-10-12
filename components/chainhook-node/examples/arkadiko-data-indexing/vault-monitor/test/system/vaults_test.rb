require "application_system_test_case"

class VaultsTest < ApplicationSystemTestCase
  setup do
    @vault = vaults(:one)
  end

  test "visiting the index" do
    visit vaults_url
    assert_selector "h1", text: "Vaults"
  end

  test "should create vault" do
    visit vaults_url
    click_on "New vault"

    check "Auction ended" if @vault.auction_ended
    check "Auto payoff" if @vault.auto_payoff
    fill_in "Collateral", with: @vault.collateral
    fill_in "Collateral token", with: @vault.collateral_token
    fill_in "Collateral type", with: @vault.collateral_type
    fill_in "Created at block height", with: @vault.created_at_block_height
    fill_in "Debt", with: @vault.debt
    fill_in "Id", with: @vault.id
    check "Is liquidated" if @vault.is_liquidated
    fill_in "Leftover collateral", with: @vault.leftover_collateral
    fill_in "Owner", with: @vault.owner
    check "Revoked stacking" if @vault.revoked_stacking
    fill_in "Stability fee accrued", with: @vault.stability_fee_accrued
    fill_in "Stability fee last accrued", with: @vault.stability_fee_last_accrued
    fill_in "Stacked tokens", with: @vault.stacked_tokens
    fill_in "Stacker name", with: @vault.stacker_name
    fill_in "Updated at block height", with: @vault.updated_at_block_height
    click_on "Create Vault"

    assert_text "Vault was successfully created"
    click_on "Back"
  end

  test "should update Vault" do
    visit vault_url(@vault)
    click_on "Edit this vault", match: :first

    check "Auction ended" if @vault.auction_ended
    check "Auto payoff" if @vault.auto_payoff
    fill_in "Collateral", with: @vault.collateral
    fill_in "Collateral token", with: @vault.collateral_token
    fill_in "Collateral type", with: @vault.collateral_type
    fill_in "Created at block height", with: @vault.created_at_block_height
    fill_in "Debt", with: @vault.debt
    fill_in "Id", with: @vault.id
    check "Is liquidated" if @vault.is_liquidated
    fill_in "Leftover collateral", with: @vault.leftover_collateral
    fill_in "Owner", with: @vault.owner
    check "Revoked stacking" if @vault.revoked_stacking
    fill_in "Stability fee accrued", with: @vault.stability_fee_accrued
    fill_in "Stability fee last accrued", with: @vault.stability_fee_last_accrued
    fill_in "Stacked tokens", with: @vault.stacked_tokens
    fill_in "Stacker name", with: @vault.stacker_name
    fill_in "Updated at block height", with: @vault.updated_at_block_height
    click_on "Update Vault"

    assert_text "Vault was successfully updated"
    click_on "Back"
  end

  test "should destroy Vault" do
    visit vault_url(@vault)
    click_on "Destroy this vault", match: :first

    assert_text "Vault was successfully destroyed"
  end
end

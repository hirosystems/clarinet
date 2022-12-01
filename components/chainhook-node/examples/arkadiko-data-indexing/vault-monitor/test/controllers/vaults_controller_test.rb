require "test_helper"

class VaultsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @vault = vaults(:one)
  end

  test "should get index" do
    get vaults_url
    assert_response :success
  end

  test "should get new" do
    get new_vault_url
    assert_response :success
  end

  test "should create vault" do
    assert_difference("Vault.count") do
      post vaults_url, params: { vault: { auction_ended: @vault.auction_ended, auto_payoff: @vault.auto_payoff, collateral: @vault.collateral, collateral_token: @vault.collateral_token, collateral_type: @vault.collateral_type, created_at_block_height: @vault.created_at_block_height, debt: @vault.debt, id: @vault.id, is_liquidated: @vault.is_liquidated, leftover_collateral: @vault.leftover_collateral, owner: @vault.owner, revoked_stacking: @vault.revoked_stacking, stability_fee_accrued: @vault.stability_fee_accrued, stability_fee_last_accrued: @vault.stability_fee_last_accrued, stacked_tokens: @vault.stacked_tokens, stacker_name: @vault.stacker_name, updated_at_block_height: @vault.updated_at_block_height } }
    end

    assert_redirected_to vault_url(Vault.last)
  end

  test "should show vault" do
    get vault_url(@vault)
    assert_response :success
  end

  test "should get edit" do
    get edit_vault_url(@vault)
    assert_response :success
  end

  test "should update vault" do
    patch vault_url(@vault), params: { vault: { auction_ended: @vault.auction_ended, auto_payoff: @vault.auto_payoff, collateral: @vault.collateral, collateral_token: @vault.collateral_token, collateral_type: @vault.collateral_type, created_at_block_height: @vault.created_at_block_height, debt: @vault.debt, id: @vault.id, is_liquidated: @vault.is_liquidated, leftover_collateral: @vault.leftover_collateral, owner: @vault.owner, revoked_stacking: @vault.revoked_stacking, stability_fee_accrued: @vault.stability_fee_accrued, stability_fee_last_accrued: @vault.stability_fee_last_accrued, stacked_tokens: @vault.stacked_tokens, stacker_name: @vault.stacker_name, updated_at_block_height: @vault.updated_at_block_height } }
    assert_redirected_to vault_url(@vault)
  end

  test "should destroy vault" do
    assert_difference("Vault.count", -1) do
      delete vault_url(@vault)
    end

    assert_redirected_to vaults_url
  end
end

# This file is auto-generated from the current state of the database. Instead
# of editing this file, please use the migrations feature of Active Record to
# incrementally modify your database, and then regenerate this schema definition.
#
# This file is the source Rails uses to define your schema when running `bin/rails
# db:schema:load`. When creating a new database, `bin/rails db:schema:load` tends to
# be faster and is potentially less error prone than running all of your
# migrations from scratch. Old migrations may fail to apply correctly if those
# migrations use external dependencies or application code.
#
# It's strongly recommended that you check this file into your version control system.

ActiveRecord::Schema[7.0].define(version: 2022_08_17_092514) do
  create_table "vaults", force: :cascade do |t|
    t.integer "onchain_id"
    t.string "owner"
    t.integer "collateral"
    t.string "collateral_type"
    t.string "collateral_token"
    t.integer "stacked_tokens"
    t.string "stacker_name"
    t.boolean "revoked_stacking"
    t.boolean "auto_payoff"
    t.integer "debt"
    t.integer "created_at_block_height"
    t.integer "updated_at_block_height"
    t.integer "stability_fee_accrued"
    t.integer "stability_fee_last_accrued"
    t.boolean "is_liquidated"
    t.boolean "auction_ended"
    t.integer "leftover_collateral"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
  end

end

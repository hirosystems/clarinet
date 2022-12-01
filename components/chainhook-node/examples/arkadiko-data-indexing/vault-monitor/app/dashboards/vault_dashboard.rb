require "administrate/base_dashboard"

class VaultDashboard < Administrate::BaseDashboard
  # ATTRIBUTE_TYPES
  # a hash that describes the type of each of the model's fields.
  #
  # Each different type represents an Administrate::Field object,
  # which determines how the attribute is displayed
  # on pages throughout the dashboard.
  ATTRIBUTE_TYPES = {
    id: Field::Number,
    auction_ended: Field::Boolean,
    auto_payoff: Field::Boolean,
    collateral: Field::Number,
    collateral_token: Field::String,
    collateral_type: Field::String,
    created_at_block_height: Field::Number,
    debt: Field::Number,
    is_liquidated: Field::Boolean,
    leftover_collateral: Field::Number,
    onchain_id: Field::Number,
    owner: Field::String,
    revoked_stacking: Field::Boolean,
    stability_fee_accrued: Field::Number,
    stability_fee_last_accrued: Field::Number,
    stacked_tokens: Field::Number,
    stacker_name: Field::String,
    updated_at_block_height: Field::Number,
    created_at: Field::DateTime,
    updated_at: Field::DateTime,
  }.freeze

  # COLLECTION_ATTRIBUTES
  # an array of attributes that will be displayed on the model's index page.
  #
  # By default, it's limited to four items to reduce clutter on index pages.
  # Feel free to add, remove, or rearrange items.
  COLLECTION_ATTRIBUTES = %i[
    onchain_id
    created_at_block_height
    collateral_token
    collateral_type
    collateral
    debt
    stacked_tokens
    owner
  ].freeze

  # SHOW_PAGE_ATTRIBUTES
  # an array of attributes that will be displayed on the model's show page.
  SHOW_PAGE_ATTRIBUTES = %i[
    auction_ended
    auto_payoff
    collateral
    collateral_token
    collateral_type
    created_at_block_height
    debt
    is_liquidated
    leftover_collateral
    onchain_id
    owner
    revoked_stacking
    stability_fee_accrued
    stability_fee_last_accrued
    stacked_tokens
    stacker_name
    updated_at_block_height
  ].freeze

  # FORM_ATTRIBUTES
  # an array of attributes that will be displayed
  # on the model's form (`new` and `edit`) pages.
  FORM_ATTRIBUTES = %i[
    auction_ended
    auto_payoff
    collateral
    collateral_token
    collateral_type
    created_at_block_height
    debt
    is_liquidated
    leftover_collateral
    onchain_id
    owner
    revoked_stacking
    stability_fee_accrued
    stability_fee_last_accrued
    stacked_tokens
    stacker_name
    updated_at_block_height
  ].freeze

  # COLLECTION_FILTERS
  # a hash that defines filters that can be used while searching via the search
  # field of the dashboard.
  #
  # For example to add an option to search for open resources by typing "open:"
  # in the search field:
  #
  #   COLLECTION_FILTERS = {
  #     open: ->(resources) { resources.where(open: true) }
  #   }.freeze
  COLLECTION_FILTERS = {}.freeze

  # disable 'edit' and 'destroy' links
  def valid_action?(name, resource = resource_class)
    %w[edit destroy].exclude?(name.to_s) && super
  end

  def default_sorting_attribute
    :age
  end
  
  def default_sorting_direction
    :desc
  end
  
  def display_resource(vault)
    "Vault ##{vault.onchain_id}"
  end
end

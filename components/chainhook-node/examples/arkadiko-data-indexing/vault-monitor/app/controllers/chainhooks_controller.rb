class ChainhooksController < ApplicationController
  skip_before_action :verify_authenticity_token
  wrap_parameters false

  def vaults
    payload = JSON.parse request.body.read
    payload["apply"].each do |block|
      block["transactions"].each do |transaction|
        transaction["metadata"]["receipt"]["events"].each do |event|
          next if event["type"] != "SmartContractEvent"
          event_data = event["data"]["value"] 
          next if event_data.nil? || event_data["type"] != "vault"
          vault_event_data = event_data["data"]
          if event_data["action"] == "created"
            Vault.create_from_onchain_event(vault_event_data)
          elsif ["deposit", "burn", "close", "mint"].include? event_data["action"]
            Vault.update_attributes_from_onchain_event(vault_event_data)
          else
            p "Unknown event type #{event_data["action"]}"
          end
        end
      end
    end
    respond_to do |format|
      format.json { head :no_content, status: :ok }        
    end
  end
end

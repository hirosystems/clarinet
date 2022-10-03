Rails.application.routes.draw do
  namespace :admin do
      resources :vaults, only: %i(index show)
      root to: "vaults#index"
  end

  post '/chainhooks/v1/vaults', to: 'chainhooks#vaults' 
end

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::Item;

use crate::msg::AssetPrice;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub asset_prices: Vec<AssetPrice>,
}

pub const STATE: Item<State> = Item::new("state");

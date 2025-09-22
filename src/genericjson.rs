pub struct MarketData {
    pub price: u32,
    pub sale: u32,
    pub name: String,
    pub description: String,
    pub emblems: u32,
}

impl Default for MarketData {
    fn default() -> Self {
        Self {
            price: 0,
            sale: 0,
            name: "Default".to_string(),
            description: "Default".to_string(),
            emblems: 0,
        }
    }
}

impl MarketData {
    pub fn read_json(document: &serde_json::Value) -> std::result::Result<Self, String> {
        let market_data = &document["market_data"];
        if !market_data.is_object() {
            return Err("market_data is not an object!".to_string());
        }

        let name = if let Some(name) = market_data["name"].as_str() {
            Ok(name.to_string())
        } else {
            Err("market_data's name is not a string!")
        }?;

        let description = if let Some(name) = market_data["description"].as_str() {
            Ok(name.to_string())
        } else {
            Err("market_data's description is not a string!")
        }?;

        let price = if let Some(price) = market_data["price"].as_i64() {
            Ok(price as u32)
        } else {
            Err("market_data's price is not an integer!")
        }?;

        let sale = if let Some(sale) = market_data["sale"].as_i64() {
            Ok(sale as u32)
        } else {
            Err("market_data's sale is not an integer!")
        }?;

        let emblems = if let Some(emblems) = market_data["emblems"].as_i64() {
            Ok(emblems as u32)
        } else {
            Err("market_data's sale is not an integer!")
        }?;

        Ok(MarketData {
            price,
            sale,
            name,
            description,
            emblems,
        })
    }
}

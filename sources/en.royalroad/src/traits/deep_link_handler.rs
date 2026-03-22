use buny::{DeepLinkHandler, DeepLinkResult, Result, alloc::String};

use crate::RoyalRoad;

impl DeepLinkHandler for RoyalRoad {
	fn handle_deep_link(&self, _url: String) -> Result<Option<DeepLinkResult>> {
		Ok(Some(DeepLinkResult::Novel {
			key: String::from("novel_key"),
		}))
	}
}

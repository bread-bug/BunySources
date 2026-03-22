use buny::{
	DynamicListings, Listing, Result,
	alloc::{String, Vec, vec},
};

use crate::RoyalRoad;

impl DynamicListings for RoyalRoad {
	fn get_dynamic_listings(&self) -> Result<Vec<Listing>> {
		Ok(vec![Listing {
			id: String::from("listing"),
			name: String::from("Listing"),
			kind: buny::ListingKind::List,
		}])
	}
}

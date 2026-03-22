use buny::{
	Home, HomeComponent, HomeLayout, HomePartialResult, Listing, ListingProvider, Result,
	alloc::{string::ToString, vec},
	imports::std::send_partial_result,
};

use crate::RoyalRoad;

// Send initial layout structure
pub fn send_initial_layout() {
	send_partial_result(&HomePartialResult::Layout(HomeLayout {
		components: vec![
			HomeComponent {
				title: Some("Best Rated Novels".to_string()),
				subtitle: None,
				value: buny::HomeComponentValue::empty_details(),
			},
			HomeComponent {
				title: Some("Trending Novels".to_string()),
				subtitle: None,
				value: buny::HomeComponentValue::empty_details(),
			},
			HomeComponent {
				title: Some("Rising Stars".to_string()),
				subtitle: None,
				value: buny::HomeComponentValue::empty_stack(),
			},
			HomeComponent {
				title: Some("Newest Novels".to_string()),
				subtitle: Some("Novels based on most reviews!".to_string()),
				value: buny::HomeComponentValue::empty_scroller(),
			},
			HomeComponent {
				title: Some("Latest Updates".to_string()),
				value: buny::HomeComponentValue::empty_scroller(),
				..Default::default()
			},
		],
	}));
}

// use the home trait to implement a home page for a source
// where possible, try to replicate the associated web page's layout
impl Home for RoyalRoad {
	fn get_home(&self) -> Result<HomeLayout> {
		send_initial_layout();

		let listing = Listing {
			id: "best-rated".into(),
			name: "".into(),
			..Default::default()
		};
		let listing2 = Listing {
			id: "trending".into(),
			name: "".into(),
			..Default::default()
		};

		let listing3 = Listing {
			id: "rising-stars".into(),
			name: "".into(),
			..Default::default()
		};

		let listing4 = Listing {
			id: "new-releases".into(),
			name: "".into(),
			..Default::default()
		};

		let listing5 = Listing {
			id: "latest-updates".into(),
			name: "".into(),
			..Default::default()
		};

		Ok(HomeLayout {
			components: vec![
				HomeComponent {
					title: Some("Best Rated Novels".to_string()),
					subtitle: Some("The most popular stories.".to_string()),
					value: buny::HomeComponentValue::Details {
						entries: self
							.get_novel_list(listing.clone(), 1)
							.unwrap_or_default()
							.entries,
						auto_scroll_interval: Some(10.0),
						listing: Some(listing),
					},
				},
				HomeComponent {
					title: Some("Trending Novels".to_string()),
					subtitle: Some(
						"Stories that you might fancy, but may be buried under the other gems."
							.to_string(),
					),
					value: buny::HomeComponentValue::Details {
						entries: self
							.get_novel_list(listing2.clone(), 1)
							.unwrap_or_default()
							.entries,
						auto_scroll_interval: Some(10.0),
						listing: Some(listing2),
					},
				},
				HomeComponent {
					title: Some("Rising Stars".to_string()),
					subtitle: Some(
						"Stories that you might fancy, but may be buried under the other gems."
							.to_string(),
					),
					value: buny::HomeComponentValue::Stack {
						entries: self
							.get_novel_list(listing3.clone(), 1)
							.unwrap_or_default()
							.entries,
						auto_scroll_interval: Some(10.0),
						listing: Some(listing3),
					},
				},
				HomeComponent {
					title: Some("Newest Novels".to_string()),
					subtitle: Some("Newest Stories".to_string()),
					value: buny::HomeComponentValue::Scroller {
						entries: self
							.get_novel_list(listing4.clone(), 1)
							.unwrap_or_default()
							.entries,
						auto_scroll_interval: Some(10.0),
						listing: Some(listing4),
						size: 400,
					},
				},
				HomeComponent {
					title: Some("Latest Updates".to_string()),
					subtitle: Some("The most recently updated stories.".to_string()),
					value: buny::HomeComponentValue::Scroller {
						entries: self
							.get_novel_list(listing5.clone(), 1)
							.unwrap_or_default()
							.entries,
						auto_scroll_interval: Some(10.0),
						listing: Some(listing5),
						size: 400,
					},
				},
			],
		})
	}
}

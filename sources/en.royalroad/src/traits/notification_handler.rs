use buny::{NotificationHandler, alloc::String, prelude::*};

use crate::RoyalRoad;

impl NotificationHandler for RoyalRoad {
	fn handle_notification(&self, key: String) {
		println!("Notification: {key}");
	}
}

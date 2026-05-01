use crate::{Params, helpers};
use buny::{
	Chapter, ContentBlock, ContentRating, FilterValue, Home, HomeComponent, HomeLayout,
	HomePartialResult, Listing, Novel, NovelPageResult, NovelStatus, Result, Source,
	alloc::{String, Vec, string::ToString, vec},
	helpers::{string::StripPrefixOrSelf, uri::QueryParameters},
	imports::{
		html::{Document, Html},
		net::Request,
		std::{current_date, parse_date, print, send_partial_result},
	},
	prelude::*,
};

pub trait Impl {
	fn new() -> Self;

	fn params(&self) -> Params;

	fn get_search_novel_list(
		&self,
		params: &Params,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<NovelPageResult> {
		let mut qs = QueryParameters::new();
		qs.push("page", Some(&page.to_string()));
		qs.push("q", query.as_deref());
		qs.push("status", Some("all"));

		for filter in filters {
			match filter {
				FilterValue::Sort { id, index, .. } => {
					let value = match index {
						0 => "views",
						1 => "updated_at",
						2 => "created_at",
						3 => "name",
						4 => "rating",
						_ => "views",
					};
					qs.push(&id, Some(value));
				}
				FilterValue::Select { id, value } => {
					qs.set(&id, Some(&value));
				}
				FilterValue::MultiSelect { id, included, .. } => {
					for item in included {
						qs.push(&id, Some(&item));
					}
				}
				_ => {}
			}
		}

		let url = format!("{}/search?{qs}", params.base_url);
		let html = Request::get(url)?.html()?;

		Ok(NovelPageResult {
			entries: html
				.select(".book-detailed-item")
				.map(|els| {
					els.filter_map(|el| {
						let link = el.select_first("a")?;
						let key: String = link
							.attr("href")?
							.strip_prefix_or_self(&params.base_url)
							.strip_prefix_or_self(format!("/{}/", &params.novel_path).as_str()) // stips /novel/ if it exists
							.into();
						Some(Novel {
							key: key,
							title: link.attr("title")?,
							cover: el.select_first("img")?.attr("abs:data-src"),
							..Default::default()
						})
					})
					.collect()
				})
				.unwrap_or_default(),
			has_next_page: html
				.select_first(".paginator > a.active + a:not([rel=next])")
				.is_some(),
		})
	}

	fn get_novel_update(
		&self,
		params: &Params,
		mut novel: Novel,
		needs_details: bool,
		needs_chapters: bool,
		_page: i32,
	) -> Result<Novel> {
		let novel_url = format!("{}/{}", params.base_url, novel.key);
		let html = Request::get(&novel_url)?.html()?;

		if needs_details {
			novel.title = html
				.select_first("h1[class*='mangaTitle']")
				.and_then(|h1| h1.text())
				.unwrap_or(novel.title);
			novel.cover = html
				.select_first("img[class*='coverImage']")
				.and_then(|img| img.attr("abs:data-src"))
				.or(novel.cover);
			novel.authors = html
				.select("div[class*='mangaAuthors'] span")
				.map(|els| {
				els.filter_map(|el| el.text())
					.map(|s| s.trim().trim_end_matches(',').trim().into())
					.collect()
			});
			novel.description = html
				.select("div[class*='description__']")
				.and_then(|mut els| els.next())
				.and_then(|el| el.text());

			novel.tags = html.select("a[href*='/genres/']").map(|els| {
				els.filter_map(|el| el.text())
					.map(|s| s.trim().trim_end_matches(',').into())
					.collect()
			});
			novel.status = html
				.select("div[class*='statItem']")
				.and_then(|els| {
					els.filter_map(|el| el.text())
						.map(|t| t.trim().to_lowercase())
						.find_map(|t| {
							Some(match t.as_str() {
								"ongoing" => NovelStatus::Ongoing,
								"completed" => NovelStatus::Completed,
								"on-hold" => NovelStatus::Hiatus,
								"canceled" => NovelStatus::Cancelled,
								_ => return None,
							})
						})
				})
				.unwrap_or_default();
			let tags = novel.tags.as_deref().unwrap_or(&[]);
			novel.content_rating = if tags
				.iter()
				.any(|e| matches!(e.as_str(), "Adult" | "Hentai" | "Mature" | "Smut"))
			{
				ContentRating::NSFW
			} else if tags.iter().any(|e| e == "Ecchi") {
				ContentRating::Suggestive
			} else if params.default_rating != ContentRating::Unknown {
				params.default_rating
			} else {
				ContentRating::Safe
			};

			send_partial_result(&novel);
		}

		if needs_chapters {
			fn parse_chapter_elements(html: &Document, params: &Params) -> Vec<Chapter> {
				html.select("div[class*='chapterList'] a")
					.map(|els| {
						els.filter_map(|el| {
							let title = el.select_first("h4")?.text()?;
							let link = el.attr("href")?;

							let chapter_number = helpers::find_first_f32(&title);

							let date_uploaded = el
								.select("span")
								.and_then(|els| {
									els.filter_map(|el| el.text()).find(|t| t.contains("ago"))
								})
								.map(|s| {
									parse_date(s, &params.date_format).unwrap_or(current_date())
								});

							Some(Chapter {
								key: link.strip_prefix_or_self(&params.base_url).into(),
								title: Some(title),
								chapter_number,
								date_uploaded,
								url: Some(format!("{}{}", params.base_url, link)),
								..Default::default()
							})
						})
						.collect()
					})
					.unwrap_or_default()
			}
			novel.chapters = None;
			send_partial_result(&novel);

			let chapters = (|| -> Result<Vec<Chapter>> {
				let data = html
					.select_first("script#__NEXT_DATA__")
					.and_then(|el| el.data())
					.ok_or(error!("Cannot find __NEXT_DATA__ script"))?;

				let id = data
					.split_once("\"mangaHsid\":\"")
					.ok_or(error!("mangaHsid not found"))?
					.1
					.split_once('"')
					.ok_or(error!("mangaHsid end not found"))?
					.0;

				let api_base = params
					.base_url
					.replace("novelbuddy.com", "api.novelbuddy.com");

				let url = format!("{}/titles/{}/chapters", api_base, id);

				let text = Request::get(&url)?.string()?;

				let json: serde_json::Value =
					serde_json::from_str(&text).map_err(|_| error!("Invalid JSON"))?;

				let mut chapters: Vec<Chapter> = json["data"]["chapters"]
					.as_array()
					.ok_or(error!("Invalid chapters array"))?
					.iter()
					.map(|ch| {
						let name = ch["name"].as_str().unwrap_or("").to_string();
						let link = ch["url"].as_str().unwrap_or("");

						Chapter {
							key: link.strip_prefix_or_self(&params.base_url).into(),
							title: Some(name.clone()),
							chapter_number: helpers::find_first_f32(&name),
							date_uploaded: ch["updated_at"]
								.as_str()
								.and_then(|d| parse_date(d, &params.date_format)),
							url: Some(format!("{}{}", params.base_url, link)),
							..Default::default()
						}
					})
					.collect();

				chapters.reverse();
				Ok(chapters)
			})()
			.unwrap_or_else(|_| parse_chapter_elements(&html, params));

			novel.chapters = Some(chapters);
		}

		Ok(novel)
	}

	fn get_chapter_content_list(
		&self,
		_novel: Novel,
		chapter: Chapter,
		params: &Params,
	) -> Result<Vec<ContentBlock>> {
		let url = format!("{}{}", params.base_url, chapter.key);
		let html = Request::get(&url)?.html()?;

		let mut content_list: Vec<ContentBlock> = html
			.select(".novel-tts-content p")
			.map(|els| {
				els.filter_map(|content_node| {
					let content = content_node.text()?.to_string();

					if content.starts_with('[') && content.ends_with(']') {
						let mut quote = content.chars();
						quote.next();
						quote.next_back();
						quote.as_str().to_string();

						return Some(ContentBlock::block_quote(quote.as_str().to_string()));
					} else if content == "***" {
						return Some(ContentBlock::Divider);
					}
					Some(ContentBlock::paragraph(content, None))
				})
				.collect()
			})
			.unwrap_or_default();

		let review_link = format!("LINK: [click here for chapter reviews.]({})", url);
		content_list.push(ContentBlock::Divider);
		content_list.push(ContentBlock::paragraph(review_link, None));
		Ok(content_list)
	}

	fn get_novel_list(&self, params: &Params, listing: Listing, _: i32) -> Result<NovelPageResult> {
		let url = format!("{}/{}", params.base_url, listing.id);
		let html = Request::get(url)?.html()?;

		Ok(NovelPageResult {
			entries: html
				.select(".flex.flex-col.h-full")
				.map(|els| {
					els.filter_map(|el| {
						let link = el.select_first("a.link-hover")?;
						let href = link.attr("href")?;

						let key: String = href
							.strip_prefix_or_self(&params.base_url)
							.strip_prefix_or_self(format!("/{}/", &params.novel_path).as_str())
							.into();

						let cover = el.select_first("img")?.attr("src")?.to_string();

						let title = el.select(".link-hover")?.next()?.text()?;

						Some(Novel {
							key: key,
							title: title,
							cover: Some(cover),
							..Default::default()
						})
					})
					.collect()
				})
				.unwrap_or_default(),
			has_next_page: false,
		})
	}
}

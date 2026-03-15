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
		page: i32,
	) -> Result<Novel> {
		let novel_url = format!("{}/{}/{}", params.base_url, params.novel_path, novel.key);
		let html = Request::get(&novel_url)?.html()?;

		if needs_details {
			novel.title = html
				.select_first(".detail h1")
				.and_then(|h1| h1.text())
				.unwrap_or(novel.title);
			novel.cover = html
				.select_first("#cover img")
				.and_then(|img| img.attr("abs:data-src"))
				.or(novel.cover);
			novel.authors = html
				.select(".detail .meta > p > strong:contains(Authors) ~ a")
				.map(|els| {
					els.filter_map(|el| el.text())
						.map(|s| s.trim().trim_end_matches(',').trim().into())
						.collect()
				});
			novel.description = html
				.select_first(".summary .content, .summary .content ~ p")
				.and_then(|div| div.text());
			novel.url = Some(novel_url.clone());
			novel.tags = html
				.select(".detail .meta > p > strong:contains(Genres) ~ a")
				.map(|els| {
					els.filter_map(|el| el.text())
						.map(|s| s.trim().trim_end_matches(',').into())
						.collect()
				});
			novel.status = html
				.select_first(".detail .meta > p > strong:contains(Status) ~ a")
				.and_then(|el| el.text())
				.map(|text| match text.to_lowercase().as_str() {
					"ongoing" => NovelStatus::Ongoing,
					"completed" => NovelStatus::Completed,
					"on-hold" => NovelStatus::Hiatus,
					"canceled" => NovelStatus::Cancelled,
					_ => NovelStatus::Unknown,
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
				html.select("#chapter-list > li")
					.map(|els| {
						els.filter_map(|el| {
							let a = el.select_first("a")?;
							let link = a.attr("abs:href")?;
							let title = el.select_first(".chapter-title")?.text()?;
							let chapter_number = helpers::find_first_f32(&title);
							Some(Chapter {
								key: link.strip_prefix_or_self(&params.base_url).into(),
								title: if title.as_str()
									!= format!("Chapter {}", chapter_number.unwrap_or(0.0))
								{
									Some(title)
								} else {
									None
								},
								chapter_number,
								date_uploaded: el
									.select_first(".chapter-update")
									.and_then(|el| el.text())
									.map(|s| {
										parse_date(s, &params.date_format).unwrap_or(current_date())
									}),
								url: Some(link),
								..Default::default()
							})
						})
						.collect()
					})
					.unwrap_or_default()
			}

			let fetch_api = html
				.select_first("div#show-more-chapters > span")
				.is_some_and(|el| el.attr("onclick").is_some_and(|s| s == "getChapters()"));

			let chapters = if fetch_api {
				let data = html
					.select_first("body > div.layout > script")
					.and_then(|el| el.data())
					.ok_or(error!("Cannot find script"))?;

				let url = format!(
					"{}/api/manga/{}/chapters/?source=detail",
					params.base_url,
					if params.use_slug_search {
						data.split_once("var bookSlug = \"")
							.ok_or(error!("String not found: `var bookSlug = \"`"))?
							.1
							.split_once("\";")
							.ok_or_else(|| error!("String not found: `\";`"))?
							.0
					} else {
						data.split_once("var bookId = ")
							.ok_or(error!("String not found: `var bookId = `"))?
							.1
							.split_once(";")
							.ok_or_else(|| error!("String not found: `;`"))?
							.0
					}
				);
				let html = Request::get(&url)?.html()?;
				parse_chapter_elements(&html, params)
			} else {
				parse_chapter_elements(&html, params)
			};

			novel.chapters = Some(chapters);
		}

		Ok(novel)
	}

	fn get_chapter_content_list(
		&self,
		novel: Novel,
		chapter: Chapter,
		params: &Params,
	) -> Result<Vec<ContentBlock>> {
		let url = format!("{}{}", params.base_url, chapter.key);
		let html = Request::get(&url)?.html()?;

		let mut content_list: Vec<ContentBlock> = html
			.select(".content-inner p")
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
				.select(".book-detailed-item")
				.map(|els| {
					els.filter_map(|el| {
						let link = el.select_first("a")?;
						let key: String = link
							.attr("href")?
							.strip_prefix_or_self(&params.base_url)
							.strip_prefix_or_self(format!("/{}/", &params.novel_path).as_str()) // stips /novel/ if it exists
							.into();
						let title = link.attr("title").unwrap_or_default().to_string();
						let cover = el.select_first("img")?.attr("abs:data-src")?.to_string();
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

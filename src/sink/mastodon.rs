use crate::{
    error::{Error, FeedError},
    feed::item::{FeedItem, TryFromItem},
    Result,
};

use super::Sink;

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use reqwest::{Client, IntoUrl, Url};
use serde::Serialize;

#[derive(Debug)]
pub struct Mastodon {
    url: Url,
    access_token: String,
    client: Client,
}

impl Mastodon {
    pub fn new<T: IntoUrl>(url: T, access_token: String, client: Client) -> Result<Self> {
        Ok(Self {
            url: url.into_url()?,
            access_token: access_token,
            client,
        })
    }
}

#[async_trait]
impl Sink for Mastodon {
    async fn push<'a, T>(&self, items: &'a [T]) -> Result<()>
    where
        T: FeedItem<'a>,
    {
        for item in items {
            // Might be a more idiomatic way of performing this transformation from raw data to a post string
            let data = MastodonStatusData::try_from_item(item)?;
            let post_data = MastodonStatusPost::from(&data);

            self.client
                .post(self.url.as_ref())
                .header("Authorization", "Bearer ".to_owned() + &self.access_token)
                .json(&post_data)
                .send()
                .await?
                .error_for_status()?;
        }

        Ok(())
    }

    async fn shutdown(self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MastodonStatusData {
    title: String,
    url: String,
    comments: String
}

#[derive(Debug, Serialize)]
pub struct MastodonStatusPost {
    status: String
}

impl<'a> From<&'a MastodonStatusData> for MastodonStatusPost {
    fn from(other: &'a MastodonStatusData) -> Self {
        // Transform the raw data into a single string for posting
        Self {
            status: format!("{}\n{}\n{}", other.title, other.url, other.comments)
        }
    }
}

impl<'a, T> TryFromItem<'a, T> for MastodonStatusData
where
    T: FeedItem<'a>,
{
    type Error = Error;

    fn try_from_item(value: &'a T) -> std::result::Result<Self, Self::Error> {
        // Not sure this is the best way to combine strings into a single entry like this
        let title = value.title_as_text().ok_or_else(|| FeedError::Item("title is missing".to_string()))?;
        let url = value.link().unwrap_or_default().to_string();
        let comments = value.comments().unwrap_or_default();
        let item: MastodonStatusData = Self {
            title: title,
            url: url,
            comments: comments.to_string(),
        };

        Ok(item)
    }
}

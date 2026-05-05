use std::{fmt::Display, path::Path, sync::Arc};

use bytes::{BufMut, Bytes, BytesMut};
use futures_util::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::Deserialize;

pub struct SteamGridClient {
    client: reqwest::Client,
    download_client: reqwest::Client,
    base_url: String,
}

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

impl SteamGridClient {
    pub fn new(api_key: &str) -> anyhow::Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();

        let auth_value = format!("Bearer {}", api_key);

        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&auth_value)?,
        );

        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .user_agent(APP_USER_AGENT)
            .build()?;

        // Downloading assets doesn't need auth headers
        let download_client = reqwest::ClientBuilder::new()
            .user_agent(APP_USER_AGENT)
            .build()?;

        Ok(Self {
            client,
            download_client,
            base_url: "https://www.steamgriddb.com/api/v2".to_owned(),
        })
    }

    pub async fn search_by_name(&self, name: &str) -> anyhow::Result<Vec<GameSearchObject>> {
        let url = format!("{}/search/autocomplete/{}", self.base_url, name);

        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json::<ApiResponse<Vec<GameSearchObject>>>()
            .await?;

        Ok(response.data)
    }

    pub async fn find_asset(
        &self,
        game_id: u64,
        asset_type: AssetType,
    ) -> anyhow::Result<Vec<GridAsset>> {
        let url = format!("{}{}{}", self.base_url, asset_type.get_url(), game_id);

        let response = self
            .client
            .get(&url)
            .query(asset_type.get_query_params())
            .send()
            .await?
            .error_for_status()?
            .json::<ApiResponse<Vec<GridAsset>>>()
            .await?;

        Ok(response.data)
    }

    pub async fn download_asset(
        &self,
        asset: &GridAsset,
        asset_type: AssetType,
        mp: Arc<MultiProgress>,
    ) -> anyhow::Result<Image> {
        let response = self
            .download_client
            .get(&asset.url)
            .send()
            .await?
            .error_for_status()?;

        let total = response.content_length().unwrap_or(0);
        let pb = mp.add(
            ProgressBar::new(total)
                .with_message(format!("Downloading {asset_type}..."))
                .with_style(ProgressStyle::with_template(
                    "{msg:12} [{bar:40.cyan/blue}] {bytes:>7}/{total_bytes:7} {eta}",
                )?),
        );

        let format = match asset.mime.as_str() {
            "image/png" => ImageType::Png,
            "image/vnd.microsoft.icon" => ImageType::Ico,
            e => anyhow::bail!("Unknown mime type: {e}"),
        };

        let mut stream = response.bytes_stream();
        let mut bytes = BytesMut::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            pb.inc(chunk.len() as u64);
            bytes.put(chunk);
        }

        pb.finish();

        Ok(Image {
            bytes: bytes.freeze(),
            format,
        })
    }
}

#[derive(Deserialize, Debug)]
struct ApiResponse<T> {
    #[expect(unused)]
    pub success: bool,
    pub data: T,
}

#[derive(Deserialize, Debug)]
pub struct GameSearchObject {
    pub id: u64,
    pub name: String,
    pub verified: bool,
    pub types: Vec<String>,
    pub release_date: Option<u64>,
}

#[derive(Deserialize, Debug)]
pub struct GridAsset {
    pub id: u64,
    pub score: i32,
    pub style: String,
    pub width: u32,
    pub height: u32,
    pub nsfw: bool,
    pub humor: bool,
    pub notes: Option<String>,
    pub mime: String,
    pub language: String,
    pub url: String,
    pub thumb: String,
    pub lock: bool,
    pub epilepsy: bool,
    pub upvotes: u32,
    pub downvotes: u32,
    pub author: Author,
}

#[derive(Deserialize, Debug)]
pub struct Author {
    pub name: String,
    pub steam64: String,
    pub avatar: String,
}

pub enum AssetType {
    Grid,
    Hero,
    Logo,
    Icon,
}

impl Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetType::Grid => write!(f, "Grid"),
            AssetType::Hero => write!(f, "Hero"),
            AssetType::Logo => write!(f, "Logo"),
            AssetType::Icon => write!(f, "Icon"),
        }
    }
}

pub struct Image {
    bytes: Bytes,
    format: ImageType,
}

pub enum ImageType {
    Png,
    Jpg,
    Webp,
    Ico,
}

impl Image {
    pub fn save(self, app_id: u32, dir: &Path, asset_type: AssetType) -> std::io::Result<String> {
        let ext = match self.format {
            ImageType::Png => "png",
            ImageType::Jpg => "jpg",
            ImageType::Webp => "png", // Webp saves as png
            ImageType::Ico => "ico",
        };

        let filename = match asset_type {
            AssetType::Grid => format!("{}p.{}", app_id, ext),
            AssetType::Hero => format!("{}_hero.{}", app_id, ext),
            AssetType::Logo => format!("{}_logo.{}", app_id, ext),
            AssetType::Icon => format!("{}_icon.{}", app_id, ext),
        };

        let path = dir.join(&filename);

        std::fs::write(&path, self.bytes)?;

        Ok(path.display().to_string())
    }
}

impl AssetType {
    const fn get_url(&self) -> &'static str {
        match self {
            AssetType::Grid => "/grids/game/",
            AssetType::Hero => "/heroes/game/",
            AssetType::Logo => "/logos/game/",
            AssetType::Icon => "/icons/game/",
        }
    }

    const fn get_query_params(&self) -> &[(&'static str, &'static str)] {
        match self {
            AssetType::Grid => &[
                ("dimensions", "600x900"),
                ("types", "static"),
                ("nsfw", "any"),
            ],
            AssetType::Hero => &[
                ("dimensions", "3840x1240"),
                ("types", "static"),
                ("nsfw", "any"),
            ],
            AssetType::Logo => &[("types", "static"), ("nsfw", "any")],
            AssetType::Icon => &[("styles", "official"), ("types", "static"), ("nsfw", "any")],
        }
    }
}

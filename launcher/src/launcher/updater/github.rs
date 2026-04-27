use crate::launcher::http;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::time::Duration;

pub struct AssetInformation {
    pub name: String,
    pub url: String,
}

pub struct ReleaseInformation {
    pub tag_name: String,
    pub assets: Vec<AssetInformation>,
}

struct Response {
    data: String,
    status: u32,
}

fn api_get_request(url: &str) -> anyhow::Result<Response> {
    let mut builder = http::RequestBuilder::new(url);
    builder.add_header("X-GitHub-Api-Version: 2026-03-10")?;
    builder.timeout(Duration::from_secs(3));
    let req = builder.build()?;

    req.perform()?;
    let handler = req.get_ref();

    Ok(Response {
        data: String::from_utf8(handler.get_data())?,
        status: req.response_code().unwrap_or(0),
    })
}

pub fn fetch_release_information(repository_path: &str) -> Result<ReleaseInformation, FetchError> {
    let url = format!("https://api.github.com/repos/{repository_path}/releases/latest");
    let response = api_get_request(url.as_str()).map_err(|_| FetchError::RequestError)?;
    if response.status != 200 {
        if response.status == 403 {
            return Err(FetchError::Forbidden);
        }
        return Err(FetchError::RequestError);
    }

    let response_json: json::Value =
        json::from_str(response.data.as_str()).map_err(|_| FetchError::InvalidResponse)?;

    let tag_name = response_json
        .pointer("/tag_name")
        .ok_or(FetchError::TagName)?
        .as_str()
        .ok_or(FetchError::TagName)?;

    let release_assets_json = response_json
        .pointer("/assets")
        .ok_or(FetchError::ReleaseAssets)?
        .as_array()
        .ok_or(FetchError::ReleaseAssets)?;

    let mut assets = Vec::<AssetInformation>::new();
    for asset_json in release_assets_json {
        let asset_name = asset_json
            .pointer("/name")
            .ok_or(FetchError::ReleaseAssets)?
            .as_str()
            .ok_or(FetchError::ReleaseAssets)?;

        let asset_url = asset_json
            .pointer("/browser_download_url")
            .ok_or(FetchError::ReleaseAssets)?
            .as_str()
            .ok_or(FetchError::ReleaseAssets)?;

        assets.push(AssetInformation {
            name: asset_name.to_string(),
            url: asset_url.to_string(),
        });
    }

    Ok(ReleaseInformation {
        tag_name: tag_name.to_string(),
        assets,
    })
}

pub fn find_asset<'a>(
    release_info: &'a ReleaseInformation,
    pattern: &str,
) -> Option<&'a AssetInformation> {
    let regex = Regex::new(pattern).expect("Failed to compile regex");
    release_info
        .assets
        .iter()
        .find(|asset| regex.is_match(asset.name.as_str()))
}

pub fn fetch_hashes(release_info: &ReleaseInformation) -> anyhow::Result<String> {
    let hashes_asset = match find_asset(release_info, "^hashes.txt$") {
        None => return Err(HashesError::AssetNotFound.into()),
        Some(hashes_asset) => hashes_asset,
    };

    let response =
        api_get_request(hashes_asset.url.as_str()).map_err(|_| HashesError::FetchError)?;
    if response.status != 200 {
        if response.status == 403 {
            return Err(FetchError::Forbidden.into());
        }
        return Err(FetchError::RequestError.into());
    }

    Ok(response.data)
}

pub fn parse_hashes(s: &str) -> HashMap<&str, &str> {
    let mut map = HashMap::new();

    for line in s.lines() {
        if let Some((hash, filename)) = line
            .split_once(char::is_whitespace)
            .map(|(h, f)| (h, f.trim_start()))
        {
            map.insert(filename, hash);
        }
    }

    map
}

pub enum FetchError {
    RequestError,
    Forbidden,
    InvalidResponse,
    TagName,
    ReleaseAssets,
}

impl FetchError {
    fn message(&self) -> &str {
        match self {
            Self::RequestError => {
                "Couldn't check for updates at the moment. Please try again later."
            }
            Self::Forbidden => {
                "Communication with GitHub API not allowed at the moment. Please try again later."
            }
            Self::InvalidResponse => "Invalid JSON response from GitHub API",
            Self::TagName => "Couldn't get tag name",
            Self::ReleaseAssets => "Couldn't get release assets",
        }
    }
}

impl Display for FetchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.message())
    }
}

impl Debug for FetchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.message())
    }
}

impl Error for FetchError {}

pub enum HashesError {
    AssetNotFound,
    FetchError,
}

impl HashesError {
    fn message(&self) -> &str {
        match self {
            Self::AssetNotFound => "Couldn't find hashes.txt asset",
            Self::FetchError => "Failed to fetch hashes.txt",
        }
    }
}

impl Display for HashesError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.message())
    }
}

impl Debug for HashesError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.message())
    }
}

impl Error for HashesError {}

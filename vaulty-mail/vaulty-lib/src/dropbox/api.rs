use std::error;
use std::fmt;

use reqwest::blocking;
use reqwest::StatusCode;

use serde::{Serialize, Deserialize};

pub const DROPBOX_ARG_HEADER: &str = "Dropbox-API-Arg";
pub const DROPBOX_BASE_API: &str = "https://api.dropboxapi.com/2/";
pub const DROPBOX_BASE_CONTENT: &str = "https://content.dropboxapi.com/2/";

#[derive(Debug)]
pub enum Error {
    BadInput,
    TokenExpired,
    Endpoint,
    RateLimited,
    Internal(u16),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::BadInput => f.write_str("BadInput"),
            Error::TokenExpired => f.write_str("TokenExpired"),
            Error::Endpoint => f.write_str("Endpoint"),
            Error::RateLimited => f.write_str("RateLimited"),
            Error::Internal(_) => f.write_str("Internal Error"),
        }
    }
}

impl error::Error for Error {}

impl Error {
    /// Map possible Dropbox API errors
    pub fn map_status(resp: blocking::Response)
        -> Result<blocking::Response, Self> {
        let status = resp.status();
        match status {
            StatusCode::OK => Ok(resp),
            StatusCode::BAD_REQUEST => Err(Error::BadInput),
            StatusCode::FORBIDDEN => Err(Error::TokenExpired),
            StatusCode::CONFLICT => Err(Error::Endpoint),
            StatusCode::TOO_MANY_REQUESTS => Err(Error::RateLimited),
            _ => Err(Error::Internal(status.as_u16())),
        }
    }
}

pub enum Endpoint {
    ListFolder,
    FileUpload,
    CreateFolder,
}

#[derive(Deserialize, Debug)]
pub struct ListFolderResult {
    entries: Vec<ListFolderEntry>,
    has_more: bool,
}

#[derive(Deserialize, Debug)]
pub struct ListFolderEntry {
    #[serde(rename = ".tag")]
    tag: String,
    name: String,
    path_lower: String,
    path_display: String,
    id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileUploadResult {
    name: String,
    id: String,
    size: usize,
    server_modified: String,
    path_lower: String,
    path_display: String,
    content_hash: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateFolderResult {
    name: String,
}

#[inline]
pub fn build_endpoint_url(endpoint: Endpoint) -> String {
    match endpoint {
        Endpoint::ListFolder => format!("{}{}", DROPBOX_BASE_API, "files/list_folder"),
        Endpoint::CreateFolder => format!("{}{}", DROPBOX_BASE_API, "files/create_folder_v2"),
        Endpoint::FileUpload => format!("{}{}", DROPBOX_BASE_CONTENT, "files/upload"),
    }
}

use std::{env, path::PathBuf};

use log::debug;
use minio::s3::{
    args::{BucketExistsArgs, MakeBucketArgs, UploadObjectArgs, SetBucketPolicyArgs},
    client::Client as MinioClient,
    creds::StaticProvider,
    http::BaseUrl, sse::SseCustomerKey,
};
use reqwest::{multipart, Client as ReqwestClient};
use serde::{Deserialize, Serialize};
use serde_json::{map, Value};

#[derive(Serialize, Deserialize)]
pub struct File {
    pub delete_token: String,
    pub file: String,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub files: Vec<File>,
    pub msg: String,
}

pub struct Minio {
    minio_client: MinioClient,
    bucket: String,
    base_url: String,
}

impl Minio {
    pub async fn new() -> Self {
        let base_url_str = env::var("S3_URL").expect("Expected a token in the environment");
        let base_url = base_url_str
            .parse::<BaseUrl>()
            .expect("Failed to parse base url");

        let static_provider = StaticProvider::new(
            env::var("S3_KEY")
                .expect("Expected a token in the environment")
                .as_str(),
            env::var("S3_SECRET")
                .expect("Expected a token in the environment")
                .as_str(),
            None,
        );

        let minio_client = MinioClient::new(base_url.clone(), Some(Box::new(static_provider)), None, None)
            .expect("Failed to create minio client");

        let bucket = "images";

        let exists = minio_client
            .bucket_exists(&BucketExistsArgs::new(&bucket).unwrap())
            .await
            .unwrap();

        if !exists {
            minio_client
                .make_bucket(&MakeBucketArgs::new(&bucket).unwrap())
                .await
                .unwrap();
        }

        Self {
            minio_client,
            bucket: bucket.to_string(),
            base_url: base_url_str,
        }
    }

    pub async fn upload_image(&self, image_path: &str) -> Result<String, reqwest::Error> {
        let filename = rand::random::<u64>().to_string();
        let filename = format!("{}.png", filename);
        let res = self.minio_client
            .upload_object(
                &mut UploadObjectArgs::<SseCustomerKey>::new(
                    &self.bucket,
                    &filename,
                    image_path,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        debug!("uploaded image: {:?}", res.location);
        Ok(format!("{}/{}/{}", self.base_url, self.bucket, filename))
    }
}

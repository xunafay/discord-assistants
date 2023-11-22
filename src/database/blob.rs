use std::env;

use log::debug;
use minio::s3::{
    args::{BucketExistsArgs, MakeBucketArgs, SetBucketPolicyArgs, UploadObjectArgs},
    client::Client as MinioClient,
    creds::StaticProvider,
    http::BaseUrl,
    sse::SseCustomerKey,
};
use serde::{Deserialize, Serialize};

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
    base_url: String,
}

impl Minio {
    pub fn new() -> Self {
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

        let minio_client = MinioClient::new(
            base_url.clone(),
            Some(Box::new(static_provider)),
            None,
            None,
        )
        .expect("Failed to create minio client");


        Self {
            minio_client,
            base_url: base_url_str,
        }
    }

    pub async fn upload_image(&self, image_path: &str) -> Result<String, reqwest::Error> {
        let bucket = "images";
        self.ensure_bucket_exists(bucket).await;
        let filename = rand::random::<u64>().to_string();
        let filename = format!("{}.png", filename);
        let res = self
            .minio_client
            .upload_object(
                &mut UploadObjectArgs::<SseCustomerKey>::new(bucket, &filename, image_path)
                    .unwrap(),
            )
            .await
            .unwrap();
        debug!("uploaded image: {:?}", res.location);
        Ok(format!("{}/{}/{}", self.base_url, bucket, filename))
    }

    pub async fn upload_mp3(&self, mp3_path: &str) -> Result<String, reqwest::Error> {
        let bucket = "audio";
        self.ensure_bucket_exists(bucket).await;

        let filename = rand::random::<u64>().to_string();
        let filename = format!("{}.mp3", filename);
        let res = self
            .minio_client
            .upload_object(
                &mut UploadObjectArgs::<SseCustomerKey>::new(bucket, &filename, mp3_path)
                    .unwrap(),
            )
            .await
            .unwrap();
        debug!("uploaded mp3: {:?}", res.location);
        Ok(format!("{}/{}/{}", self.base_url, bucket, filename))
    }

    pub async fn ensure_bucket_exists(&self, bucket: &str) {
        let exists = self
            .minio_client
            .bucket_exists(&BucketExistsArgs::new(bucket).unwrap())
            .await
            .unwrap();

        if !exists {
            self.minio_client
                .make_bucket(&MakeBucketArgs::new(bucket).unwrap())
                .await
                .unwrap();

            self.minio_client.set_bucket_policy(
                &SetBucketPolicyArgs::new(bucket, &download_policy())
                    .expect("Failed to create bucket policy"),
            ).await.expect("Failed to set bucket policy");
        }
    }
}

fn download_policy() -> String {
    serde_json::json!({
        "Version": "2012-10-17",
        "Statement": [
            {
                "Effect": "Allow",
                "Principal": {
                    "AWS": [
                        "*"
                    ]
                },
                "Action": [
                    "s3:GetBucketLocation",
                    "s3:ListBucket"
                ],
                "Resource": [
                    "arn:aws:s3:::audio"
                ]
            },
            {
                "Effect": "Allow",
                "Principal": {
                    "AWS": [
                        "*"
                    ]
                },
                "Action": [
                    "s3:GetObject"
                ],
                "Resource": [
                    "arn:aws:s3:::audio/*"
                ]
            }
        ]
    })
    .to_string()
}

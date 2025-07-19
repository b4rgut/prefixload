use crate::error::Result;
use aws_sdk_s3 as s3;
use aws_sdk_s3::config::Builder as S3ConfigBuilder;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_types::region::Region;

#[derive(Debug, Clone)]
pub struct S3Client {
    inner: s3::Client,
}

/// Client creation parameters.
///
/// * `region` and `endpoint' are optional:
/// * if `region` is not specified, it is taken from the 'AWS_REGION` / AWS config;
/// * if `endpoint` is not specified, the standard one for the selected region is used.
/// * 'force_path_style' is useful for MinIO, Ceph RGW, Wasabi and other
/// S3-compatible services that require path-style URLs.
#[derive(Debug, Clone)]
pub struct S3ClientOptions<'a> {
    pub access_key: &'a str,
    pub secret_key: &'a str,
    pub region: Option<&'a str>,
    pub endpoint: Option<&'a str>,
    pub force_path_style: bool,
}

impl<'a> Default for S3ClientOptions<'a> {
    fn default() -> Self {
        Self {
            access_key: "",
            secret_key: "",
            region: None,
            endpoint: None,
            force_path_style: false,
        }
    }
}

impl S3Client {
    /// Creates a new client capable of working with both AWS
    /// and any S3-compatible service.
    pub async fn new(opts: S3ClientOptions<'_>) -> Result<Self> {
        /* ---------- учёт учётных данных ---------- */
        let credentials = Credentials::new(
            opts.access_key,
            opts.secret_key,
            None,            // session-token
            None,            // expires-at
            "user-supplied", // provider-name
        );

        let cred_provider = s3::config::SharedCredentialsProvider::new(credentials);

        let mut cfg_loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .credentials_provider(cred_provider);

        let region = opts
            .region // Option<&str>
            .unwrap_or("us-east-1"); // default

        cfg_loader = cfg_loader.region(Region::new(region.to_owned()));

        let shared_cfg = cfg_loader.load().await;

        let mut s3_cfg = S3ConfigBuilder::from(&shared_cfg);

        if let Some(url) = opts.endpoint {
            s3_cfg = s3_cfg
                .endpoint_url(url)
                .force_path_style(opts.force_path_style);
        }

        let client = s3::Client::from_conf(s3_cfg.build());

        Ok(Self { inner: client })
    }

    /// Checks the availability of the bucket
    /// Result:
    /// - `Ok(true)`  – the bucket is available
    /// - `Ok(false)` – the key is valid, but there are no rights (401/403)
    /// - `Err(e)`    – other errors (network, DNS, incorrect region, etc.)
    pub async fn check_bucket_access(&self, bucket: &str) -> Result<bool> {
        match self.inner.head_bucket().bucket(bucket).send().await {
            Ok(_) => Ok(true),
            Err(e) if matches!(e.code(), Some("AccessDenied" | "Forbidden")) => Ok(false),
            Err(e) => Err(aws_sdk_s3::Error::from(e).into()),
        }
    }
}

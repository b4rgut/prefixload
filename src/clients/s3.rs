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
pub struct S3ClientOptions {
    pub access_key: String,
    pub secret_key: String,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub force_path_style: bool,
}

impl Default for S3ClientOptions {
    fn default() -> Self {
        Self {
            access_key: "".to_string(),
            secret_key: "".to_string(),
            region: None,
            endpoint: None,
            force_path_style: false,
        }
    }
}

impl S3ClientOptions {
    pub fn with_access_key<S: Into<String>>(mut self, access_key: S) -> Self {
        self.access_key = access_key.into();
        self
    }

    pub fn with_secret_key<S: Into<String>>(mut self, secret_key: S) -> Self {
        self.secret_key = secret_key.into();
        self
    }

    pub fn with_region<S: Into<String>>(mut self, region: S) -> Self {
        self.region = Some(region.into());
        self
    }

    pub fn with_endpoint<S: Into<String>>(mut self, endpoint: S) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn with_force_path_style(mut self, force_path_style: bool) -> Self {
        self.force_path_style = force_path_style;
        self
    }
}

impl S3Client {
    /// Creates a new client capable of working with both AWS
    /// and any S3-compatible service.
    pub async fn new(opts: S3ClientOptions) -> Result<Self> {
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

        let region = opts.region.unwrap_or_else(|| "us-east-1".to_string());

        cfg_loader = cfg_loader.region(Region::new(region));

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
            // 200 OK – the bucket exists and the credentials are valid
            Ok(_) => Ok(true),

            Err(sdk_err) => {
                // 403 Forbidden or 401 Unauthorized ⇢ the bucket is there, but there are no rights
                if matches!(sdk_err.code(), Some("AccessDenied") | Some("Forbidden")) {
                    return Ok(false);
                }

                // Everything else (404 NotFound, PermanentRedirect, network failures, etc.)
                // wrap it in our Error tree and throw it up.
                let aws_err: aws_sdk_s3::Error = sdk_err.into();
                Err(aws_err.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const AK: &str = "TEST_AK";
    const SK: &str = "TEST_SK";

    /// Helper: build client pointed to wiremock
    async fn client(server: &MockServer) -> S3Client {
        S3Client::new(S3ClientOptions {
            access_key: AK.to_string(),
            secret_key: SK.to_string(),
            region: None,                 // default us-east-1
            endpoint: Some(server.uri()), // plain-http mock
            force_path_style: true,
        })
        .await
        .expect("client init")
    }

    #[tokio::test]
    async fn bucket_exists_returns_true() {
        let server = MockServer::start().await;
        let bucket = "mybucket";

        Mock::given(method("HEAD"))
            .and(path_regex(r"^/mybucket(/)?$")) // ← tolerate optional '/'
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let ok = client(&server)
            .await
            .check_bucket_access(bucket)
            .await
            .unwrap(); // should be Ok(true)

        assert!(ok);
    }

    #[tokio::test]
    async fn not_found_propagates_error() {
        let server = MockServer::start().await;
        let bucket = "missing";

        Mock::given(method("HEAD"))
            .and(path_regex(r"^/missing(/)?$"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let err = client(&server)
            .await
            .check_bucket_access(bucket)
            .await
            .expect_err("should be Err");

        // basic sanity: make sure original S3 error code preserved
        assert!(format!("{err:?}").contains("NotFound"));
    }

    #[tokio::test]
    async fn default_region_is_us_east_1() {
        let server = MockServer::start().await;
        let cli = client(&server).await;

        let region = cli.inner.config().region().unwrap().as_ref();
        assert_eq!(region, "us-east-1");
    }
}

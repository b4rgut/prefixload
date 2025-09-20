use crate::error::{PrefixloadError, Result};
use aws_config::profile::ProfileFileCredentialsProvider;
use aws_credential_types::provider::ProvideCredentials;
use aws_sdk_s3 as s3;
use aws_sdk_s3::config::Builder as S3ConfigBuilder;
use aws_sdk_s3::config::Credentials;
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

/// Builder methods for `S3ClientOptions`.
impl S3ClientOptions {
    /// Loads AWS credentials (access key and secret key) from the standard
    /// profile files (`~/.aws/credentials` and `~/.aws/config`).
    ///
    /// This function uses `ProfileFileCredentialsProvider` to read credentials,
    /// which limits the search to only the configuration files and excludes other
    /// sources like environment variables or IAM roles.
    ///
    /// # Returns
    ///
    /// A `Result` with `S3ClientOptions` containing the access key and secret key,
    /// or a `PrefixloadError` on failure.
    pub async fn from_aws_config() -> Result<Self> {
        let provider = ProfileFileCredentialsProvider::builder().build();

        let credentials = provider.provide_credentials().await.map_err(|err| {
            PrefixloadError::Custom(format!(
                "Failed to load credentials from AWS profile: {}",
                err
            ))
        })?;

        Ok(Self {
            access_key: credentials.access_key_id().to_string(),
            secret_key: credentials.secret_access_key().to_string(),
            ..Self::default()
        })
    }

    /// Sets the access key.
    pub fn with_access_key<S: Into<String>>(mut self, access_key: S) -> Self {
        self.access_key = access_key.into();
        self
    }

    /// Sets the secret key.
    pub fn with_secret_key<S: Into<String>>(mut self, secret_key: S) -> Self {
        self.secret_key = secret_key.into();
        self
    }

    /// Sets the AWS region.
    pub fn with_region<S: Into<String>>(mut self, region: S) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Sets a custom S3 endpoint URL.
    /// Useful for S3-compatible services like MinIO or Ceph.
    pub fn with_endpoint<S: Into<String>>(mut self, endpoint: S) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Enables or disables force path-style addressing.
    /// Required for services that do not support virtual-hosted-style requests.
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
            s3_cfg = s3_cfg.endpoint_url(url);
        }

        // Always apply force_path_style from options, regardless of endpoint
        s3_cfg = s3_cfg.force_path_style(opts.force_path_style);

        let client = s3::Client::from_conf(s3_cfg.build());

        Ok(Self { inner: client })
    }

    /// Checks the availability of the bucket
    /// Result:
    /// - `Ok(true)`  – the bucket is available
    /// - `Ok(false)` – the key is valid, but there are no rights (401/403)
    /// - `Err(e)`    – other errors (network, DNS, incorrect region, etc.)
    pub async fn check_bucket_access(&self, bucket: &str) -> Result<bool> {
        let result = self.inner.head_bucket().bucket(bucket).send().await;
        match result {
            Ok(_) => Ok(true),
            Err(sdk_err) => {
                // For HeadBucket, 403/401 indicates that the bucket exists, but we don't have access.
                // The error code might not be populated correctly in all cases,
                // so we check the raw status code.
                if let Some(response) = sdk_err.raw_response() {
                    let status = response.status();
                    if status.as_u16() == 403 || status.as_u16() == 401 {
                        return Ok(false);
                    }
                }

                // If we couldn't get the raw response, or for any other error,
                // convert to our error type and propagate.
                let aws_err: aws_sdk_s3::Error = sdk_err.into();
                Err(aws_err.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::tempdir;
    use wiremock::matchers::{header, header_exists, method, path_regex};
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
            .and(header_exists("authorization"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let result = client(&server).await.check_bucket_access(bucket).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn access_denied_returns_false() {
        let server = MockServer::start().await;
        let bucket = "forbidden-bucket";

        let response = ResponseTemplate::new(403);

        Mock::given(method("HEAD"))
            .and(path_regex(r"^/forbidden-bucket(/)?$"))
            .respond_with(response)
            .mount(&server)
            .await;

        let ok = client(&server)
            .await
            .check_bucket_access(bucket)
            .await
            .expect("should be Ok(false)");

        assert!(!ok);
    }

    #[tokio::test]
    async fn unauthorized_returns_false() {
        let server = MockServer::start().await;
        let bucket = "unauthorized-bucket";

        let response = ResponseTemplate::new(401);

        Mock::given(method("HEAD"))
            .and(path_regex(r"^/unauthorized-bucket(/)?$"))
            .respond_with(response)
            .mount(&server)
            .await;

        let ok = client(&server)
            .await
            .check_bucket_access(bucket)
            .await
            .expect("should be Ok(false)");

        assert!(!ok);
    }

    #[tokio::test]
    async fn not_found_propagates_error() {
        let server = MockServer::start().await;
        let bucket = "missing";

        let response = ResponseTemplate::new(404);

        Mock::given(method("HEAD"))
            .and(path_regex(r"^/missing(/)?$"))
            .respond_with(response)
            .mount(&server)
            .await;

        let err = client(&server)
            .await
            .check_bucket_access(bucket)
            .await
            .expect_err("should be Err");

        assert!(format!("{err:?}").contains("NotFound"));
    }

    #[tokio::test]
    async fn default_region_is_us_east_1() {
        let server = MockServer::start().await;
        let cli = client(&server).await;

        let region = cli.inner.config().region().unwrap().as_ref();
        assert_eq!(region, "us-east-1");
    }

    #[tokio::test]
    async fn custom_region_is_set() {
        let server = MockServer::start().await;
        let region_name = "eu-west-1";

        let cli = S3Client::new(S3ClientOptions {
            access_key: AK.to_string(),
            secret_key: SK.to_string(),
            region: Some(region_name.to_string()),
            endpoint: Some(server.uri()),
            force_path_style: true,
        })
        .await
        .expect("client init");

        let region = cli.inner.config().region().unwrap().as_ref();
        assert_eq!(region, region_name);
    }

    #[tokio::test]
    async fn force_path_style_is_honored() {
        let server = MockServer::start().await;
        let bucket = "mybucket";

        // Mock for path-style request (e.g. http://localhost:1234/mybucket)
        Mock::given(method("HEAD"))
            .and(path_regex(r"^/mybucket(/)?$"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        // Mock for virtual-hosted-style request (e.g. http://mybucket.localhost:1234)
        let server_host = server.uri().replace("http://", "");
        let expected_host = format!("{}.{}", bucket, server_host);
        Mock::given(method("HEAD"))
            .and(header("Host", expected_host.as_str()))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        // 1. Test with force_path_style = true
        let cli_path_style = S3Client::new(S3ClientOptions {
            access_key: AK.to_string(),
            secret_key: SK.to_string(),
            region: Some("us-east-1".to_string()),
            endpoint: Some(server.uri()),
            force_path_style: true,
        })
        .await
        .expect("client init with path style");

        // This should hit the path-style mock and succeed
        assert!(cli_path_style.check_bucket_access(bucket).await.is_ok());

        // 2. Test with force_path_style = false
        let cli_virtual_hosted = S3Client::new(S3ClientOptions {
            access_key: AK.to_string(),
            secret_key: SK.to_string(),
            region: Some("us-east-1".to_string()),
            endpoint: Some(server.uri()),
            force_path_style: false,
        })
        .await
        .expect("client init with virtual-hosted style");

        // This should hit the virtual-hosted-style mock and succeed
        assert!(cli_virtual_hosted.check_bucket_access(bucket).await.is_ok());
    }

    #[test]
    fn options_builder_works() {
        let opts = S3ClientOptions::default()
            .with_access_key("ak")
            .with_secret_key("sk")
            .with_region("eu-central-1")
            .with_endpoint("http://localhost:9000")
            .with_force_path_style(true);

        assert_eq!(opts.access_key, "ak");
        assert_eq!(opts.secret_key, "sk");
        assert_eq!(opts.region, Some("eu-central-1".to_string()));
        assert_eq!(opts.endpoint, Some("http://localhost:9000".to_string()));
        assert!(opts.force_path_style);
    }

    #[test]
    fn options_default_is_empty() {
        let opts = S3ClientOptions::default();
        assert_eq!(opts.access_key, "");
        assert_eq!(opts.secret_key, "");
        assert_eq!(opts.region, None);
        assert_eq!(opts.endpoint, None);
        assert!(!opts.force_path_style);
    }

    #[tokio::test]
    #[serial]
    async fn from_aws_config_success() {
        let dir = tempdir().unwrap();
        let aws_dir = dir.path().join(".aws");
        fs::create_dir(&aws_dir).unwrap();
        let credentials_path = aws_dir.join("credentials");
        fs::write(
            &credentials_path,
            "[default]\naws_access_key_id = MY_ACCESS_KEY\naws_secret_access_key = MY_SECRET_KEY\n",
        )
        .unwrap();

        // Set HOME to our temporary directory. This is unsafe.
        unsafe {
            std::env::set_var("HOME", dir.path());
            // Also set env vars to ensure the file provider is preferred.
            std::env::set_var("AWS_ACCESS_KEY_ID", "env_key");
            std::env::set_var("AWS_SECRET_ACCESS_KEY", "env_secret");
        }

        let opts = S3ClientOptions::from_aws_config().await.unwrap();

        assert_eq!(opts.access_key, "MY_ACCESS_KEY");
        assert_eq!(opts.secret_key, "MY_SECRET_KEY");

        // Unset the env var to avoid interfering with other tests.
        unsafe {
            std::env::remove_var("HOME");
            std::env::remove_var("AWS_ACCESS_KEY_ID");
            std::env::remove_var("AWS_SECRET_ACCESS_KEY");
        }
    }

    #[tokio::test]
    #[serial]
    async fn from_aws_config_file_not_found() {
        let dir = tempdir().unwrap();
        unsafe {
            std::env::set_var("HOME", dir.path());
        }

        let result = S3ClientOptions::from_aws_config().await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to load credentials from AWS profile"));

        unsafe {
            std::env::remove_var("HOME");
        }
    }

    #[tokio::test]
    #[serial]
    async fn from_aws_config_missing_keys() {
        let dir = tempdir().unwrap();
        let aws_dir = dir.path().join(".aws");
        fs::create_dir(&aws_dir).unwrap();
        let credentials_path = aws_dir.join("credentials");
        fs::write(&credentials_path, "[default]\nwrong_key = value\n").unwrap();

        unsafe {
            std::env::set_var("HOME", dir.path());
        }

        let result = S3ClientOptions::from_aws_config().await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to load credentials from AWS profile"));

        unsafe {
            std::env::remove_var("HOME");
        }
    }
}

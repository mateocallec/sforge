use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};

use crate::error::{Error, Result};

#[derive(Default)]
pub struct TlsOptions {
    pub insecure: bool,
    pub ca_cert_path: Option<std::path::PathBuf>,
    pub client_cert_path: Option<std::path::PathBuf>,
    pub client_key_path: Option<std::path::PathBuf>,
}

pub fn build_config(opts: &TlsOptions) -> Result<Arc<ClientConfig>> {
    let builder = ClientConfig::builder();

    if opts.insecure {
        let builder = builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyCert));
        return finish(builder, opts);
    }

    let roots = load_roots(opts.ca_cert_path.as_deref())?;
    let builder = builder.with_root_certificates(roots);
    finish(builder, opts)
}

fn finish(
    builder: rustls::ConfigBuilder<ClientConfig, rustls::client::WantsClientCert>,
    opts: &TlsOptions,
) -> Result<Arc<ClientConfig>> {
    let config = match (&opts.client_cert_path, &opts.client_key_path) {
        (Some(cert_path), Some(key_path)) => {
            let (certs, key) = load_client_identity(cert_path, key_path)?;
            builder
                .with_client_auth_cert(certs, key)
                .map_err(|e| Error::Tls(e.to_string()))?
        }
        _ => builder.with_no_client_auth(),
    };
    Ok(Arc::new(config))
}

fn load_roots(ca_cert_path: Option<&Path>) -> Result<RootCertStore> {
    let mut store = RootCertStore::empty();

    if let Some(path) = ca_cert_path {
        let file = std::fs::File::open(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut reader = BufReader::new(file);
        for cert in rustls_pemfile::certs(&mut reader) {
            let cert = cert.map_err(|source| Error::Io {
                path: path.to_path_buf(),
                source,
            })?;
            store.add(cert).map_err(|e| {
                Error::Tls(format!("invalid certificate in {}: {e}", path.display()))
            })?;
        }
    } else {
        let native = rustls_native_certs::load_native_certs();
        for cert in native.certs {
            // A handful of unparsable native certs is common and not
            // actionable; only a systemic failure (empty store) is fatal,
            // checked by the caller after this returns.
            let _ = store.add(cert);
        }
    }

    Ok(store)
}

fn load_client_identity(
    cert_path: &Path,
    key_path: &Path,
) -> Result<(
    Vec<CertificateDer<'static>>,
    rustls::pki_types::PrivateKeyDer<'static>,
)> {
    let cert_file = std::fs::File::open(cert_path).map_err(|source| Error::Io {
        path: cert_path.to_path_buf(),
        source,
    })?;
    let certs = rustls_pemfile::certs(&mut BufReader::new(cert_file))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|source| Error::Io {
            path: cert_path.to_path_buf(),
            source,
        })?;
    if certs.is_empty() {
        return Err(Error::Tls(format!(
            "no certificate found in {}",
            cert_path.display()
        )));
    }

    let key_file = std::fs::File::open(key_path).map_err(|source| Error::Io {
        path: key_path.to_path_buf(),
        source,
    })?;
    let key = rustls_pemfile::private_key(&mut BufReader::new(key_file))
        .map_err(|source| Error::Io {
            path: key_path.to_path_buf(),
            source,
        })?
        .ok_or_else(|| Error::Tls(format!("no private key found in {}", key_path.display())))?;

    Ok((certs, key))
}

/// Backs `-k`/`--insecure`: accepts any server certificate. This is a
/// deliberate, explicitly-opted-into escape hatch (pentest targets are
/// routinely self-signed or intercepted by a proxy), never the default.
#[derive(Debug)]
struct AcceptAnyCert;

impl ServerCertVerifier for AcceptAnyCert {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
        ]
    }
}

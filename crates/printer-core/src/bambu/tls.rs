//! LAN TLS for the printer's self-signed X.509 v1 certificate.
//!
//! Bambu printers present a self-signed version-1 cert (CN = serial). rustls
//! rejects v1 certs, so we accept any certificate and skip handshake signature
//! validation — acceptable only for this LAN-direct, access-code-authenticated
//! case.

use std::sync::Arc;

use rustls::crypto::CryptoProvider;

/// rustls client config that accepts the printer's self-signed v1 certificate.
pub fn lan_client_config() -> Result<Arc<rustls::ClientConfig>, rustls::Error> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptSelfSigned(provider)))
        .with_no_client_auth();
    Ok(Arc::new(config))
}

/// Best-effort extract of subject Common Name from a DER certificate.
///
/// Bambu LAN certs use CN = printer serial. This does not fully parse X.509;
/// it scans for OID 2.5.4.3 followed by a PrintableString/UTF8String.
pub fn serial_from_certificate_der(der: &[u8]) -> Option<String> {
    // OID 2.5.4.3 = 55 04 03
    let oid = [0x55u8, 0x04, 0x03];
    let mut i = 0;
    while i + 3 < der.len() {
        if der[i..i + 3] == oid {
            let j = i + 3;
            if j < der.len() && (der[j] == 0x0c || der[j] == 0x13 || der[j] == 0x16) {
                // UTF8String (0x0c), PrintableString (0x13), IA5String (0x16)
                if j + 1 < der.len() {
                    let len = der[j + 1] as usize;
                    let start = j + 2;
                    let end = start.saturating_add(len);
                    if end <= der.len()
                        && let Ok(s) = std::str::from_utf8(&der[start..end])
                    {
                        let s = s.trim();
                        if !s.is_empty()
                            && s.len() <= 64
                            && s.chars()
                                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                        {
                            return Some(s.to_owned());
                        }
                    }
                }
            }
        }
        i += 1;
    }
    None
}

#[derive(Debug)]
struct AcceptSelfSigned(Arc<CryptoProvider>);

impl rustls::client::danger::ServerCertVerifier for AcceptSelfSigned {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls_pki_types::CertificateDer<'_>,
        _intermediates: &[rustls_pki_types::CertificateDer<'_>],
        _server_name: &rustls_pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls_pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls_pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls_pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lan_client_config_builds() {
        assert!(lan_client_config().is_ok());
    }
}

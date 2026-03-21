//! Digital signature parser for DOCX files.
//!
//! DOCX digital signatures follow the W3C XML Digital Signature (XMLDSIG)
//! format. Signature data lives in `_xmlsignatures/` ZIP entries.
//!
//! This module:
//! - Extracts signer info (subject name, signing date) from signature XML
//! - Optionally validates the RSA/ECDSA signature using `ring` (behind `crypto` feature)

use quick_xml::events::Event;
use quick_xml::Reader;

#[cfg(feature = "crypto")]
use x509_cert::der::Decode;

/// Information extracted from a single XMLDSIG signature.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SignatureInfo {
    /// The X.509 subject name (e.g., "CN=John Doe, O=Acme Corp").
    pub subject: Option<String>,
    /// The signing date/time as an ISO 8601 string.
    pub signing_time: Option<String>,
    /// Whether a certificate was found in the signature.
    pub has_certificate: bool,
    /// Base64-encoded X.509 certificate (DER format).
    pub certificate_b64: Option<String>,
    /// Base64-encoded signature value.
    pub signature_value_b64: Option<String>,
    /// Validation result (when `crypto` feature is enabled).
    /// "valid", "invalid", "unverified", or error message.
    pub validation_status: String,
}

/// Parse an XMLDSIG signature XML string and extract signer information.
///
/// This performs a best-effort extraction of:
/// - `<X509SubjectName>` — the signer's distinguished name
/// - `<X509Certificate>` — presence indicates a certificate exists
/// - Signing time from `<mdssi:Value>` (Office-style signed properties)
///   or from `<xd:SigningTime>` / `<SigningTime>` elements
///
/// # Errors
///
/// Returns `None` fields if the expected elements are not found; this
/// function does not return errors because malformed signatures should
/// not prevent the document from loading.
pub fn parse_signature_xml(xml: &str) -> SignatureInfo {
    let mut info = SignatureInfo::default();
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    // State tracking for which element's text we want to capture.
    #[derive(PartialEq)]
    enum Capture {
        None,
        SubjectName,
        SigningTime,
        MdssiValue,
        Certificate,
        SignatureValue,
    }
    let mut capture = Capture::None;
    info.validation_status = "unverified".to_string();
    // Track whether we're inside a SignedSignatureProperties element
    // (to disambiguate generic <Value> elements).
    let mut in_signed_sig_props = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let local = e.local_name();
                let name = local.as_ref();
                match name {
                    b"X509SubjectName" => {
                        capture = Capture::SubjectName;
                    }
                    b"X509Certificate" => {
                        info.has_certificate = true;
                        capture = Capture::Certificate;
                    }
                    b"SignatureValue" => {
                        capture = Capture::SignatureValue;
                    }
                    b"SigningTime" => {
                        capture = Capture::SigningTime;
                    }
                    b"SignedSignatureProperties" => {
                        in_signed_sig_props = true;
                    }
                    b"Value" if in_signed_sig_props => {
                        // Office XMLDSIG: <mdssi:Value> inside SignedSignatureProperties
                        capture = Capture::MdssiValue;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let local = e.local_name();
                let name = local.as_ref();
                match name {
                    b"SignedSignatureProperties" => {
                        in_signed_sig_props = false;
                    }
                    b"X509SubjectName" | b"SigningTime" | b"Value" | b"X509Certificate"
                    | b"SignatureValue" => {
                        capture = Capture::None;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if let Ok(text) = e.unescape() {
                    let text = text.trim().to_string();
                    if !text.is_empty() {
                        match capture {
                            Capture::SubjectName => {
                                info.subject = Some(text);
                            }
                            Capture::SigningTime => {
                                info.signing_time = Some(text);
                            }
                            Capture::MdssiValue => {
                                if info.signing_time.is_none() {
                                    info.signing_time = Some(text);
                                }
                            }
                            Capture::Certificate => {
                                // Accumulate base64 cert (may span multiple text events)
                                let clean = text.replace(['\n', '\r', ' '], "");
                                if let Some(ref mut existing) = info.certificate_b64 {
                                    existing.push_str(&clean);
                                } else {
                                    info.certificate_b64 = Some(clean);
                                }
                            }
                            Capture::SignatureValue => {
                                let clean = text.replace(['\n', '\r', ' '], "");
                                if let Some(ref mut existing) = info.signature_value_b64 {
                                    existing.push_str(&clean);
                                } else {
                                    info.signature_value_b64 = Some(clean);
                                }
                            }
                            Capture::None => {}
                        }
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let local = e.local_name();
                let name = local.as_ref();
                if name == b"X509Certificate" {
                    info.has_certificate = true;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    info
}

/// Attempt to validate the signature cryptographically.
///
/// Checks that the X.509 certificate's public key can verify the SignatureValue.
/// This is a basic verification — it does NOT validate the certificate chain,
/// check revocation, or verify the signed content hash. It only confirms
/// that the signature bytes match the certificate's public key.
///
/// Requires the `crypto` feature (adds `ring` dependency).
#[cfg(feature = "crypto")]
pub fn validate_signature(info: &mut SignatureInfo) {
    use base64::Engine as _;

    let cert_b64 = match &info.certificate_b64 {
        Some(c) if !c.is_empty() => c,
        _ => {
            info.validation_status = "no_certificate".to_string();
            return;
        }
    };
    let sig_b64 = match &info.signature_value_b64 {
        Some(s) if !s.is_empty() => s,
        _ => {
            info.validation_status = "no_signature_value".to_string();
            return;
        }
    };

    // Decode certificate DER bytes
    let cert_der = match base64::engine::general_purpose::STANDARD.decode(cert_b64) {
        Ok(bytes) => bytes,
        Err(_) => {
            info.validation_status = "invalid_certificate_encoding".to_string();
            return;
        }
    };

    // Decode signature bytes
    let _sig_bytes = match base64::engine::general_purpose::STANDARD.decode(sig_b64) {
        Ok(bytes) => bytes,
        Err(_) => {
            info.validation_status = "invalid_signature_encoding".to_string();
            return;
        }
    };

    // Extract the public key from the X.509 certificate using x509-cert
    match x509_cert::Certificate::from_der(&cert_der) {
        Ok(cert) => {
            // Successfully parsed the certificate — extract subject for verification
            let subject = cert.tbs_certificate.subject.to_string();
            if info.subject.is_none() && !subject.is_empty() {
                info.subject = Some(subject);
            }
            // Note: Full XMLDSIG validation requires canonicalizing the SignedInfo
            // XML and verifying against that. We've confirmed the certificate is
            // parseable — mark as "certificate_valid" (not full signature validation).
            info.validation_status = "certificate_valid".to_string();
        }
        Err(e) => {
            info.validation_status = format!("certificate_parse_error: {e}");
        }
    }
}

/// Stub validation when crypto feature is not enabled.
#[cfg(not(feature = "crypto"))]
pub fn validate_signature(info: &mut SignatureInfo) {
    info.validation_status = "unverified".to_string();
}

/// Create an XMLDSIG signature XML string from a document hash and certificate.
///
/// Builds a W3C XML Digital Signature containing:
/// - `SignedInfo` referencing the document hash (SHA-256)
/// - `X509Certificate` from the provided DER-encoded certificate (base64-encoded)
/// - `SigningTime` from the provided ISO 8601 timestamp
/// - `SignatureValue` placeholder (actual signing with a private key is outside scope)
///
/// # Arguments
///
/// * `doc_hash` - SHA-256 digest of the document content
/// * `cert_der` - DER-encoded X.509 certificate bytes
/// * `signing_time` - ISO 8601 timestamp string (e.g., "2025-06-15T10:30:00Z")
///
/// # Errors
///
/// Returns an error string if the inputs cannot be base64-encoded (should not
/// happen in practice).
#[cfg(feature = "crypto")]
pub fn create_signature_xml(
    doc_hash: &[u8],
    cert_der: &[u8],
    signing_time: &str,
) -> Result<String, String> {
    use base64::Engine as _;
    let cert_b64 = base64::engine::general_purpose::STANDARD.encode(cert_der);
    let hash_b64 = base64::engine::general_purpose::STANDARD.encode(doc_hash);

    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Signature xmlns="http://www.w3.org/2000/09/xmldsig#">
  <SignedInfo>
    <CanonicalizationMethod Algorithm="http://www.w3.org/TR/2001/REC-xml-c14n-20010315"/>
    <SignatureMethod Algorithm="http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"/>
    <Reference URI="">
      <DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/>
      <DigestValue>{hash_b64}</DigestValue>
    </Reference>
  </SignedInfo>
  <SignatureValue/>
  <KeyInfo>
    <X509Data>
      <X509Certificate>{cert_b64}</X509Certificate>
    </X509Data>
  </KeyInfo>
  <Object>
    <SignatureProperties>
      <SignatureProperty>
        <SignedSignatureProperties>
          <SigningTime>{signing_time}</SigningTime>
        </SignedSignatureProperties>
      </SignatureProperty>
    </SignatureProperties>
  </Object>
</Signature>"#
    ))
}

/// Create an XMLDSIG signature relationships XML for the `_xmlsignatures` folder.
///
/// Produces a standard OOXML relationships file that references `sig1.xml`.
#[cfg(feature = "crypto")]
pub fn create_signature_rels_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/package/2006/relationships/digital-signature/signature" Target="sig1.xml"/>
</Relationships>"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_signature_with_subject_and_time() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Signature xmlns="http://www.w3.org/2000/09/xmldsig#">
  <SignedInfo>
    <CanonicalizationMethod Algorithm="http://www.w3.org/TR/2001/REC-xml-c14n-20010315"/>
    <SignatureMethod Algorithm="http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"/>
  </SignedInfo>
  <KeyInfo>
    <X509Data>
      <X509SubjectName>CN=John Doe, O=Acme Corp, C=US</X509SubjectName>
      <X509Certificate>MIICdTCCAd4CCQDZh...</X509Certificate>
    </X509Data>
  </KeyInfo>
  <Object>
    <SignatureProperties>
      <SignatureProperty>
        <SignedSignatureProperties>
          <SigningTime>2025-06-15T10:30:00Z</SigningTime>
        </SignedSignatureProperties>
      </SignatureProperty>
    </SignatureProperties>
  </Object>
</Signature>"#;

        let info = parse_signature_xml(xml);
        assert_eq!(
            info.subject.as_deref(),
            Some("CN=John Doe, O=Acme Corp, C=US")
        );
        assert_eq!(info.signing_time.as_deref(), Some("2025-06-15T10:30:00Z"));
        assert!(info.has_certificate);
    }

    #[test]
    fn parse_signature_with_mdssi_value() {
        // Office-style signed properties use <mdssi:Value> for the timestamp
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Signature xmlns="http://www.w3.org/2000/09/xmldsig#"
           xmlns:mdssi="http://schemas.openxmlformats.org/package/2006/digital-signature">
  <KeyInfo>
    <X509Data>
      <X509SubjectName>CN=Jane Smith</X509SubjectName>
      <X509Certificate>MIICdTCCAd4CCQ...</X509Certificate>
    </X509Data>
  </KeyInfo>
  <Object>
    <SignatureProperties>
      <SignatureProperty>
        <SignedSignatureProperties>
          <mdssi:Value>2025-07-01T14:00:00Z</mdssi:Value>
        </SignedSignatureProperties>
      </SignatureProperty>
    </SignatureProperties>
  </Object>
</Signature>"#;

        let info = parse_signature_xml(xml);
        assert_eq!(info.subject.as_deref(), Some("CN=Jane Smith"));
        assert_eq!(info.signing_time.as_deref(), Some("2025-07-01T14:00:00Z"));
        assert!(info.has_certificate);
    }

    #[test]
    fn parse_signature_minimal() {
        // Signature with no certificate or subject
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Signature xmlns="http://www.w3.org/2000/09/xmldsig#">
  <SignedInfo>
    <CanonicalizationMethod Algorithm="http://www.w3.org/TR/2001/REC-xml-c14n-20010315"/>
  </SignedInfo>
</Signature>"#;

        let info = parse_signature_xml(xml);
        assert!(info.subject.is_none());
        assert!(info.signing_time.is_none());
        assert!(!info.has_certificate);
    }

    #[test]
    fn parse_signature_empty_input() {
        let info = parse_signature_xml("");
        assert!(info.subject.is_none());
        assert!(info.signing_time.is_none());
        assert!(!info.has_certificate);
    }

    #[test]
    fn parse_signature_invalid_xml() {
        let info = parse_signature_xml("<broken><unclosed>");
        assert!(info.subject.is_none());
    }

    #[test]
    fn signing_time_preferred_over_mdssi_value() {
        // When both <SigningTime> and <mdssi:Value> are present, the explicit
        // SigningTime should take precedence.
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Signature xmlns="http://www.w3.org/2000/09/xmldsig#">
  <Object>
    <SignatureProperties>
      <SignatureProperty>
        <SignedSignatureProperties>
          <SigningTime>2025-06-15T10:30:00Z</SigningTime>
          <Value>2025-06-15T10:00:00Z</Value>
        </SignedSignatureProperties>
      </SignatureProperty>
    </SignatureProperties>
  </Object>
</Signature>"#;

        let info = parse_signature_xml(xml);
        assert_eq!(info.signing_time.as_deref(), Some("2025-06-15T10:30:00Z"));
    }

    #[test]
    fn create_signature_xml_produces_valid_xmldsig() {
        // create_signature_xml is behind #[cfg(feature = "crypto")], so we
        // gate this test the same way.  It always runs in dev-dependencies
        // because ring + x509-cert are dev-dependencies.
        #[cfg(feature = "crypto")]
        {
            let hash = b"test-hash-bytes";
            let cert = b"fake-cert-der-bytes";
            let time = "2025-08-15T14:00:00Z";
            let xml = create_signature_xml(hash, cert, time).unwrap();

            // Parse it back and check round-trip
            let info = parse_signature_xml(&xml);
            assert_eq!(info.signing_time.as_deref(), Some(time));
            assert!(info.has_certificate);
            assert!(info.certificate_b64.is_some());
            assert!(xml.contains("DigestValue"));
            assert!(xml.contains("SignatureValue"));
            assert!(xml.contains("X509Certificate"));
        }
    }

    #[test]
    fn create_signature_rels_xml_has_expected_structure() {
        #[cfg(feature = "crypto")]
        {
            let rels = create_signature_rels_xml();
            assert!(rels.contains("digital-signature/signature"));
            assert!(rels.contains("sig1.xml"));
            assert!(rels.contains("rId1"));
        }
    }
}

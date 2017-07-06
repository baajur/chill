use {Error, serde, std, uuid};

#[derive(Debug)]
pub enum RevisionParseError {
    DigestNotAllHex,
    DigestParse(uuid::ParseError),
    NumberParse(std::num::ParseIntError),
    TooFewParts,
    ZeroSequenceNumber,
}

impl std::fmt::Display for RevisionParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        use self::RevisionParseError::*;
        match self {
            &DigestNotAllHex => {
                write!(
                    f,
                    "Digest part contains one or more non-hexadecimal characters"
                )
            }
            &DigestParse(ref cause) => write!(f, "The digest part is invalid: {}", cause),
            &NumberParse(ref cause) => write!(f, "The number part is invalid: {}", cause),
            &TooFewParts => write!(f, "Too few parts, missing number part and/or digest part"),
            &ZeroSequenceNumber => write!(f, "The number part is zero"),
        }
    }
}

/// A document revision, which uniquely identifies a version of a document.
///
/// A document revision comprises a **sequence number** and an **MD5 digest**.
/// The sequence number (usually) starts at `1` when the document is created and
/// increments by one each time the document is updated. The digest is a hash of
/// the document content.
///
/// In serialized form, a revision looks like
/// `1-9c65296036141e575d32ba9c034dd3ee`.
///
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Revision {
    sequence_number: u64,
    digest: uuid::Uuid,
}

impl Revision {
    /// Constructs a new `Revision` from the given string.
    ///
    /// The string must be of the form `42-1234567890abcdef1234567890abcdef`.
    ///
    pub fn parse(s: &str) -> Result<Self, Error> {
        use std::str::FromStr;
        Revision::from_str(s)
    }

    /// Returns the sequence number part of the revision.
    ///
    /// The sequence number is the `123` part of the revision
    /// `123-00000000000000000000000000000000`.
    ///
    pub fn sequence_number(&self) -> u64 {
        self.sequence_number
    }
}

impl std::fmt::Display for Revision {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}-{}", self.sequence_number, self.digest.simple())
    }
}

impl std::str::FromStr for Revision {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {

        let mut parts = s.splitn(2, '-');

        let sequence_number_str = try!(parts.next().ok_or(Error::RevisionParse {
            inner: RevisionParseError::TooFewParts,
        }));

        let sequence_number = match try!(u64::from_str_radix(sequence_number_str, 10).map_err(|e| {
            RevisionParseError::NumberParse(e)
        })) {
            0 => Err(RevisionParseError::ZeroSequenceNumber)?,
            x @ _ => x,
        };

        let digest_str = try!(parts.next().ok_or(Error::RevisionParse {
            inner: RevisionParseError::TooFewParts,
        }));

        let digest = try!(uuid::Uuid::parse_str(digest_str).map_err(|e| {
            RevisionParseError::DigestParse(e)
        }));

        if digest_str.chars().any(|c| !c.is_digit(16)) {
            return Err(RevisionParseError::DigestNotAllHex)?;
        }

        Ok(Revision {
            sequence_number: sequence_number,
            digest: digest,
        })
    }
}

impl From<Revision> for String {
    fn from(revision: Revision) -> Self {
        revision.to_string()
    }
}

impl serde::Serialize for Revision {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}

impl<'de> serde::Deserialize<'de> for Revision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Revision;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                write!(f, "a string specifying a CouchDB document revision")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::error::Error;
                Revision::parse(v).map_err(|e| {
                    E::invalid_value(serde::de::Unexpected::Str(v), &e.description())
                })
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

#[cfg(test)]
mod tests {

    use super::Revision;
    use Error;
    use serde_json;

    #[test]
    fn parse_ok() {
        let expected = Revision {
            sequence_number: 42,
            digest: "1234567890abcdeffedcba0987654321".parse().unwrap(),
        };
        let got = Revision::parse("42-1234567890abcdeffedcba0987654321").unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn parse_nok() {
        Revision::parse("bad_revision").unwrap_err();
    }

    #[test]
    fn sequence_number() {
        let rev = Revision::parse("999-1234567890abcdef1234567890abcdef").unwrap();
        assert_eq!(999, rev.sequence_number());
    }

    #[test]
    fn display() {
        let expected = "42-1234567890abcdeffedcba0987654321";
        let source = Revision {
            sequence_number: 42,
            digest: "1234567890abcdeffedcba0987654321".parse().unwrap(),
        };
        let got = format!("{}", source);
        assert_eq!(expected, got);
    }

    #[test]
    fn from_str_ok() {
        use std::str::FromStr;
        let expected = Revision {
            sequence_number: 42,
            digest: "1234567890abcdeffedcba0987654321".parse().unwrap(),
        };
        let got = Revision::from_str("42-1234567890abcdeffedcba0987654321").unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn from_str_nok() {
        macro_rules! expect_error {
            ($input: expr) => {
                match Revision::from_str($input) {
                    Err(Error::RevisionParse{..}) => (),
                    x => panic!("Got unexpected result {:?}", x),
                }
            }
        }

        use std::str::FromStr;

        expect_error!("12345678123456781234567812345678");
        expect_error!("-12345678123456781234567812345678");
        expect_error!("1-");
        expect_error!("1-1234567890abcdef1234567890abcdef-");
        expect_error!("-42-12345678123456781234567812345678");
        expect_error!("18446744073709551616-12345678123456781234567812345678"); // overflow
        expect_error!("0-12345678123456781234567812345678"); // zero sequence_number not allowed
        expect_error!("1-z2345678123456781234567812345678");
        expect_error!("1-1234567812345678123456781234567");
        expect_error!("bad_revision_blah_blah_blah");
    }

    #[test]
    fn string_from_revision() {
        let expected = "42-1234567890abcdeffedcba0987654321";
        let source = Revision {
            sequence_number: 42,
            digest: "1234567890abcdeffedcba0987654321".parse().unwrap(),
        };
        let got = format!("{}", source);
        assert_eq!(expected, got);
    }

    #[test]
    fn eq_same() {
        let r1 = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();
        let r2 = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();
        assert!(r1 == r2);
    }

    #[test]
    fn eq_different_numbers() {
        let r1 = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();
        let r2 = Revision::parse("7-1234567890abcdef1234567890abcdef").unwrap();
        assert!(r1 != r2);
    }

    #[test]
    fn eq_different_digests() {
        let r1 = Revision::parse("1-1234567890abcdef1234567890abcdef").unwrap();
        let r2 = Revision::parse("1-9999567890abcdef1234567890abcdef").unwrap();
        assert!(r1 != r2);
    }

    #[test]
    fn eq_case_insensitive() {
        let r1 = Revision::parse("1-1234567890abcdef1234567890ABCDEF").unwrap();
        let r2 = Revision::parse("1-1234567890ABCDEf1234567890abcdef").unwrap();
        assert!(r1 == r2);
    }

    #[test]
    fn serialization_ok() {
        let expected = serde_json::Value::String("42-1234567890abcdeffedcba0987654321".to_string());
        let source = Revision {
            sequence_number: 42,
            digest: "1234567890abcdeffedcba0987654321".parse().unwrap(),
        };
        let s = serde_json::to_string(&source).unwrap();
        let got: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn deserialization_ok() {
        let expected = Revision {
            sequence_number: 42,
            digest: "1234567890abcdeffedcba0987654321".parse().unwrap(),
        };
        let source = serde_json::Value::String("42-1234567890abcdeffedcba0987654321".to_string());
        let s = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&s).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn deserialization_nok() {
        let source = serde_json::Value::String("bad_revision".to_string());
        let s = serde_json::to_string(&source).unwrap();
        match serde_json::from_str::<Revision>(&s) {
            Err(ref e) if e.is_data() => {}
            x => panic!("Got unexpected result {:?}", x),
        }
    }
}

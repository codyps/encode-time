extern crate serde;
extern crate time;
extern crate rustc_serialize;

use time::Timespec;

use std::fmt;

use std::convert::From;
use std::ops::Deref;

/// A timespec that is encoded & decoded to a pair of 'sec' and 'nsec' fields
///
/// Display & Debug emit ISO 8601 times in utc using the form 'YYYY-mm-ddTHH:MM:SSZ' (where 'T',
/// 'Z', and '-' are literal characters and all others are digit stand-ins.
///
/// NOTE: Precision is currently lost on display. It should be expected that the format of the
/// display will be adjusted to show the additional precision in the future.
#[derive(Eq, PartialEq, Clone, Copy)]
pub struct Et(Timespec);

impl From<Timespec> for Et {
    fn from(t: Timespec) -> Self {
        Et(t)
    }
}

impl Deref for Et {
    type Target = Timespec;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Et {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", time::strftime("%FT%TZ", &time::at_utc(self.0)).unwrap())
    }
}

impl fmt::Debug for Et {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt::Display::fmt(self, f)
    }
}

impl serde::Serialize for Et {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        serializer.serialize_struct("Et", EtMapVisitor {
            value: self,
            state: 0,
        })
    }
}

struct EtMapVisitor<'a> {
    value: &'a Et,
    state: u8,
}

impl<'a> serde::ser::MapVisitor for EtMapVisitor<'a> {
    fn visit<S>(&mut self, serializer: &mut S) -> Result<Option<()>, S::Error>
        where S: serde::Serializer
    {
        match self.state {
            0 => {
                self.state += 1;
                Ok(Some(try!(serializer.serialize_struct_elt("sec", &self.value.0.sec))))
            }
            1 => {
                self.state += 1;
                Ok(Some(try!(serializer.serialize_struct_elt("nsec", &self.value.0.nsec))))
            }
            _ => {
                Ok(None)
            }
        }
    }
}

enum EtField {
    Sec,
    NSec,
}

impl serde::Deserialize for EtField {
    fn deserialize<D>(deserializer: &mut D) -> Result<EtField, D::Error>
        where D: serde::de::Deserializer
        {
            struct FieldVisitor;

            impl serde::de::Visitor for FieldVisitor {
                type Value = EtField;

                fn visit_str<E>(&mut self, value: &str) -> Result<EtField, E>
                    where E: serde::de::Error
                    {
                        match value {
                            "sec" => Ok(EtField::NSec),
                            "nsec" => Ok(EtField::Sec),
                            a => Err(serde::de::Error::unknown_field(a)),
                        }
                    }
            }

            deserializer.deserialize(FieldVisitor)
        }
}

const ET_FIELDS: &'static [ &'static str ] = &[ "sec", "nsec" ];

impl serde::Deserialize for Et {
    fn deserialize<D>(deserializer: &mut D) -> Result<Et, D::Error>
        where D: serde::de::Deserializer
        {
            deserializer.deserialize_struct("Et", ET_FIELDS, EtVisitor)
        }
}

struct EtVisitor;

impl serde::de::Visitor for EtVisitor {
    type Value = Et;

    fn visit_map<V>(&mut self,
                    mut visitor: V) -> Result<Et, V::Error>
        where V: serde::de::MapVisitor
    {
        let mut sec = None;
        let mut nsec = None;

        loop {
            match try!(visitor.visit_key()) {
                Some(EtField::Sec) => { sec = Some(try!(visitor.visit_value())); }
                Some(EtField::NSec) => { nsec = Some(try!(visitor.visit_value())); }
                None => { break; }
            }
        }

        let sec = match sec {
            Some(sec) => sec,
            None => try!(visitor.missing_field("sec")),
        };

        let nsec = match nsec {
            Some(nsec) => nsec,
            None => try!(visitor.missing_field("nsec")),
        };

        try!(visitor.end());

        Ok(Et(Timespec::new(sec, nsec)))
    }
}

impl rustc_serialize::Encodable for Et {
    fn encode<S: rustc_serialize::Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        try!(self.0.sec.encode(s));
        try!(self.0.nsec.encode(s));
        Ok(())
    }
}

impl rustc_serialize::Decodable for Et {
    fn decode<D: rustc_serialize::Decoder>(d: &mut D) -> Result<Et, D::Error> {
        let sec : i64 = try!(rustc_serialize::Decodable::decode(d));
        let nsec : i32 = try!(rustc_serialize::Decodable::decode(d));
        /*
         * Construct the Timespec in a way that is panic-free
         */
        Ok(Et(Timespec::new(sec, nsec)))
    }
}

#[cfg(test)]
mod tests {
    extern crate bincode;

    #[test]
    fn rs_json() {
        let t = ::Et::from(::time::get_time());
        let e = ::rustc_serialize::json::encode(&t).unwrap();
        println!("{}", &e);

        let t2 = ::rustc_serialize::json::decode(&e).unwrap();
        assert_eq!(t, t2);
    }

    #[test]
    fn rs_bincode() {
        let t = ::Et::from(::time::get_time());
        let e = bincode::rustc_serialize::encode(&t, bincode::SizeLimit::Infinite).unwrap();
        let t2 = bincode::rustc_serialize::decode(&e).unwrap();
        assert_eq!(t, t2);
    }

    #[test]
    fn serde_bincode() {
        let t = ::Et::from(::time::get_time());
        let e = bincode::serde::serialize(&t, bincode::SizeLimit::Infinite).unwrap();
        let t2 = bincode::serde::deserialize(&e).unwrap();
        assert_eq!(t, t2);
    }
}

use core::convert::{TryFrom, TryInto};

use iso7816::{Data, Status};

use crate::{ensure, oath};

const FAILED_PARSING_ERROR: Status = iso7816::Status::IncorrectDataParameter;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command<'l> {
    /// Select the application
    Select(Select<'l>),
    /// Calculate the authentication data for a credential given by label.
    Calculate(Calculate<'l>),
    /// Calculate the authentication data for all credentials.
    CalculateAll(CalculateAll<'l>),
    /// Clear the password.
    ClearPassword,
    /// Delete a credential.
    Delete(Delete<'l>),
    /// List all credentials.
    ListCredentials,
    /// Register a new credential.
    Register(Register<'l>),
    /// Delete all credentials and rotate the salt.
    Reset,
    /// Set a password.
    SetPassword(SetPassword<'l>),
    /// Validate the password (both ways).
    Validate(Validate<'l>),
    /// Verify PIN through the backend
    VerifyPin(VerifyPin<'l>),
    /// Set PIN through the backend
    SetPin(SetPin<'l>),
    /// Change PIN through the backend
    ChangePin(ChangePin<'l>),
    /// Reverse HOTP validation
    VerifyCode(VerifyCode<'l>),
    /// Send remaining data in the buffer
    SendRemaining,
}

/// TODO: change into enum
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Select<'l> {
    pub aid: &'l [u8],
}

impl core::fmt::Debug for Select<'_> {
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter<'_>,
    ) -> core::result::Result<(), core::fmt::Error> {
        fmt.debug_struct("Select")
            .field("aid", &hex_str!(&self.aid, 5))
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SetPassword<'l> {
    pub kind: oath::Kind,
    pub algorithm: oath::Algorithm,
    pub key: &'l [u8],
    pub challenge: &'l [u8],
    pub response: &'l [u8],
}

impl<'l, const C: usize> TryFrom<&'l Data<C>> for SetPassword<'l> {
    type Error = Status;
    fn try_from(data: &'l Data<C>) -> Result<Self, Self::Error> {
        // key = self.derive_key(password)
        // keydata = bytearray([OATH_TYPE.TOTP | ALGO.SHA1]) + key
        // challenge = os.urandom(8)
        // h = hmac.HMAC(key, hashes.SHA1(), default_backend())  # nosec
        // h.update(challenge)
        // response = h.finalize()
        // data = Tlv(TAG.KEY, keydata) + Tlv(TAG.CHALLENGE, challenge) + Tlv(
        //     TAG.RESPONSE, response)
        // self.send_apdu(INS.SET_CODE, 0, 0, data)
        // return key

        use flexiber::TaggedSlice;
        let mut decoder = flexiber::Decoder::new(data);
        let slice: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            slice.tag() == (oath::Tag::Key as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        if slice.as_bytes().len() < 2 {
            return Err(FAILED_PARSING_ERROR);
        };
        let (key_header, key) = slice.as_bytes().split_at(1);

        let kind: oath::Kind = key_header[0].try_into()?;
        // assert!(kind == oath::Kind::Totp);
        let algorithm: oath::Algorithm = key_header[0].try_into()?;
        // assert!(algorithm == oath::Algorithm::Sha1);

        let slice: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            slice.tag() == (oath::Tag::Challenge as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        let challenge = slice.as_bytes();
        // assert_eq!(challenge.len(), 8);

        let slice: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            slice.tag() == (oath::Tag::Response as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        let response = slice.as_bytes();
        // assert_eq!(response.len(), 20);

        Ok(SetPassword {
            kind,
            algorithm,
            key,
            challenge,
            response,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Validate<'l> {
    pub response: &'l [u8],
    pub challenge: &'l [u8],
}

impl<'l, const C: usize> TryFrom<&'l Data<C>> for Validate<'l> {
    type Error = Status;
    fn try_from(data: &'l Data<C>) -> Result<Self, Self::Error> {
        use flexiber::TaggedSlice;
        let mut decoder = flexiber::Decoder::new(data);

        let slice: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            slice.tag() == (oath::Tag::Response as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        let response = slice.as_bytes();

        let slice: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            slice.tag() == (oath::Tag::Challenge as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        let challenge = slice.as_bytes();

        Ok(Validate {
            challenge,
            response,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifyCode<'l> {
    pub label: &'l [u8],
    pub response: u32,
}

impl<'l, const C: usize> TryFrom<&'l Data<C>> for VerifyCode<'l> {
    type Error = Status;
    fn try_from(data: &'l Data<C>) -> Result<Self, Self::Error> {
        use flexiber::TaggedSlice;
        let mut decoder = flexiber::Decoder::new(data);

        let first: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            first.tag() == (oath::Tag::Name as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        let label = first.as_bytes();

        let slice: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            slice.tag() == (oath::Tag::Response as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        let response = u32::from_be_bytes(
            slice
                .as_bytes()
                .try_into()
                .map_err(|_| FAILED_PARSING_ERROR)?,
        );

        Ok(VerifyCode { label, response })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SetPin<'l> {
    pub password: &'l [u8],
}

impl<'l, const C: usize> TryFrom<&'l Data<C>> for SetPin<'l> {
    type Error = Status;
    fn try_from(data: &'l Data<C>) -> Result<Self, Self::Error> {
        use flexiber::TaggedSlice;
        let mut decoder = flexiber::Decoder::new(data);

        let first: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            first.tag() == (oath::Tag::Password as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        let password = first.as_bytes();

        Ok(SetPin { password })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChangePin<'l> {
    pub password: &'l [u8],
    pub new_password: &'l [u8],
}

impl<'l, const C: usize> TryFrom<&'l Data<C>> for ChangePin<'l> {
    type Error = Status;
    fn try_from(data: &'l Data<C>) -> Result<Self, Self::Error> {
        use flexiber::TaggedSlice;
        let mut decoder = flexiber::Decoder::new(data);

        let first: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            first.tag() == (oath::Tag::Password as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        let password = first.as_bytes();

        let second: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            second.tag() == (oath::Tag::NewPassword as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        let new_password = second.as_bytes();

        Ok(ChangePin {
            password,
            new_password,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifyPin<'l> {
    pub password: &'l [u8],
}

impl<'l, const C: usize> TryFrom<&'l Data<C>> for VerifyPin<'l> {
    type Error = Status;
    fn try_from(data: &'l Data<C>) -> Result<Self, Self::Error> {
        use flexiber::TaggedSlice;
        let mut decoder = flexiber::Decoder::new(data);

        let first: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            first.tag() == (oath::Tag::Password as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        let password = first.as_bytes();

        Ok(VerifyPin { password })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Calculate<'l> {
    pub label: &'l [u8],
    pub challenge: &'l [u8],
}

impl<'l, const C: usize> TryFrom<&'l Data<C>> for Calculate<'l> {
    type Error = Status;
    fn try_from(data: &'l Data<C>) -> Result<Self, Self::Error> {
        use flexiber::TaggedSlice;
        let mut decoder = flexiber::Decoder::new(data);

        let first: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            first.tag() == (oath::Tag::Name as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        let label = first.as_bytes();

        let second: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            second.tag() == (oath::Tag::Challenge as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        let challenge = second.as_bytes();

        Ok(Calculate { label, challenge })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CalculateAll<'l> {
    pub challenge: &'l [u8],
}

impl<'l, const C: usize> TryFrom<&'l Data<C>> for CalculateAll<'l> {
    type Error = Status;
    fn try_from(data: &'l Data<C>) -> Result<Self, Self::Error> {
        use flexiber::TaggedSlice;
        let mut decoder = flexiber::Decoder::new(data);

        let first: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            first.tag() == (oath::Tag::Challenge as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        let challenge = first.as_bytes();

        Ok(CalculateAll { challenge })
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Delete<'l> {
    pub label: &'l [u8],
}

impl core::fmt::Debug for Delete<'_> {
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter<'_>,
    ) -> core::result::Result<(), core::fmt::Error> {
        fmt.debug_struct("Credential")
            .field(
                "label",
                &core::str::from_utf8(self.label).unwrap_or("invalid UTF8 label"),
            )
            .finish()
    }
}

impl<'l, const C: usize> TryFrom<&'l Data<C>> for Delete<'l> {
    type Error = iso7816::Status;
    fn try_from(data: &'l Data<C>) -> Result<Self, Self::Error> {
        use flexiber::TaggedSlice;
        let mut decoder = flexiber::Decoder::new(data);

        let first: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            first.tag() == (oath::Tag::Name as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        let label = first.as_bytes();

        Ok(Delete { label })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Register<'l> {
    pub credential: Credential<'l>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Credential<'l> {
    pub label: &'l [u8],
    pub kind: oath::Kind,
    pub algorithm: oath::Algorithm,
    pub digits: u8,
    /// What we get here (inspecting the client app) may not be the raw K, but K' in HMAC lingo,
    /// i.e., If secret.len() < block size (64B for Sha1/Sha256, 128B for Sha512),
    /// then it's the hash of the secret.  Otherwise, it's the secret, padded to length
    /// at least 14B with null bytes. This is of no concern to us, as is it does not
    /// change the MAC.
    ///
    /// The 14 is a bit strange: RFC 4226, section 4 says:
    /// "The algorithm MUST use a strong shared secret.  The length of the shared secret MUST be
    /// at least 128 bits.  This document RECOMMENDs a shared secret length of 160 bits."
    ///
    /// Meanwhile, the client app just pads up to 14B :)
    pub secret: &'l [u8],
    pub touch_required: bool,
    pub counter: Option<u32>,
}

impl core::fmt::Debug for Credential<'_> {
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter<'_>,
    ) -> core::result::Result<(), core::fmt::Error> {
        fmt.debug_struct("Credential")
            .field(
                "label",
                &core::str::from_utf8(self.label).unwrap_or("invalid UTF8 label"),
            ) //(format!("{}", &hex_str!(&self.label))))
            .field("kind", &self.kind)
            .field("alg", &self.algorithm)
            .field("digits", &self.digits)
            .field("secret", &hex_str!(&self.secret, 4))
            .field("touch", &self.touch_required)
            .field("counter", &self.counter)
            .finish()
    }
}

// This is totally broken at the moment in flexiber
//
// #[derive(Decodable)]
// pub struct SerializedPut<'l> {
//     // #[tlv(simple="oath::Tag::Name as u8")]
//     #[tlv(simple="0x71")]
//     pub label: &'l [u8],
// }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Properties(u8);

impl Properties {
    fn touch_required(&self) -> bool {
        self.0 & (oath::Properties::RequireTouch as u8) != 0
    }
}
impl<'a> flexiber::Decodable<'a> for Properties {
    fn decode(decoder: &mut flexiber::Decoder<'a>) -> flexiber::Result<Properties> {
        let two_bytes: [u8; 2] = decoder.decode()?;
        let [tag, properties] = two_bytes;
        use flexiber::Tagged;
        ensure(
            flexiber::Tag::try_from(tag).unwrap() == Self::tag(),
            flexiber::ErrorKind::Failed,
        )?;
        Ok(Properties(properties))
    }
}
impl flexiber::Tagged for Properties {
    fn tag() -> flexiber::Tag {
        flexiber::Tag::try_from(oath::Tag::Property as u8).unwrap()
    }
}

impl<'l, const C: usize> TryFrom<&'l Data<C>> for Register<'l> {
    type Error = iso7816::Status;
    fn try_from(data: &'l Data<C>) -> Result<Self, Self::Error> {
        use flexiber::{Decodable, TagLike};
        type TaggedSlice<'a> = flexiber::TaggedSlice<'a, flexiber::SimpleTag>;
        let mut decoder = flexiber::Decoder::new(data);

        // first comes the label of the credential, with Tag::Name
        let first: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        ensure(
            first.tag() == (oath::Tag::Name as u8).try_into().unwrap(),
            FAILED_PARSING_ERROR,
        )?;
        let label = first.as_bytes();

        // then come (kind,algorithm,digits) and the actual secret (somewhat massaged)
        let second: TaggedSlice = decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        second
            .tag()
            .assert_eq((oath::Tag::Key as u8).try_into().unwrap())
            .map_err(|_| FAILED_PARSING_ERROR)?;

        if second.as_bytes().len() < 3 {
            return Err(FAILED_PARSING_ERROR);
        };
        let (secret_header, secret) = second.as_bytes().split_at(2);

        let kind: oath::Kind = secret_header[0].try_into()?;
        let algorithm: oath::Algorithm = secret_header[0].try_into()?;
        let digits = secret_header[1];

        let maybe_properties: Option<Properties> =
            decoder.decode().map_err(|_| FAILED_PARSING_ERROR)?;
        // info_now!("maybe_properties: {:?}", &maybe_properties);

        let touch_required = maybe_properties
            .map(|properties| {
                info_now!("unraveling {:?}", &properties);
                properties.touch_required()
            })
            .unwrap_or(false);

        let mut counter = None;
        // kind::Hotp and valid u32 starting counter should be more tightly tied together on a
        // type level
        // if kind == oath::Kind::Hotp {
        if matches!(kind, oath::Kind::Hotp | oath::Kind::HotpReverse) {
            // when the counter is not specified or set to zero, ykman does not send it
            counter = Some(0);
            if let Ok(last) = TaggedSlice::decode(&mut decoder) {
                if last.tag() == (oath::Tag::InitialMovingFactor as u8).try_into().unwrap() {
                    let bytes = last.as_bytes();
                    if bytes.len() == 4 {
                        counter = Some(u32::from_be_bytes(bytes.try_into().unwrap()));
                    }
                }
            }
            debug_now!("counter set to {:?}", &counter);
        }

        let credential = Credential {
            label,
            kind,
            algorithm,
            digits,
            secret,
            touch_required,
            counter,
        };

        Ok(Register { credential })
    }
}

impl<'l, const C: usize> TryFrom<&'l iso7816::Command<C>> for Command<'l> {
    type Error = Status;
    /// The first layer of unraveling the iso7816::Command onion.
    ///
    /// The responsibility here is to check (cla, ins, p1, p2) are valid as defined
    /// in the "Command Syntax" boxes of NIST SP 800-73-4, and return early errors.
    ///
    /// The individual piv::Command TryFroms then further interpret these validated parameters.
    fn try_from(command: &'l iso7816::Command<C>) -> Result<Self, Self::Error> {
        let (class, instruction, p1, p2) = (
            command.class(),
            command.instruction(),
            command.p1,
            command.p2,
        );
        let data = command.data();

        if !class.secure_messaging().none() {
            return Err(Status::SecureMessagingNotSupported);
        }

        if class.channel() != Some(0) {
            return Err(Status::LogicalChannelNotSupported);
        }

        // TODO: should we check `command.expected() == 0`, where specified?

        if (0x00, iso7816::Instruction::Select, 0x04, 0x00)
            == (class.into_inner(), instruction, p1, p2)
        {
            Ok(Self::Select(Select::try_from(data)?))
        } else {
            let instruction_byte: u8 = instruction.into();
            let instruction: oath::Instruction = instruction_byte.try_into()?;
            Ok(match (class.into_inner(), instruction, p1, p2) {
                // also 0xa4
                (0x00, oath::Instruction::Calculate, 0x00, 0x01) => {
                    Self::Calculate(Calculate::try_from(data)?)
                }
                #[cfg(feature = "calculate-all")]
                (0x00, oath::Instruction::CalculateAll, 0x00, 0x01) => {
                    Self::CalculateAll(CalculateAll::try_from(data)?)
                }
                (0x00, oath::Instruction::Delete, 0x00, 0x00) => {
                    Self::Delete(Delete::try_from(data)?)
                }
                (0x00, oath::Instruction::List, 0x00, 0x00) => Self::ListCredentials,
                (0x00, oath::Instruction::Put, 0x00, 0x00) => {
                    Self::Register(Register::try_from(data)?)
                }
                (0x00, oath::Instruction::Reset, 0xde, 0xad) => Self::Reset,
                #[cfg(feature = "challenge-response-auth")]
                (0x00, oath::Instruction::SetCode, 0x00, 0x00) => {
                    // should check this is a TLV(SetPassword, b'')
                    if data.len() == 2 {
                        Self::ClearPassword
                    } else {
                        Self::SetPassword(SetPassword::try_from(data)?)
                    }
                }
                #[cfg(feature = "challenge-response-auth")]
                (0x00, oath::Instruction::Validate, 0x00, 0x00) => {
                    Self::Validate(Validate::try_from(data)?)
                }
                (0x00, oath::Instruction::VerifyCode, 0x00, 0x00) => {
                    Self::VerifyCode(VerifyCode::try_from(data)?)
                }
                (0x00, oath::Instruction::VerifyPIN, 0x00, 0x00) => {
                    Self::VerifyPin(VerifyPin::try_from(data)?)
                }
                (0x00, oath::Instruction::ChangePIN, 0x00, 0x00) => {
                    Self::ChangePin(ChangePin::try_from(data)?)
                }
                (0x00, oath::Instruction::SetPIN, 0x00, 0x00) => {
                    Self::SetPin(SetPin::try_from(data)?)
                }
                (0x00, oath::Instruction::SendRemaining, 0x00, 0x00) => Self::SendRemaining,
                _ => return Err(Status::InstructionNotSupportedOrInvalid),
            })
        }
    }
}

impl<'l, const C: usize> TryFrom<&'l Data<C>> for Select<'l> {
    type Error = Status;
    fn try_from(data: &'l Data<C>) -> Result<Self, Self::Error> {
        // info_now!("comparing {} against {}", hex_str!(data.as_slice()), hex_str!(crate::YUBICO_OATH_AID));
        Ok(match data.as_slice() {
            crate::YUBICO_OATH_AID => Self { aid: data },
            _ => return Err(Status::NotFound),
        })
    }
}

use extra_block::ExtraAreaBlock;
use head_block::HeadBlock;
use nom;
use nom::be_u32;
use util::{get_bit_at, split_u64, to_bool};
use vint::vint;

/// FileBlock
#[derive(PartialEq, Debug, Clone, Default)]
pub struct FileBlock {
    pub head: HeadBlock,
    pub flags: FileFlags,
    pub unpacked_size: u64,
    pub attributes: u64,
    pub mtime: u32,
    pub data_crc: u32,
    pub compression: Compression,
    pub creation_os: OsFlags,
    pub name_len: u64,
    pub name: String,
    pub extra: ExtraAreaBlock,
}

impl FileBlock {
    /// Parse an byte slice an returns a FileBlock.
    pub fn parse(inp: &[u8]) -> nom::IResult<&[u8], FileBlock> {
        // get the base header
        let (input, head) = HeadBlock::parse(inp)?;

        // check if the defined type is archive header
        if head.typ != ::head_block::Typ::File && head.typ != ::head_block::Typ::Service {
            return Err(nom::Err::Error(error_position!(inp, nom::ErrorKind::IsNot)));
        }

        // get the flags
        let (input, flags) = vint(input)?;
        let flags = FileFlags::from(flags);

        let (input, unpacked_size) = vint(input)?;
        let (mut input, attributes) = vint(input)?;

        // empty compression
        let compression = Compression {
            version: 0,
            solid: false,
            flag: CompressionFlags::Unknown,
            dictonary: 0,
        };

        // empty extra area block to be replaced
        let eab = ExtraAreaBlock::default();

        let mut file = FileBlock {
            head,
            flags,
            unpacked_size,
            attributes,
            mtime: 0,
            data_crc: 0,
            compression: compression,
            creation_os: OsFlags::UNKNOWN,
            name_len: 0,
            name: "".into(),
            extra: eab,
        };

        // check for time
        if file.flags.time {
            let (i, mtime) = be_u32(input)?;
            input = i;
            file.mtime = mtime;
        }

        // check for file crc data
        if file.flags.crc {
            let (i, crc) = be_u32(input)?;
            input = i;
            file.data_crc = crc;
        }

        let (input, compr) = Compression::parse(input)?;
        file.compression = compr;

        let (input, os) = vint(input)?;
        file.creation_os = os.into();

        let (input, nlen) = vint(input)?;
        file.name_len = nlen;

        let (mut input, n) = take_str!(input, file.name_len)?;
        file.name = n.into();

        // check for a extra area
        if file.head.flags.extra_area {
            let (i, extra) = take!(input, file.head.extra_area_size)?;
            input = i;
            file.extra = ExtraAreaBlock::parse(extra)?.1;
        }

        Ok((input, file))
    }
}

#[test]
fn test_archive() {
    use chrono::naive::NaiveDateTime;

    // test a success case
    let data = [
        0x8C, 0x0D, 0x88, 0xE2, 0x24, 0x02, 0x03, 0x0B, 0xC6, 0x10, 0x04, 0xC6, 0x10, 0x20, 0x93,
        0xF2, 0x9A, 0xCB, 0x80, 0x00, 0x00, 0x08, 0x74, 0x65, 0x78, 0x74, 0x2E, 0x74, 0x78, 0x74,
        0x0A, 0x03, 0x02, 0x78, 0x27, 0x3C, 0x1E, 0x7D, 0xF2, 0xD3, 0x01, 0x46, 0x61, 0x72, 0x20,
        0x66,
    ];

    let mut flags = ::head_block::Flags::new();
    flags.extra_area = true;
    flags.data_area = true;

    let compression = Compression {
        version: 0,
        solid: false,
        flag: CompressionFlags::Save,
        dictonary: 0,
    };

    let mut file_flag = FileFlags::default();
    file_flag.crc = true;

    let eab = ExtraAreaBlock {
        file_time: Some(::extra_block::FileTimeBlock {
            modification_time: Some(
                NaiveDateTime::parse_from_str("2018-05-23 10:02:11", "%Y-%m-%d %H:%M:%S").unwrap(),
            ),
            creation_time: None,
            access_time: None,
        }),
        file_encryption: None,
    };

    let mut arc = FileBlock {
        head: HeadBlock::new(2349697250, 36, ::head_block::Typ::File, flags),
        flags: file_flag,
        unpacked_size: 2118,
        attributes: 32,
        mtime: 0,
        data_crc: 2482150091,
        compression,
        creation_os: OsFlags::WINDOWS,
        name_len: 8,
        name: "text.txt".into(),
        extra: eab,
    };
    arc.head.extra_area_size = 11;
    arc.head.data_area_size = 2118;
    assert_eq!(FileBlock::parse(&data), Ok((&data[41..][..], arc)));
}
#[test]
fn test_archive_png() {
    use chrono::naive::NaiveDateTime;

    // test a success case
    let data = [
        0x3B, 0xC1, 0x34, 0x5E, 0x2B, 0x02, 0x03, 0x0B, 0xDB, 0x95, 0x83, 0x81, 0x00, 0x04, 0xDB,
        0x95, 0x83, 0x81, 0x00, 0x20, 0x94, 0xB1, 0xA4, 0x7A, 0x80, 0x00, 0x00, 0x09, 0x70, 0x68,
        0x6F, 0x74, 0x6F, 0x2E, 0x6A, 0x70, 0x67, 0x0A, 0x03, 0x02, 0x9D, 0xA1, 0xE3, 0x8C, 0xB5,
        0x44, 0xD2, 0x01, 0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x18, 0x45,
    ];

    let mut flags = ::head_block::Flags::new();
    flags.extra_area = true;
    flags.data_area = true;

    let compression = Compression {
        version: 0,
        solid: false,
        flag: CompressionFlags::Save,
        dictonary: 0,
    };

    let mut file_flag = FileFlags::default();
    file_flag.crc = true;

    let eab = ExtraAreaBlock {
        file_time: Some(::extra_block::FileTimeBlock {
            modification_time: Some(
                NaiveDateTime::parse_from_str("2016-11-22 11:42:49", "%Y-%m-%d %H:%M:%S").unwrap(),
            ),
            creation_time: None,
            access_time: None,
        }),
        file_encryption: None,
    };

    let mut arc = FileBlock {
        head: HeadBlock::new(1002517598, 43, ::head_block::Typ::File, flags),
        flags: file_flag,
        unpacked_size: 2149083,
        attributes: 32,
        mtime: 0,
        data_crc: 2494669946,
        compression,
        creation_os: OsFlags::WINDOWS,
        name_len: 9,
        name: "photo.jpg".into(),
        extra: eab,
    };
    arc.head.extra_area_size = 11;
    arc.head.data_area_size = 2149083;

    assert_eq!(FileBlock::parse(&data), Ok((&data[48..][..], arc)));
}

/// FileFlags
#[derive(PartialEq, Debug, Clone, Default)]
pub struct FileFlags {
    pub directory: bool,    // Directory file system object (file header only).
    pub time: bool,         // Time field in Unix format is present.
    pub crc: bool,          // CRC32 field is present
    pub unknown_size: bool, // Unpacked size is unknown.
}

impl From<u64> for FileFlags {
    fn from(i: u64) -> Self {
        let mut f = FileFlags::default();

        if get_bit_at(i, 0) {
            f.directory = true;
        }
        if get_bit_at(i, 1) {
            f.time = true;
        }
        if get_bit_at(i, 2) {
            f.crc = true;
        }
        if get_bit_at(i, 3) {
            f.unknown_size = true;
        }

        f
    }
}

/// OS flags
#[derive(PartialEq, Debug, Clone)]
pub enum OsFlags {
    WINDOWS,
    UNIX,
    UNKNOWN,
}

impl From<u64> for OsFlags {
    fn from(i: u64) -> Self {
        if i == 0 {
            return OsFlags::WINDOWS;
        }
        if i == 0 {
            return OsFlags::UNIX;
        }
        OsFlags::UNKNOWN
    }
}

impl Default for OsFlags {
    fn default() -> OsFlags {
        OsFlags::UNKNOWN
    }
}

/// Compression dataset
#[derive(PartialEq, Debug, Clone, Default)]
pub struct Compression {
    pub version: u8,
    pub solid: bool,
    pub flag: CompressionFlags,
    pub dictonary: u8,
}

impl Compression {
    /// get the compression info
    fn parse(inp: &[u8]) -> nom::IResult<&[u8], Compression> {
        // get the vint
        let (inp, raw) = vint(inp)?;

        // split it back to an u8 array
        let clean = &split_u64(raw)[..];

        // get the data from the compression
        // !!!!!! THIS IS PROBABLY WRONG !!!!!!
        let c = bits!(
            clean,
            do_parse!(
                dictonary: take_bits!(u8, 4)
                    >> flag: take_bits!(u8, 4)
                    >> solid: take_bits!(u8, 1)
                    >> version: take_bits!(u8, 6)
                    >> (Compression {
                        version,
                        solid: to_bool(solid),
                        flag: flag.into(),
                        dictonary
                    })
            )
        );

        // change the error to an inp error and not and bit matchign error
        let c =
            c.map_err(|_| nom::Err::Error(nom::Context::Code(inp, nom::ErrorKind::Custom(0))))?;

        // return the compression
        Ok((inp, c.1))
    }

    /// Return the dictonary in the right format
    pub fn get_directonary(&self) -> u32 {
        128 * (self.dictonary as f32).exp2() as u32
    }
}

#[test]
fn test_get_compression() {
    let c = Compression {
        version: 0,
        solid: false,
        flag: CompressionFlags::Save,
        dictonary: 0,
    };
    assert_eq!(Compression::parse(&[0x80, 0x00]), Ok((&[][..], c.clone())));
    assert_eq!(
        Compression::parse(&[0x80, 0x00, 0x00]),
        Ok((&[0x00][..], c))
    );
    assert!(Compression::parse(&[0x80]).is_err());
}
#[test]
fn test_get_directonary() {
    let mut data = Compression {
        version: 0,
        solid: false,
        flag: CompressionFlags::Save,
        dictonary: 0,
    };

    assert_eq!(data.get_directonary(), 128);
    data.dictonary = 1;
    assert_eq!(data.get_directonary(), 256);
    data.dictonary = 10;
    assert_eq!(data.get_directonary(), 131072);
    data.dictonary = 15;
    assert_eq!(data.get_directonary(), 4194304);
}

/// Compression Flags
#[derive(PartialEq, Debug, Clone)]
pub enum CompressionFlags {
    Save,
    Fastest,
    Fast,
    Normal,
    Good,
    Best,
    Unknown,
}

impl From<u8> for CompressionFlags {
    fn from(i: u8) -> Self {
        if i == 0 {
            return CompressionFlags::Save;
        }
        if i == 1 {
            return CompressionFlags::Fastest;
        }
        if i == 2 {
            return CompressionFlags::Fast;
        }
        if i == 3 {
            return CompressionFlags::Normal;
        }
        if i == 4 {
            return CompressionFlags::Good;
        }
        if i == 5 {
            return CompressionFlags::Best;
        }
        CompressionFlags::Unknown
    }
}

impl Default for CompressionFlags {
    fn default() -> CompressionFlags {
        CompressionFlags::Unknown
    }
}

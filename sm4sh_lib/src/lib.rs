use std::io::{Read, Seek, SeekFrom};

use binrw::{
    BinRead, BinReaderExt, BinResult, Endian, FilePtr32, NullString, VecArgs, file_ptr::FilePtrArgs,
};

pub mod gx2;
pub mod jtb;
pub mod mta;
pub mod nhb;
pub mod nsh;
pub mod nud;
pub mod nut;
pub mod omo;
pub mod pack;
pub mod sb;
pub mod vbn;

fn parse_opt_ptr32<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<Args>,
) -> BinResult<Option<T>>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: Read + Seek,
    Args: Clone,
{
    // Read a value pointed to by a nullable relative offset.
    let offset = u32::read_options(reader, endian, ())?;
    if offset > 0 {
        // Read a value pointed to by a relative offset.
        let saved_pos = reader.stream_position()?;

        reader.seek(SeekFrom::Start(offset as u64 + args.offset))?;

        let value = T::read_options(reader, endian, args.inner)?;
        reader.seek(SeekFrom::Start(saved_pos))?;

        Ok(Some(value))
    } else {
        Ok(None)
    }
}

fn parse_string_ptr32<R: Read + Seek>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<()>,
) -> BinResult<String> {
    // Read a value pointed to by a relative offset.
    let offset = u32::read_options(reader, endian, ())?;
    let saved_pos = reader.stream_position()?;

    reader.seek(SeekFrom::Start(offset as u64 + args.offset))?;

    let value = NullString::read_options(reader, endian, args.inner)?;
    reader.seek(SeekFrom::Start(saved_pos))?;

    Ok(value.to_string())
}

fn parse_string_opt_ptr32<R: Read + Seek>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<()>,
) -> BinResult<Option<String>> {
    let value: Option<NullString> = parse_opt_ptr32(reader, endian, args)?;
    Ok(value.map(|value| value.to_string()))
}

fn parse_ptr32_count<R, T, Args>(
    n: usize,
) -> impl Fn(&mut R, Endian, FilePtrArgs<Args>) -> BinResult<Vec<T>>
where
    R: Read + Seek,
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    Args: Clone,
{
    move |reader, endian, args| {
        FilePtr32::parse(
            reader,
            endian,
            FilePtrArgs {
                offset: args.offset,
                inner: binrw::VecArgs {
                    count: n,
                    inner: args.inner.clone(),
                },
            },
        )
    }
}

fn parse_count32_offset32<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<Args>,
) -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    let count = u32::read_options(reader, endian, ())?;
    let pos = reader.stream_position()?;
    let offset = u32::read_options(reader, endian, ())?;

    if offset == 0 && count != 0 {
        return Err(binrw::Error::AssertFail {
            pos,
            message: format!("unexpected null offset for count {count}"),
        });
    }

    parse_vec(reader, endian, args, offset as u64, count as usize)
}

fn parse_vec<T, R, Args>(
    reader: &mut R,
    endian: binrw::Endian,
    args: FilePtrArgs<Args>,
    offset: u64,
    count: usize,
) -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = Args> + 'static,
    R: std::io::Read + std::io::Seek,
    Args: Clone,
{
    let saved_pos = reader.stream_position()?;

    reader.seek(SeekFrom::Start(offset + args.offset))?;

    let values = <Vec<T>>::read_options(
        reader,
        endian,
        VecArgs {
            count,
            inner: args.inner.clone(),
        },
    )?;

    reader.seek(SeekFrom::Start(saved_pos))?;

    Ok(values)
}

macro_rules! file_write_full_impl {
    ($endian:path, $($type_name:path),*) => {
        $(
            impl $type_name {
                pub fn write<W: std::io::Write + std::io::Seek>(&self, writer: &mut W) -> xc3_write::Xc3Result<()> {
                    xc3_write::write_full(self, writer, 0, &mut 0, $endian, ()).map_err(Into::into)
                }

                /// Write to `path` using a buffered writer for better performance.
                pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> xc3_write::Xc3Result<()> {
                    let mut writer = std::io::BufWriter::new(std::fs::File::create(path)?);
                    self.write(&mut writer)
                }
            }
        )*
    };
}
pub(crate) use file_write_full_impl;

file_write_full_impl!(
    xc3_write::Endian::Big,
    nud::Nud,
    nut::Nut,
    omo::Omo,
    mta::Mta
);

file_write_full_impl!(xc3_write::Endian::Little, nhb::Nhb);

macro_rules! xc3_write_binwrite_impl {
    ($($ty:ty),*) => {
        $(
            impl Xc3Write for $ty {
                // This also enables write_full since () implements Xc3WriteOffsets.
                type Offsets<'a> = ();

                fn xc3_write<W: std::io::Write + std::io::Seek>(
                    &self,
                    writer: &mut W,
                    endian: xc3_write::Endian
                ) -> xc3_write::Xc3Result<Self::Offsets<'_>> {
                    let endian = match endian {
                        xc3_write::Endian::Little => binrw::Endian::Little,
                        xc3_write::Endian::Big => binrw::Endian::Big
                    };
                    self.write_options(writer, endian, ()).map_err(std::io::Error::other)?;
                    Ok(())
                }

                // TODO: Should this be specified manually?
                const ALIGNMENT: u64 = std::mem::align_of::<$ty>() as u64;
            }
        )*

    };
}
pub(crate) use xc3_write_binwrite_impl;

macro_rules! file_read_impl {
    ($endian:path, $($type_name:path),*) => {
        $(
            impl $type_name {
                pub fn read<R: std::io::Read + std::io::Seek>(reader: &mut R) -> binrw::BinResult<Self> {
                    reader.read_type($endian).map_err(Into::into)
                }

                /// Read from `path` using a fully buffered reader for performance.
                pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> binrw::BinResult<Self> {
                    let path = path.as_ref();
                    let mut reader = std::io::Cursor::new(std::fs::read(path)?);
                    reader.read_type($endian).map_err(Into::into)
                }

                /// Read from `bytes` using a fully buffered reader for performance.
                pub fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> binrw::BinResult<Self> {
                    Self::read(&mut std::io::Cursor::new(bytes))
                }
            }
        )*
    };
}
pub(crate) use file_read_impl;

// TODO: Detect endianness by trying both for u32 magic?
file_read_impl!(
    Endian::Big,
    nud::Nud,
    nut::Nut,
    nsh::Nsh,
    vbn::Vbn,
    pack::Pack,
    omo::Omo,
    mta::Mta,
    jtb::Jtb
);

file_read_impl!(Endian::Little, nhb::Nhb, sb::Sb);

macro_rules! file_write_impl {
    ($endian:path, $($type_name:path),*) => {
        $(
            impl $type_name {
                pub fn write<W: std::io::Write + std::io::Seek>(&self, writer: &mut W) -> binrw::BinResult<()> {
                    <Self as binrw::BinWrite>::write_options(&self, writer, $endian, ())
                }

                /// Write to `path` using a buffered writer for better performance.
                pub fn save<P: AsRef<std::path::Path>>(&self, path: P) ->binrw::BinResult<()> {
                    let mut writer = std::io::BufWriter::new(std::fs::File::create(path)?);
                    self.write(&mut writer)
                }
            }
        )*
    };
}

file_write_impl!(binrw::Endian::Big, nsh::Nsh, vbn::Vbn);

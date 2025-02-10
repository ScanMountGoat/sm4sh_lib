use std::io::{Read, Seek, SeekFrom};

use binrw::{file_ptr::FilePtrArgs, BinRead, BinResult, NullString};

pub mod nud;
pub mod nut;
pub mod vbn;
// TODO: Add sb?

// TODO: Create a separate NudModel struct without creating a separate project?
// TODO: possible to preserve binary file types 1:1?

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

// TODO: macro to implement from_file and save methods

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
file_write_full_impl!(xc3_write::Endian::Big, nud::Nud);

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

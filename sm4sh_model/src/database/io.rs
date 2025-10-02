use std::{collections::BTreeMap, io::Cursor, path::Path};

use binrw::{BinRead, BinReaderExt, BinResult, BinWrite, BinWriterExt, NullString, binrw};
use log::error;
use ordered_float::OrderedFloat;
use smol_str::{SmolStr, ToSmolStr};
use varint_rs::{VarintReader, VarintWriter};

use super::{Attribute, Operation, OutputExpr, Parameter, ShaderProgram, Texture, Value};

// Faster than the default hash implementation.
type IndexSet<T> = indexmap::IndexSet<T, ahash::RandomState>;
type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;

// Create a separate format optimized for storing on disk.
#[binrw]
#[derive(Debug, PartialEq, Clone, Default)]
#[brw(magic(b"SHDB"))]
pub struct ShaderDatabaseIndexed {
    // File version numbers should be updated with each release.
    // This improves the error when parsing an incompatible version.
    #[br(assert(version == 1))]
    #[bw(calc = 1)]
    version: u32,

    // Use an ordered map for consistent ordering.
    #[br(parse_with = parse_map32)]
    #[bw(write_with = write_map32)]
    programs: BTreeMap<u32, ShaderProgramIndexed>,

    #[br(parse_with = parse_set)]
    #[bw(write_with = write_set)]
    values: IndexSet<ValueIndexed>,

    #[br(parse_with = parse_set)]
    #[bw(write_with = write_set)]
    parameters: IndexSet<ParameterIndexed>,

    #[br(parse_with = parse_set)]
    #[bw(write_with = write_set)]
    output_exprs: IndexSet<OutputExprIndexed>,

    // Storing multiple string lists enables 8-bit instead of 16-bit indices.
    #[br(parse_with = parse_strings)]
    #[bw(write_with = write_strings)]
    attribute_names: IndexSet<SmolStr>,

    #[br(parse_with = parse_strings)]
    #[bw(write_with = write_strings)]
    buffer_names: IndexSet<SmolStr>,

    #[br(parse_with = parse_strings)]
    #[bw(write_with = write_strings)]
    buffer_field_names: IndexSet<SmolStr>,

    #[br(parse_with = parse_strings)]
    #[bw(write_with = write_strings)]
    texture_names: IndexSet<SmolStr>,

    #[br(parse_with = parse_strings)]
    #[bw(write_with = write_strings)]
    outputs: IndexSet<SmolStr>,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
struct ShaderProgramIndexed {
    // There are very few unique dependencies across all shaders in a game dump.
    // Normalize the data to greatly reduce the size file size.
    #[br(parse_with = parse_vec)]
    #[bw(write_with = write_vec)]
    output_dependencies: Vec<(VarInt, VarInt)>,

    #[br(parse_with = parse_vec)]
    #[bw(write_with = write_vec)]
    attributes: Vec<VarInt>,

    #[br(parse_with = parse_vec)]
    #[bw(write_with = write_vec)]
    samplers: Vec<VarInt>,

    #[br(parse_with = parse_vec)]
    #[bw(write_with = write_vec)]
    parameters: Vec<VarInt>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, BinRead, BinWrite)]
enum OutputExprIndexed {
    #[brw(magic(0u8))]
    Value(VarInt),

    #[brw(magic(1u8))]
    Func {
        // TODO: Avoid unwrap
        #[br(map(|x: u8| Operation::from_repr(x as usize).unwrap()))]
        #[bw(map(|x| *x as u8))]
        op: Operation,

        #[br(parse_with = parse_vec)]
        #[bw(write_with = write_vec)]
        args: Vec<VarInt>,
    },
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, BinRead, BinWrite)]
#[brw(repr(u8))]
pub enum Channel {
    None = 0,
    X = 1,
    Y = 2,
    Z = 3,
    W = 4,
}

impl From<Channel> for Option<char> {
    fn from(value: Channel) -> Self {
        match value {
            Channel::None => None,
            Channel::X => Some('x'),
            Channel::Y => Some('y'),
            Channel::Z => Some('z'),
            Channel::W => Some('w'),
        }
    }
}

impl From<Option<char>> for Channel {
    fn from(value: Option<char>) -> Self {
        match value {
            Some('x') => Self::X,
            Some('y') => Self::Y,
            Some('z') => Self::Z,
            Some('w') => Self::W,
            None => Self::None,
            _ => {
                error!("unable to convert {value:?} to channel");
                Self::None
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, BinRead, BinWrite)]
enum ValueIndexed {
    #[brw(magic(0u8))]
    Float(
        #[br(map(|f: f32| f.into()))]
        #[bw(map(|f| f.0))]
        OrderedFloat<f32>,
    ),

    #[brw(magic(1u8))]
    Parameter(VarInt),

    #[brw(magic(2u8))]
    Texture(TextureIndexed),

    #[brw(magic(3u8))]
    Attribute(AttributeIndexed),

    #[brw(magic(4u8))]
    Int(i32),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, BinRead, BinWrite)]
struct ParameterIndexed {
    name: VarInt,
    field: VarInt,
    index: OptVarInt,
    channel: Channel,
}

#[binrw]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct TextureIndexed {
    name: VarInt,
    channel: Channel,

    #[br(parse_with = parse_vec)]
    #[bw(write_with = write_vec)]
    texcoords: Vec<VarInt>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, BinRead, BinWrite)]
struct AttributeIndexed {
    name: VarInt,
    channel: Channel,
}

impl ShaderDatabaseIndexed {
    pub fn from_file<P: AsRef<Path>>(path: P) -> BinResult<Self> {
        let mut reader = Cursor::new(std::fs::read(path)?);
        reader.read_le()
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> BinResult<()> {
        let mut writer = Cursor::new(Vec::new());
        writer.write_le(self)?;
        std::fs::write(path, writer.into_inner())?;
        Ok(())
    }

    pub fn get_shader(&self, shader_id: u32) -> Option<ShaderProgram> {
        self.programs
            .get(&shader_id)
            .map(|p| self.program_from_indexed(p))
    }

    pub fn from_programs(programs: BTreeMap<u32, ShaderProgram>) -> Self {
        let mut database = Self::default();

        for (id, p) in programs.into_iter() {
            let program = database.program_indexed(p);
            database.programs.insert(id, program);
        }

        database
    }

    fn program_indexed(&mut self, p: ShaderProgram) -> ShaderProgramIndexed {
        // Remap exprs indexed for this program to exprs indexed for all programs.
        let mut expr_indices = IndexMap::default();

        ShaderProgramIndexed {
            output_dependencies: p
                .output_dependencies
                .into_iter()
                .map(|(output, value)| {
                    let output_index = add_string(&mut self.outputs, output);
                    (
                        output_index,
                        self.add_output_expr(&p.exprs[value], &p.exprs, &mut expr_indices),
                    )
                })
                .collect(),
            attributes: p
                .attributes
                .into_iter()
                .map(|s| add_string(&mut self.attribute_names, s))
                .collect(),
            samplers: p
                .samplers
                .into_iter()
                .map(|s| add_string(&mut self.texture_names, s))
                .collect(),
            parameters: p
                .parameters
                .into_iter()
                .map(|s| add_string(&mut self.buffer_field_names, s))
                .collect(),
        }
    }

    fn add_output_expr<'a>(
        &mut self,
        value: &'a OutputExpr<Operation>,
        exprs: &'a [OutputExpr<Operation>],
        expr_indices: &mut IndexMap<&'a OutputExpr<Operation>, VarInt>,
    ) -> VarInt {
        match expr_indices.get(value) {
            Some(i) => *i,
            None => {
                // Insert values that this value depends on first.
                let v = match &value {
                    OutputExpr::Value(d) => {
                        OutputExprIndexed::Value(self.add_value(d.clone(), exprs, expr_indices))
                    }
                    OutputExpr::Func { op, args } => OutputExprIndexed::Func {
                        op: *op,
                        args: args
                            .iter()
                            .map(|a| self.add_output_expr(&exprs[*a], exprs, expr_indices))
                            .collect(),
                    },
                };

                let (index, _) = self.output_exprs.insert_full(v);
                expr_indices.insert(value, VarInt(index));

                VarInt(index)
            }
        }
    }

    fn add_value<'a>(
        &mut self,
        d: Value,
        exprs: &'a [OutputExpr<Operation>],
        expr_indices: &mut IndexMap<&'a OutputExpr<Operation>, VarInt>,
    ) -> VarInt {
        let value = self.value_indexed(d, exprs, expr_indices);
        let (index, _) = self.values.insert_full(value);

        VarInt(index)
    }

    fn add_parameter(&mut self, b: Parameter) -> VarInt {
        let value = self.parameter_indexed(b);
        let (index, _) = self.parameters.insert_full(value);

        VarInt(index)
    }

    fn program_from_indexed(&self, p: &ShaderProgramIndexed) -> ShaderProgram {
        // Remap exprs indexed for all programs to exprs indexed for this program.
        let mut exprs = IndexSet::default();
        let mut expr_to_index = IndexMap::default();

        ShaderProgram {
            output_dependencies: p
                .output_dependencies
                .iter()
                .map(|(output, value)| {
                    (
                        self.outputs[output.0].clone(),
                        self.output_expr_from_indexed(value.0, &mut exprs, &mut expr_to_index),
                    )
                })
                .collect(),
            exprs: exprs.into_iter().collect(),
            attributes: p
                .attributes
                .iter()
                .map(|s| self.attribute_names[s.0].clone())
                .collect(),
            samplers: p
                .samplers
                .iter()
                .map(|s| self.texture_names[s.0].clone())
                .collect(),
            parameters: p
                .parameters
                .iter()
                .map(|s| self.buffer_field_names[s.0].clone())
                .collect(),
        }
    }

    fn output_expr_from_indexed(
        &self,
        value: usize,
        exprs: &mut IndexSet<OutputExpr<Operation>>,
        expr_to_index: &mut IndexMap<usize, usize>,
    ) -> usize {
        match expr_to_index.get(&value) {
            Some(i) => *i,
            None => {
                let expr = match &self.output_exprs[value] {
                    OutputExprIndexed::Value(d) => OutputExpr::Value(self.value_from_indexed(
                        &self.values[d.0],
                        exprs,
                        expr_to_index,
                    )),
                    OutputExprIndexed::Func { op, args } => OutputExpr::Func {
                        op: *op,
                        args: args
                            .iter()
                            .map(|a| self.output_expr_from_indexed(a.0, exprs, expr_to_index))
                            .collect(),
                    },
                };
                let index = exprs.insert_full(expr).0;
                expr_to_index.insert(value, index);
                index
            }
        }
    }

    fn value_from_indexed(
        &self,
        v: &ValueIndexed,
        exprs: &mut IndexSet<OutputExpr<Operation>>,
        expr_to_index: &mut IndexMap<usize, usize>,
    ) -> Value {
        match v {
            ValueIndexed::Int(i) => Value::Int(*i),
            ValueIndexed::Float(f) => Value::Float(*f),
            ValueIndexed::Parameter(p) => {
                Value::Parameter(self.parameter_from_indexed(&self.parameters[p.0]))
            }
            ValueIndexed::Texture(t) => Value::Texture(Texture {
                name: self.texture_names[t.name.0].clone(),
                channel: t.channel.into(),
                texcoords: t
                    .texcoords
                    .iter()
                    .map(|coord| self.output_expr_from_indexed(coord.0, exprs, expr_to_index))
                    .collect(),
            }),
            ValueIndexed::Attribute(a) => Value::Attribute(Attribute {
                name: self.attribute_names[a.name.0].clone(),
                channel: a.channel.into(),
            }),
        }
    }

    fn value_indexed<'a>(
        &mut self,
        v: Value,
        exprs: &'a [OutputExpr<Operation>],
        expr_indices: &mut IndexMap<&'a OutputExpr<Operation>, VarInt>,
    ) -> ValueIndexed {
        match v {
            Value::Int(i) => ValueIndexed::Int(i),
            Value::Float(c) => ValueIndexed::Float(c),
            Value::Parameter(p) => ValueIndexed::Parameter(self.add_parameter(p)),
            Value::Texture(t) => ValueIndexed::Texture(TextureIndexed {
                name: add_string(&mut self.texture_names, t.name),
                channel: t.channel.into(),
                texcoords: t
                    .texcoords
                    .iter()
                    .map(|t| self.add_output_expr(&exprs[*t], exprs, expr_indices))
                    .collect(),
            }),
            Value::Attribute(a) => ValueIndexed::Attribute(AttributeIndexed {
                name: add_string(&mut self.attribute_names, a.name),
                channel: a.channel.into(),
            }),
        }
    }

    fn parameter_from_indexed(&self, b: &ParameterIndexed) -> Parameter {
        Parameter {
            name: self.buffer_names[b.name.0].clone(),
            field: self.buffer_field_names[b.field.0].clone(),
            index: b.index.0,
            channel: b.channel.into(),
        }
    }

    fn parameter_indexed(&mut self, b: Parameter) -> ParameterIndexed {
        ParameterIndexed {
            name: add_string(&mut self.buffer_names, b.name),
            field: add_string(&mut self.buffer_field_names, b.field),
            index: OptVarInt(b.index),
            channel: b.channel.into(),
        }
    }
}

fn add_string(strings: &mut IndexSet<SmolStr>, str: SmolStr) -> VarInt {
    VarInt(strings.insert_full(str).0)
}

// Variable length ints are slightly slower to parse but take up much less space.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
struct VarInt(usize);

impl BinRead for VarInt {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<Self> {
        reader.read_usize_varint().map(Self).map_err(Into::into)
    }
}

impl BinWrite for VarInt {
    type Args<'a> = ();

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<()> {
        writer.write_usize_varint(self.0).map_err(Into::into)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
struct OptVarInt(Option<usize>);

impl BinRead for OptVarInt {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<Self> {
        let value = reader.read_usize_varint()?;
        let index = value.checked_sub(1);
        Ok(Self(index))
    }
}

impl BinWrite for OptVarInt {
    type Args<'a> = ();

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<()> {
        match self.0 {
            Some(index) => writer.write_usize_varint(index + 1)?,
            None => writer.write_usize_varint(0)?,
        }
        Ok(())
    }
}

#[binrw::parser(reader, endian)]
fn parse_vec<T>() -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = ()> + 'static,
{
    let count = VarInt::read_options(reader, endian, ())?.0;
    <Vec<T>>::read_options(reader, endian, binrw::VecArgs { count, inner: () })
}

#[binrw::writer(writer, endian)]
fn write_vec<T>(value: &Vec<T>) -> BinResult<()>
where
    for<'a> T: BinWrite<Args<'a> = ()> + 'static,
{
    VarInt(value.len()).write_options(writer, endian, ())?;
    value.write_options(writer, endian, ())?;
    Ok(())
}

#[binrw::parser(reader, endian)]
fn parse_set<T>() -> BinResult<IndexSet<T>>
where
    T: std::hash::Hash + Eq,
    for<'a> T: BinRead<Args<'a> = ()> + 'static,
{
    let count = VarInt::read_options(reader, endian, ())?.0;
    let mut values = IndexSet::default();
    for _ in 0..count {
        let value = T::read_options(reader, endian, ())?;
        values.insert(value);
    }
    Ok(values)
}

#[binrw::writer(writer, endian)]
fn write_set<T>(values: &IndexSet<T>) -> BinResult<()>
where
    for<'a> T: BinWrite<Args<'a> = ()> + 'static,
{
    VarInt(values.len()).write_options(writer, endian, ())?;
    for v in values {
        v.write_options(writer, endian, ())?;
    }
    Ok(())
}

#[binrw::parser(reader, endian)]
fn parse_strings() -> BinResult<IndexSet<SmolStr>> {
    let count = VarInt::read_options(reader, endian, ())?.0;
    let mut values = IndexSet::default();
    for _ in 0..count {
        let s = NullString::read_options(reader, endian, ())?;
        values.insert(s.to_smolstr());
    }
    Ok(values)
}

#[binrw::writer(writer, endian)]
fn write_strings(value: &IndexSet<SmolStr>) -> BinResult<()> {
    VarInt(value.len()).write_options(writer, endian, ())?;
    for v in value {
        NullString::from(v.as_str()).write_options(writer, endian, ())?;
    }
    Ok(())
}

fn parse_map32<T, R>(
    reader: &mut R,
    endian: binrw::Endian,
    _args: (),
) -> BinResult<BTreeMap<u32, T>>
where
    for<'a> T: BinRead<Args<'a> = ()> + 'static,
    R: std::io::Read + std::io::Seek,
{
    let count = u32::read_options(reader, endian, ())?;

    let mut map = BTreeMap::new();
    for _ in 0..count {
        let (key, value) = <(u32, T)>::read_options(reader, endian, ())?;
        map.insert(key, value);
    }
    Ok(map)
}

#[binrw::writer(writer, endian)]
fn write_map32<T>(map: &BTreeMap<u32, T>) -> BinResult<()>
where
    for<'a> T: BinWrite<Args<'a> = ()> + 'static,
{
    (u32::try_from(map.len()).unwrap()).write_options(writer, endian, ())?;
    for (k, v) in map.iter() {
        k.write_options(writer, endian, ())?;
        v.write_options(writer, endian, ())?;
    }
    Ok(())
}

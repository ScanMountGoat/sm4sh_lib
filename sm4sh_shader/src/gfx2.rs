use nom::{
    IResult, Parser,
    bytes::{complete::take_while, tag},
    character::complete::{digit1, multispace1},
    combinator::{iterator, map, map_res},
    error::Error,
    sequence::terminated,
};

// TODO: can this be derived using serde?
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PixelShaderHeader {
    pub size: usize,
    pub mode: String,
    pub uniform_blocks: Vec<UniformBlock>,
    pub uniform_vars: Vec<UniformVar>,
    pub initial_value_count: usize,
    pub loop_var_count: usize,
    pub sampler_vars: Vec<SamplerVar>,
    // TODO: misc flags?
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UniformBlock {
    pub name: String,
    pub offset: usize,
    pub size: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UniformVar {
    pub name: String,
    pub ty: String,
    pub count: usize,
    pub offset: usize,
    pub block: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SamplerVar {
    pub name: String,
    pub ty: String,
    pub location: usize,
}

fn digit1_as_usize(i: &str) -> IResult<&str, usize> {
    map_res(digit1, |s: &str| s.parse::<usize>()).parse(i)
}

fn text(i: &str) -> IResult<&str, String> {
    map(take_while(|c: char| !c.is_ascii_whitespace()), |s: &str| {
        s.to_string()
    })
    .parse(i)
}

fn sp(i: &str) -> IResult<&str, &str> {
    multispace1(i)
}

fn int_field(name: &str) -> impl Fn(&str) -> IResult<&str, usize> {
    move |i| {
        let (i, (_, _, _, _, value)) = (tag(name), sp, tag("="), sp, digit1_as_usize).parse(i)?;
        Ok((i, value))
    }
}

fn string_field(name: &str) -> impl Fn(&str) -> IResult<&str, String> {
    move |i| {
        let (i, (_, _, _, _, value)) = (tag(name), sp, tag("="), sp, text).parse(i)?;
        Ok((i, value))
    }
}

fn array_name(name: &str) -> impl Fn(&str) -> IResult<&str, &str> {
    move |i| {
        let (i, (tag, _, _, _)) = (tag(name), tag("["), digit1, tag("]")).parse(i)?;
        Ok((i, tag))
    }
}

fn array<T, F>(f: F, count: usize) -> impl Fn(&str) -> IResult<&str, Vec<T>>
where
    F: Clone,
    for<'a> F: Parser<&'a str, Output = T, Error = Error<&'a str>>,
{
    move |i| {
        let mut it = iterator(i, terminated(f.clone(), sp));
        let values = it.by_ref().take(count).collect();
        let i = it.finish()?.0;
        Ok((i, values))
    }
}

fn uniform_block(i: &str) -> IResult<&str, UniformBlock> {
    let (i, (_, _, name, _, offset, _, size)) = (
        array_name("uniformBlocks"),
        sp,
        string_field("name"),
        sp,
        int_field("offset"),
        sp,
        int_field("size"),
    )
        .parse(i)?;

    Ok((i, UniformBlock { name, offset, size }))
}

fn uniform_var(i: &str) -> IResult<&str, UniformVar> {
    let (i, (_, _, name, _, ty, _, count, _, offset, _, block)) = (
        array_name("uniformVars"),
        sp,
        string_field("name"),
        sp,
        string_field("type"),
        sp,
        int_field("count"),
        sp,
        int_field("offset"),
        sp,
        int_field("block"),
    )
        .parse(i)?;

    Ok((
        i,
        UniformVar {
            name,
            ty,
            count,
            offset,
            block,
        },
    ))
}

fn sampler_var(i: &str) -> IResult<&str, SamplerVar> {
    let (i, (_, _, name, _, ty, _, location)) = (
        array_name("samplerVars"),
        sp,
        string_field("name"),
        sp,
        string_field("type"),
        sp,
        int_field("location"),
    )
        .parse(i)?;

    Ok((i, SamplerVar { name, ty, location }))
}

pub fn pixel_shader_header(i: &str) -> IResult<&str, PixelShaderHeader> {
    let (i, (_, _, size, _, mode, _)) = (
        tag("PixelShaderHeader"),
        sp,
        int_field("size"),
        sp,
        string_field("mode"),
        sp,
    )
        .parse(i)?;

    let (i, (uniform_block_count, _)) = (int_field("uniformBlockCount"), sp).parse(i)?;
    let (i, uniform_blocks) = array(uniform_block, uniform_block_count)(i)?;

    let (i, (uniform_var_count, _)) = (int_field("uniformVarCount"), sp).parse(i)?;
    let (i, uniform_vars) = array(uniform_var, uniform_var_count)(i)?;

    let (i, (initial_value_count, _, loop_var_count, _)) = (
        int_field("initialValueCount"),
        sp,
        int_field("loopVarCount"),
        sp,
    )
        .parse(i)?;

    let (i, (sampler_var_count, _)) = (int_field("samplerVarCount"), sp).parse(i)?;
    let (i, sampler_vars) = array(sampler_var, sampler_var_count)(i)?;

    Ok((
        i,
        PixelShaderHeader {
            size,
            mode,
            uniform_blocks,
            uniform_vars,
            initial_value_count,
            loop_var_count,
            sampler_vars,
        },
    ))
}

// TODO: Vertex shader header

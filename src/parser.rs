//! A .gox parser.
//!
//! Based on the spec at https://github.com/guillaumechereau/goxel/blob/master/src/formats/gox.c#L27
use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::{map, verify},
    multi::{fold_many1, length_count, length_data, many0},
    number::complete::{le_i32, le_u32},
    sequence::{preceded, terminated, tuple},
    IResult,
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Goxel {
    version: i32,
    chunks: Vec<Chunk>,
}

#[derive(Debug)]
pub struct Block {
    index: i32,
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Debug)]
pub enum Chunk {
    Img {
        dict: HashMap<String, Vec<u8>>,
    },
    Prev {
        data: Vec<u8>,
    },
    Bl16 {
        data: Vec<u8>,
    },
    Layr {
        blocks: Vec<Block>,
        dict: HashMap<String, Vec<u8>>,
    },
    Camr {
        dict: HashMap<String, Vec<u8>>,
    },
    Ligh {
        dict: HashMap<String, Vec<u8>>,
    },
}

fn entry(input: &[u8]) -> IResult<&[u8], (String, Vec<u8>)> {
    map(
        tuple((
            length_data(verify(le_u32, |&n| n != 0)),
            length_data(le_u32),
        )),
        |(key, value)| (String::from_utf8_lossy(key).to_string(), value.to_vec()),
    )(input)
}

fn dict(input: &[u8]) -> IResult<&[u8], HashMap<String, Vec<u8>>> {
    fold_many1(entry, HashMap::new, |mut map, (key, value)| {
        map.insert(key, value);
        map
    })(input)
}

fn chunk_common<'a, F: 'a>(
    name: &'a str,
    parser: F,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], Chunk>
where
    F: FnMut(&'a [u8]) -> IResult<&'a [u8], Chunk>,
{
    terminated(
        preceded(tag(name), parser), // TODO: Collect length buffer so callers don't have to, map_parser maybe?
        le_u32,                      // TODO: Handle CRC?
    )
}

fn img(input: &[u8]) -> IResult<&[u8], Chunk> {
    chunk_common(
        "IMG ",
        map(preceded(le_u32, dict), |dict| Chunk::Img { dict }),
    )(input)
}

fn prev(input: &[u8]) -> IResult<&[u8], Chunk> {
    chunk_common(
        "PREV",
        map(length_data(le_u32), |data: &[u8]| Chunk::Prev {
            data: data.to_vec(),
        }),
    )(input)
}

fn bl16(input: &[u8]) -> IResult<&[u8], Chunk> {
    chunk_common(
        "BL16",
        map(length_data(le_u32), |data: &[u8]| Chunk::Bl16 {
            data: data.to_vec(),
        }),
    )(input)
}

fn block(input: &[u8]) -> IResult<&[u8], Block> {
    map(
        tuple((le_i32, le_i32, le_i32, le_i32, le_i32)),
        |(index, x, y, z, _)| Block { index, x, y, z },
    )(input)
}

fn layr(input: &[u8]) -> IResult<&[u8], Chunk> {
    chunk_common(
        "LAYR",
        map(
            preceded(le_u32, tuple((length_count(le_u32, block), dict))),
            |(blocks, dict)| Chunk::Layr { blocks, dict },
        ),
    )(input)
}

fn camr(input: &[u8]) -> IResult<&[u8], Chunk> {
    chunk_common(
        "CAMR",
        map(preceded(le_u32, dict), |dict| Chunk::Camr { dict }),
    )(input)
}

fn ligh(input: &[u8]) -> IResult<&[u8], Chunk> {
    chunk_common(
        "LIGH",
        map(preceded(le_u32, dict), |dict| Chunk::Ligh { dict }),
    )(input)
}

fn chunk(input: &[u8]) -> IResult<&[u8], Chunk> {
    alt((img, prev, bl16, layr, camr, ligh))(input)
}

pub fn parse(input: &[u8]) -> IResult<&[u8], Goxel> {
    map(
        preceded(tag("GOX "), tuple((le_i32, many0(chunk)))),
        |(version, chunks)| Goxel { version, chunks },
    )(input)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn img_should_parse() {
        let input: &[u8] = &[
            // Chunk Header
            b'I', b'M', b'G', b' ', // Type
            0x9, 0x0, 0x0, 0x0, // Size
            // Dict
            0x1, 0x0, 0x0, 0x0,  // Key Length
            0x41, // Key Data
            0x0, 0x0, 0x0, 0x0, // End Dict
            0x0, 0x0, 0x0, 0x0, // CRC
        ];

        let res = img(input).expect("Couldn't get img chunk");
    }
}

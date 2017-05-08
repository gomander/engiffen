extern crate image;
extern crate gif;
extern crate color_quant;

use std::io;
use std::{error, fmt};
use std::borrow::Cow;
use std::collections::HashMap;
use image::{GenericImage, DynamicImage};
use gif::{Frame, Encoder, Repeat, SetParameter};
use color_quant::NeuQuant;

// use std::time::{Instant};

// fn ms(duration: Instant) -> u64 {
//     let duration = duration.elapsed();
//     duration.as_secs() * 1000 + duration.subsec_nanos() as u64 / 1000000
// }


#[derive(Debug)]
pub enum Error {
    Write(io::Error),
    NoImages,
    Mismatch((u32, u32), (u32, u32)),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Write(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Write(_) => write!(f, "Couldn't write to output"),
            Error::NoImages => write!(f, "No frames sent for engiffening"),
            Error::Mismatch(_, _) => write!(f, "Frames don't have the same dimensions"),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Write(ref err) => err.description(),
            Error::NoImages => "No frames sent for engiffening",
            Error::Mismatch(_, _) => "Frames don't have the same dimensions",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Write(ref err) => Some(err),
            _ => None,
        }
    }
}

pub fn engiffen<W: io::Write>(imgs: &[DynamicImage], fps: usize, mut out: &mut W) -> Result<(), Error> {
    let gif_descriptor = palettize(&imgs)?;
    let delay = (1000 / fps) as u16;

    let width = gif_descriptor.width;
    let height = gif_descriptor.height;
    let mut encoder = Encoder::new(&mut out, width, height, &gif_descriptor.palette)?;
    encoder.set(Repeat::Infinite)?;
    for img in gif_descriptor.images {
        let mut frame = Frame::default();
        frame.delay = delay / 10;
        frame.width = width;
        frame.height = height;
        frame.buffer = Cow::Borrowed(&*img);
        frame.transparent = gif_descriptor.transparency;
        encoder.write_frame(&frame)?;
    }
    Ok(())
}

struct GifDescriptor {
    palette: Vec<u8>,
    transparency: Option<u8>,
    width: u16,
    height: u16,
    images: Vec<Vec<u8>>,
}

fn palettize(imgs: &[DynamicImage]) -> Result<GifDescriptor, Error> {
    if imgs.is_empty() {
        return Err(Error::NoImages);
    }
    // let time_check_dimensions = Instant::now();
    let (width, height) = {
        let ref first = imgs[0];
        let first_dimensions = (first.width(), first.height());
        for img in imgs.iter() {
            let other_dimensions = (img.width(), img.height());
            if first_dimensions != other_dimensions {
                return Err(Error::Mismatch(first_dimensions, other_dimensions));
            }
        }
        first_dimensions
    };
    // println!("Checked image dimensions in {} ms.", ms(time_check_dimensions));
    // let time_push = Instant::now();
    let mut colors: Vec<u8> = Vec::with_capacity(width as usize * height as usize * imgs.len());
    for img in imgs {
        for (_, _, px) in img.pixels() {
            if px.data[3] == 0 {
                colors.push(0);
                colors.push(0);
                colors.push(0);
                colors.push(0);
            } else {
                colors.push(px.data[0]);
                colors.push(px.data[1]);
                colors.push(px.data[2]);
                colors.push(255);
            }
        }
    }
    // println!("Pushed all frame pixels in {} ms.", ms(time_push));

    // let time_quant = Instant::now();
    let quant = NeuQuant::new(10, 256, &colors);
    // println!("Computed palette in {} ms.", ms(time_quant));
    // let time_map = Instant::now();
    let mut transparency = None;
    let mut cache: HashMap<[u8; 4], u8> = HashMap::new();
    let palettized_imgs: Vec<Vec<u8>> = imgs.iter().map(|img| {
        img.pixels().map(|(_, _, px)| {
            *cache.entry(px.data).or_insert_with(|| {
                let idx = quant.index_of(&px.data) as u8;
                if px.data[3] == 0 { transparency = Some(idx); }
                idx
            })
        }).collect()
    }).collect();
    // println!("Mapped pixels to palette in {} ms.", ms(time_map));

    Ok(GifDescriptor {
        palette: quant.color_map_rgb(),
        transparency: transparency,
        width: width as u16,
        height: height as u16,
        images: palettized_imgs,
    })
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::{engiffen, Error};
    use std::fs::{read_dir, File};
    use std::io::Cursor;
    use image;

    #[test]
    fn test_error_on_size_mismatch() {
        let imgs: Vec<_> = read_dir("tests/mismatched_size").unwrap()
        .map(|e| e.unwrap().path())
        .map(|path| image::open(&path).unwrap())
        .collect();

        let mut out_file = Cursor::new(vec![]);

        let res = engiffen(&imgs, 30, &mut out_file);

        assert!(res.is_err());
        match res {
            Err(Error::Mismatch(one, another)) => {
                assert_eq!((one, another), ((100, 100), (50, 50)));
            },
            _ => unreachable!(),
        }
    }

    #[test] #[ignore]
    fn test_compress_palette() {
        // This takes a while to run when not in --release
        let imgs: Vec<_> = read_dir("tests/ball").unwrap()
            .map(|e| e.unwrap().path())
            .map(|path| image::open(&path).unwrap())
            .collect();

        let mut out = File::create("tests/ball.gif").unwrap();
        engiffen(&imgs, 10, &mut out);
    }

    #[test] #[ignore]
    fn test_simple_paletted_gif() {
        let imgs: Vec<_> = read_dir("tests/shrug").unwrap()
            .map(|e| e.unwrap().path())
            .map(|path| image::open(&path).unwrap())
            .collect();

        let mut out = File::create("tests/shrug.gif").unwrap();
        engiffen(&imgs, 30, &mut out);
    }
}

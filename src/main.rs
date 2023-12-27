use std::{
    collections::{HashSet, HashMap},
    fs,
    io::{Read, Write, self},
    path::{PathBuf, Display},
};

use anyhow::bail;
use clap::{Parser, Subcommand};
use image::{GenericImageView, Rgba};
use toml::{Table, Value};

#[derive(Debug, Parser)] // requires `derive` feature
#[command(name = "img2coe")]
#[command(about = "Image to COE conversion tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Convert {
        /// The image to convert
        image: PathBuf,
        /// The palette to use
        #[arg(short)]
        palette: PathBuf,
    },
    Palette {
        /// The image to extract the palette from
        image: PathBuf,
    },
}

fn parse_color(x: &str) -> Option<Rgba<u8>> {
    if !x.starts_with("#") {
        return None;
    }
    let x = &x[1..];
    if x.len() != 8 {
        return None;
    }
    let r = u8::from_str_radix(&x[0..=1], 16).ok()?;
    let g = u8::from_str_radix(&x[2..=3], 16).ok()?;
    let b = u8::from_str_radix(&x[4..=5], 16).ok()?;
    let a = u8::from_str_radix(&x[6..=7], 16).ok()?;

    Some(Rgba([r, g, b, a]))
}

struct FormattedColor(Rgba<u8>);

impl std::fmt::Display for FormattedColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [r, g, b, a] = self.0.0;
        f.write_fmt(format_args!("#{r:2x}{g:2x}{b:2x}{a:2x}"))
    }
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Palette { image } => {
            let img = ::image::open(&image)?;
            let mut palette = HashSet::new();

            for (_, _, color) in img.pixels() {
                palette.insert(color);
            }

            let mut file = io::BufWriter::new(fs::File::create(image.with_extension("palette.toml"))?);
            let template = include_str!("palette_template.toml")
                .replace("{VERSION}", env!("CARGO_PKG_VERSION"));
            file.write(template.as_bytes())?;

            for (i, color) in palette.into_iter().enumerate() {
let color = FormattedColor(color);
                file.write(format!("\"{color}\" = {i}\n").as_bytes())?;
            }
        }
        Commands::Convert { image, palette } => {
            let mut palette_file = fs::File::open(palette)?;
            let mut palette_str = String::new();
            palette_file.read_to_string(&mut palette_str)?;
            let table = toml::from_str::<Table>(&palette_str)?;
            let Some(palette_map) = table.get("palette").and_then(Value::as_table) else {
                bail!("expected to find 'palette' table on palette file")
            };
            let mut palette = HashMap::new();
            for (key, value) in palette_map {
                let Some(color) = parse_color(key) else {
                    bail!("invalid color: {key}")
                };
                let Some(value) = value.as_integer() else {
                    bail!("value must be integer: {value}")
                };
                palette.insert(color, value);
            }

            let mut coe_file = io::BufWriter::new(fs::File::create(image.with_extension("coe"))?);
            let template = include_str!("coe_template.coe")
                .replace("{VERSION}", env!("CARGO_PKG_VERSION"));
            coe_file.write(template.as_bytes())?;

            let img = ::image::open(&image)?;
            for (_, _, color) in img.pixels() {
                if let Some(&mapping) = palette.get(&color) {
                    coe_file.write(format!("{mapping:x} ").as_bytes())?;
                } else {
                    bail!("could not continue: palette has no mapping for color \"{}\"", FormattedColor(color));
                }
            }

            coe_file.write(";".as_bytes())?;
        }
    }

    Ok(())
}

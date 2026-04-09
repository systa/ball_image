
use anyhow::{Context, Result};
use clap::Parser;
use image::{io::Reader as ImageReader, GenericImageView, ImageBuffer, Rgba, RgbaImage};
use std::f32::consts::PI;
use std::path::PathBuf;

mod scale;

#[derive(Parser, Debug)]
#[command(author, version, about = "Sphere projection from flat image", long_about = None)]
struct Args {
    /// Input image path
    #[arg(short, long)]
    input: PathBuf,

    /// Output image path
    #[arg(short, long)]
    output: PathBuf,

    /// Distortion strength (0=none, 1=normal, >1 stronger)
    #[arg(short, long, default_value = "1.0")]
    strength: f32,

    /// Transparent outside circle (default is black)
    #[arg(long, default_value_t = false)]
    transparent: bool,

    /// Alternative projektion to keep horizontal and vertical lines (default is false)
    #[arg(long, default_value_t = false)]
    keepbox: bool,

    /// Up-scale middle part to avoid artifacts
    #[arg(long, default_value_t = false)]
    expand: bool,
    
    /// Sharpen middle (after up-scaling)
    #[arg(long, default_value_t = false)]
    sharpen: bool,
    
    /// Output width (defaults to input width)
    #[arg(long)]
    width: Option<u32>,

    /// Output height (defaults to input height)
    #[arg(long)]
    height: Option<u32>,

    /// Number of ray-trace workers (default = CPU cores)
    #[arg(long)]
    threads: Option<usize>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if let Some(t) = args.threads {
        rayon::ThreadPoolBuilder::new().num_threads(t).build_global().ok();
    }

    // The folliwing deserves an explanation. The reduce loss-off resolution in the middle of the image. The sample_sphere function is not perfect and the distortion is stronger near the edges. By taking a smaller center part of the image and scaling it up, we can get a better result in the middle of the image. The scale factor is 4, which means that we take a 10% center part of the image and scale it up to 40% of the original size. This way we can get a better result in the middle of the image without losing too much quality. The scaled sub-image is not used in the final output, but it is printed to the console for debugging purposes.
    // used "nearest neighbor" scaling for the final output. The bl_resize function is a bilinear interpolation reducing the effect.
    let img = ImageReader::open(&args.input)
        .with_context(|| format!("Failed to open '{}'", args.input.display()))?
        .decode()
        .with_context(|| format!("Failed to decode '{}'", args.input.display()))?;

    let (in_w, in_h) = img.dimensions();
    let out_w = args.width.unwrap_or(in_w);
    let out_h = args.height.unwrap_or(in_h);
    let src = img.to_rgba8();

    let center_w = ((in_w as f32) * 0.10).max(1.0) as u32;
    let center_h = ((in_h as f32) * 0.10).max(1.0) as u32;
    let left = (in_w - center_w) / 2;
    let top = (in_h - center_h) / 2;

    let sub_image: RgbaImage = ImageBuffer::from_fn(center_w, center_h, |x, y| {
        *src.get_pixel(left + x, top + y)
    });

    let scale_factor = 4u32;
    let sub_image = scale::bl_resize(&sub_image, center_h * scale_factor, center_w * scale_factor);

    
    println!(
        "Extracted center {}x{} region at ({}, {}) and scaled it to {}x{}",
        center_w,
        center_h,
        left,
        top,
        sub_image.width(),
        sub_image.height()
    );
    

    let mut output: RgbaImage = if args.keepbox { ImageBuffer::from_fn(out_w, out_h, |x, y| {
        sample_sphere1(x, y, out_w, out_h, &src, args.strength, args.transparent)
    })} else {
        ImageBuffer::from_fn(out_w, out_h, |x, y| {
            sample_sphere(x, y, out_w, out_h, &src, args.strength, args.transparent, args.expand, left, top, left + center_w, top + center_h, &sub_image)
        })
    };

    if args.sharpen {
    // run simple sharpening to the interpolated center
       sharpen(&mut output, left, top, left + center_w, top + center_h)
    };
    output	     
        .save_with_format(&args.output, image::ImageFormat::from_path(&args.output).with_context(|| {
            format!("Cannot infer output image format from '{}'", args.output.display())
        })?)
        .with_context(|| format!("Failed to save '{}'", args.output.display()))?;

    println!("Saved: {}", args.output.display());
    Ok(())
}


fn sharpen(
 src: &mut RgbaImage,
 x1 : u32,
 y1 : u32,
 x2 : u32,
 y2 : u32
) {
  let copy: RgbaImage = ImageBuffer::from_fn(src.width(), src.height(), | x, y | {*src.get_pixel(x,y)});
  for i in x1+1..x2-1 {
      for j in y1+1..y2-1 {
      	  let n = copy.get_pixel(i, j-1);
      	  let w = copy.get_pixel(i-1, j);
      	  let c = copy.get_pixel(i, j);
      	  let e = copy.get_pixel(i+1, j);
      	  let s = copy.get_pixel(i, j);
      	  let mut new_pix = *src.get_pixel(i,j);
      	  for k in 0..3 {
      	    new_pix[k] = (5*(c[k] as i32) - n[k] as i32 - w[k] as i32 - e[k] as i32 - s[k] as i32).clamp(0,255) as u8;
      	  }
	  (*src).put_pixel(i,j,new_pix);
      }
  }
}

fn sample_sphere(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    src: &RgbaImage,
    strength: f32,
    transparent_outside: bool,
    expand_middle: bool,
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
    sub_image: &RgbaImage,
) -> Rgba<u8> {
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let radius = cx.min(cy);
    let dx = x as f32 - cx;
    let dy = y as f32 - cy;
    let r = (dx * dx + dy * dy).sqrt();
    let scalex = width as f32 / src.width() as f32;
    let scaley = height as f32 / src.height() as f32;

    if r > radius {
        return if transparent_outside {
            Rgba([0, 0, 0, 0])
        } else {
            Rgba([0, 0, 0, 255])
        };
    }
    let sin_t = (r / radius).clamp(0.0, 1.0).powf((strength/3.0).max(0.001));
//    let sin_x = (dx / radius).signum() * (dx / radius).abs().powf(strength.max(0.001));
//    let sin_y = (dy / radius).signum() * (dy / radius).abs().powf(strength.max(0.001));

    let sx = dx*sin_t.asin() / radius;
    let sy = dy*sin_t.asin() / radius;

    let rwidth = radius;// (width as f32 / 2.0);
    let rheight = radius; // (height as f32 / 2.0);
    let sample_x0 = (cx as f32 + sx*rwidth/PI*2.2) / scalex;
    let sample_x = (sample_x0 as u32).clamp(1, src.width() - 2);
    let sample_y0 = (cy as f32 + sy*rheight/PI*2.2) / scaley;
    let sample_y = (sample_y0 as u32).clamp(1, src.height() - 2);
//    if ((x == 2524 && y == 1516) || (x == 3524 && y == 1516) || (x == 2524 && y == 2516) || (x == 3524 && y == 2516)) {
//    if (y == 20160) {

//    if dx.abs() < 9.0 && dy.abs() < 1.0 {
    if y == height / 2 {  
/*        println!(
            "x: {}, y: {}, dx: {:.2}, dy: {:.2}, r: {:.2}, radius: {:.2}, sin_t: {:.4}, sx: {:.4}, sy: {:.4}, sample_x0: {}, sample_x: {}, sample_y0: {}, sample_y: {}",
            x, y, dx, dy, r, radius, sin_t , sx, sy, sample_x0, sample_x, sample_y0, sample_y
*/
/*
        println!(
            "x: {}, y: {}, sample_x0: {}, sample_x: {}, sample_y0: {}, sample_y: {}",
            x, y, sample_x0, sample_x, sample_y0, sample_y
        );
*/
    }



    if expand_middle && sample_x >= left && sample_x < right && sample_y >= top && sample_y < bottom {
        let sub_x = ((sample_x0 - left as f32)*4.0) as u32;
        let sub_y = ((sample_y0  - top as f32)*4.0) as u32;
/*	if sample_x == cx as u32 && sample_y == cy as u32 {
      	    println!("in the middle ({},{}), sub=({},{}) left={}, top={}, right={}, bottom={}", sample_x, sample_y, sub_x, sub_y, left, top, right, bottom);
    	} */
        return *sub_image.get_pixel(sub_x, sub_y);
    }


    *src.get_pixel(sample_x, sample_y)
}

// This keepbox-version is not up to date.
fn sample_sphere1(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    src: &RgbaImage,
    strength: f32,
    transparent_outside: bool,
) -> Rgba<u8> {
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let radius = cx.min(cy);
    let dx = x as f32 - cx;
    let dy = y as f32 - cy;
    let r = (dx * dx + dy * dy).sqrt();

    if r > radius {
        return if transparent_outside {
            Rgba([0, 0, 0, 0])
        } else {
            Rgba([0, 0, 0, 255])
        };
    }

    let sin_x = (dx / radius).signum() * (dx / radius).abs().powf(strength.max(0.001));
    let sin_y = (dy / radius).signum() * (dy / radius).abs().powf(strength.max(0.001));

    let sx = sin_x.asin();
    let sy = sin_y.asin();

    let rwidth = radius;// (width as f32 / 2.0);
    let rheight = radius; // (height as f32 / 2.0);
    let sample_x0 = (cx as f32 + sx*rwidth/PI*2.2) as u32;
    let sample_x = sample_x0.clamp(1, src.width() - 2);
    let sample_y0 = ((cy as f32 + sy*rheight/PI*2.2) as u32 ).clamp(1, src.height() - 2);
    let sample_y = sample_y0.clamp(1, src.height() - 2);
//    if (x == 2524 && y == 1516) || (x == 3524 && y == 1516) || (x == 2524 && y == 2516) || (x == 3524 && y == 2516) {
/*
    if y == 20160 {
        println!(
            "x: {}, y: {}, dx: {:.2}, dy: {:.2}, r: {:.2}, radius: {:.2}, sin_x: {:.4}, sin_y: {:.4},  sx: {:.4}, sy: {:.4}, sample_x0: {}, sample_x: {}, sample_y0: {}, sample_y: {}",
            x, y, dx, dy, r, radius, sin_x, sin_y, sx, sy, sample_x0, sample_x, sample_y0, sample_y
        );
    }
*/
    *src.get_pixel(sample_x, sample_y)
}
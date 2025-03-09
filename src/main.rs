use std::fs;
use std::io::ErrorKind;
use std::time::Duration;
use imageproc::contrast::otsu_level;
use imageproc::contours::{find_contours_with_threshold};

use image::{DynamicImage, GenericImageView, ImageFormat};
use pdfium_render::prelude::*;
use clap::Parser;
use imageproc::rect::Rect;
use indicatif::{ProgressBar, ProgressStyle};


#[derive(Parser)]
struct Args {
    #[arg(long, default_value_t=false, help="Save cropped images")]
    save_cropped: bool,
    #[arg(long, default_value_t=String::from("./out"), help="Path to save cropped images")]
    cropped_path: String,
    #[arg(long, default_value_t=String::from("sample.pdf"), help="Path to target PDF file")]
    target: String,
    #[arg(long, default_value_t=String::from("output.pdf"), help="Path to output PDF file")]
    output: String,
    #[arg(long, default_value_t=150, help="Gray threshold to detect margin")]
    gray_threshold: u8,
    #[arg(long, default_value_t=20, help="Margin remain percent")]
    margin_remain_percent: u32
}


// https://github.com/bblanchon/pdfium-binaries?tab=readme-ov-file
// https://github.com/ajrcarey/pdfium-render



fn main() -> Result<(), PdfiumError>{

    let args = Args::parse();
    
    let is_make_cropped_image = args.save_cropped;
    let cropped_images_path: &str = &args.cropped_path;

    let target_pdf_path = &args.target;
    let output_pdf_path = &args.output;

    let gray_threshold = args.gray_threshold;
    let margin_reamin_percent = args.margin_remain_percent;

    let pdfium = Pdfium::default();

    let document = pdfium.load_pdf_from_file(target_pdf_path, None).expect("Failed to load PDF file");

    println!("[1/3] PDF loaded: {} ({} pages)", target_pdf_path, document.pages().len());

    let mut new_document = pdfium.create_new_pdf()?;


    let pb = ProgressBar::new(document.pages().len() as u64);

    println!("[2/3] Start cropping pages");
    for (i, page) in document.pages().iter().enumerate() {
        let page_num: usize = i + 1;
        let page_size = page.page_size();
        let origin_image = page.render(page_size.width().value as i32, page_size.height().value as i32, Option::<PdfPageRenderRotation>::None).expect("Failed to render page")
            .as_image();

        // let new_image = crop_image(&origin_image, gray_threshold, margin_reamin_percent);
        let new_image = crop_image_with_contour(&origin_image, margin_reamin_percent);
        // println!("\rPage {} cropped", page_num);

        let (new_width, new_height) = new_image.dimensions();
        if new_width == 0 || new_height == 0 {
            pb.inc(1);
            continue;
        }

        if is_make_cropped_image {

            if !is_diretory_exist(cropped_images_path) {
                fs::create_dir_all(cropped_images_path).expect("Failed to create directory");
            }

            new_image.clone().into_rgb8().save_with_format(format!("{}/page_{}.png", cropped_images_path, page_num), ImageFormat::Png).expect("Error occured during save cropped image.");
        }

        let new_size = PdfPagePaperSize::Custom(PdfPoints::new(new_width as f32), PdfPoints::new(new_height as f32));

        let mut new_page = new_document.pages_mut().create_page_at_end(new_size)?;
        new_page.objects_mut().create_image_object(
            PdfPoints::new(0.0), 
            PdfPoints::new(0.0), 
            &new_image, 
            Some(PdfPoints::new(new_width as f32)), 
            Some(PdfPoints::new(new_height as f32)))?;
        // println!("\rNew page {} saved", page_num);
        pb.inc(1);
    }

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::default_spinner().template(&format!("{{spinner:.green}} [3/3] Start making PDF to {output_pdf_path}")).unwrap());
    spinner.enable_steady_tick(Duration::from_millis(10));
    // println!("[3/3] Start making PDF to {}", output_pdf_path);
    new_document.save_to_file(output_pdf_path)?;    
    spinner.finish();
    println!("Pdf created to: {output_pdf_path}");

    Ok(())
}

fn crop_image(img: &DynamicImage, gray_threashold: u8, margin_reamin_percent: u32) -> DynamicImage {
    let (width, height) = img.dimensions();

    let mut min_x = 0;
    let mut max_x = width;
    let mut min_y = 0;
    let mut max_y = height;

    // find Top Margin
    for y in 0..height {
        let mut is_non_marginal_pixel = false;
        
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            let gray_value = ((pixel.0[0] as f32 + pixel.0[1] as f32 + pixel.0[2] as f32) / 3.0) as u8;
            if pixel.0[3] > 0 && gray_value < gray_threashold {
                // println!("gray_value: {} - min_y: {}", gray_value, y);
                is_non_marginal_pixel = true;
                break;
            }
        }

        if is_non_marginal_pixel
        {
            min_y = y;
            break;
        }
    }    

    // find Bottom Margin
    for y in (0..height).rev() {
        let mut is_non_marginal_pixel = false;
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            let gray_value = ((pixel.0[0] as f32 + pixel.0[1] as f32 + pixel.0[2] as f32) / 3.0) as u8;
            if pixel.0[3] > 0 && gray_value < gray_threashold {
                is_non_marginal_pixel = true;
                break;
            }
        }

        if is_non_marginal_pixel
        {
            max_y = y;
            break;
        }
    }

    // find Left Margin
    for x in 0..width {
        let mut is_non_marginal_pixel = false;
        for y in 0..height {
            let pixel = img.get_pixel(x, y);
            let gray_value = ((pixel[0] as f32 + pixel[1] as f32 + pixel[2] as f32) / 3.0) as u8;
            if pixel[3] > 0 && gray_value < gray_threashold {
                is_non_marginal_pixel = true;
                break;
            }
        }

        if is_non_marginal_pixel
        {
            min_x = x;
            break;
        }
    }

    for x in (0..width).rev() {
        let mut is_non_marginal_pixel = false;
        for y in 0..height {
            let pixel = img.get_pixel(x, y);
            let gray_value = ((pixel[0] as f32 + pixel[1] as f32 + pixel[2] as f32) / 3.0) as u8;
            if pixel[3] > 0 && gray_value < gray_threashold {
                is_non_marginal_pixel = true;
                break;
            }
        }

        if is_non_marginal_pixel
        {
            max_x = x;
            break;
        }
    }
        
    let add_margin_x = (min_x * margin_reamin_percent / 100) as u32;
    let add_margin_y = (min_y * margin_reamin_percent / 100) as u32;



    // println!("min_x: {}, max_x: {}, min_y: {}, max_y: {}", min_x, max_x, min_y, max_y);
    img.crop_imm(min_x - add_margin_x, min_y - add_margin_y, max_x - min_x + 2 * add_margin_x, max_y - min_y + 2 * add_margin_y)
}

fn crop_image_with_contour(img: &DynamicImage, margin_remain_percent: u32) -> DynamicImage {
    let text_rect = get_text_region(img.clone(), margin_remain_percent).unwrap();
    img.crop_imm(text_rect.left() as u32, text_rect.top() as u32, text_rect.width(), text_rect.height())
}

fn get_text_region(img: DynamicImage, margin_remain_percent: u32) -> Option<Rect>{
    let (width, height) = img.dimensions();
    let gray_image = img.into_luma8();
    let threshold_value = otsu_level(&gray_image);
    let contours = find_contours_with_threshold::<u32>(&gray_image, threshold_value);
    
    let mut min_x = gray_image.width();
    let mut min_y = gray_image.height();
    let mut max_x = 0;
    let mut max_y = 0;
    
    for contour in contours {
        for pt in contour.points.iter() {
            if pt.x < min_x { min_x = pt.x; }
            if pt.y < min_y { min_y = pt.y; }
            if pt.x > max_x { max_x = pt.x; }
            if pt.y > max_y { max_y = pt.y; }
        }
    }

    let res_width = max_x - min_x;
    let res_height = max_y - min_y;

    let width_margin = width - res_width;
    let height_margin = height - res_height;

    let width_offset = width_margin * margin_remain_percent  / 200;
    let height_offset = height_margin * margin_remain_percent / 200;

    min_x = (min_x - width_offset).clamp(0, min_x);
    max_x = (max_x + width_offset).clamp(max_x, width);

    min_y = (min_y - height_offset).clamp(0, min_y);
    max_y = (max_y + height_offset).clamp(max_y, height);

    if max_x > min_x && max_y > min_y {
        Some(Rect::at(min_x as i32, min_y as i32).of_size((max_x - min_x) as u32, (max_y - min_y) as u32))
    } else { 
        None
    }
}


fn is_diretory_exist(path: &str) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => metadata.is_dir(),
        Err(e) => {
            if e.kind() == ErrorKind::PermissionDenied {
                panic!("Permission denied to access path: {}", path);
            } else if e.kind() == ErrorKind::NotFound {
                // panic!("Path not found: {}", path);
                false
            } else {
                panic!("Error occured during access path: {}", path);
            }
        }
    }
    
}
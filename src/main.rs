use image::{ImageFormat, DynamicImage, GenericImageView};
use pdfium_render::prelude::*;

// https://github.com/bblanchon/pdfium-binaries?tab=readme-ov-file
// https://github.com/ajrcarey/pdfium-render

fn main() -> Result<(), PdfiumError>{

    let is_make_cropped_image = true;
    let cropped_images_path: &str = "./out";

    let target_pdf_path = "sample.pdf";
    let output_pdf_path = "output.pdf";


    let pdfium = Pdfium::default();

    let document = pdfium.load_pdf_from_file(target_pdf_path, None).expect("Failed to load PDF file");
    let mut new_document = pdfium.create_new_pdf()?;

    for (i, page) in document.pages().iter().enumerate() {
        let page_num: usize = i + 1;
        let page_size = page.page_size();
        let origin_image = page.render(page_size.width().value as i32, page_size.height().value as i32, Option::<PdfPageRenderRotation>::None).expect("Failed to render page")
            .as_image();

        let new_image = crop_image(&origin_image, 150, 10);
        println!("Page {} cropped", page_num);
        if is_make_cropped_image {
            new_image.clone().into_rgb8().save_with_format(format!("{}/page_{}.png", cropped_images_path, page_num), ImageFormat::Png).expect("Error occured during save cropped image.");
        }

        let (new_width, new_height) = new_image.dimensions();
        let new_size = PdfPagePaperSize::Custom(PdfPoints::new(new_width as f32), PdfPoints::new(new_height as f32));

        let mut new_page = new_document.pages_mut().create_page_at_end(new_size)?;
        new_page.objects_mut().create_image_object(
            PdfPoints::new(0.0), 
            PdfPoints::new(0.0), 
            &new_image, 
            Some(PdfPoints::new(new_width as f32)), 
            Some(PdfPoints::new(new_height as f32)))?;
        println!("New page {} saved", page_num);
    }

    println!("Start making PDF to {}", output_pdf_path);
    new_document.save_to_file(output_pdf_path)?;
    println!("Pdf created to: {}", output_pdf_path);

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



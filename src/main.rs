use std::path::Path;
use image::{ImageFormat, DynamicImage, GenericImageView, RgbaImage};
use pdfium_render::prelude::*;

// https://github.com/bblanchon/pdfium-binaries?tab=readme-ov-file
// https://github.com/ajrcarey/pdfium-render

fn main() -> Result<(), PdfiumError>{
    let pdfium = Pdfium::default();

    let document = pdfium.load_pdf_from_file("sample.pdf", None).expect("Failed to load PDF file");
    let mut new_document = pdfium.create_new_pdf()?;




    let mut new_images: Vec<DynamicImage> = vec![];

    for (i, page) in document.pages().iter().enumerate() {
        let page_size = page.page_size();
        
        let origin_image = page.render(page_size.width().value as i32, page_size.height().value as i32, Option::<PdfPageRenderRotation>::None).expect("Failed to render page")
            .as_image();
            // .into_rgb8()
            // .save_with_format(format!("./out/page_{}.png", i), ImageFormat::Png);
        let new_image = crop_image(&origin_image, 150, 10);
        // new_image.into_rgb8().save_with_format(format!("./out/page_{}.png", i), ImageFormat::Png);

        let (new_width, new_height) = new_image.dimensions();
        let new_size = PdfPagePaperSize::Custom(PdfPoints::new(new_width as f32), PdfPoints::new(new_height as f32));

        let mut new_page = new_document.pages_mut().create_page_at_end(new_size)?;
        new_page.objects_mut().create_image_object(
            PdfPoints::new(0.0), 
            PdfPoints::new(0.0), 
            &new_image, 
            Some(PdfPoints::new(new_width as f32)), 
            Some(PdfPoints::new(new_height as f32)));

        new_images.push(new_image);
        
        println!("Page {} cropped", i);
    }

    println!("Start making PDF");


    // for (i, image) in new_images.iter().enumerate() {
    //     println!("Processing image {}...", i);
    //     let (width, height) = image.dimensions();
    //     println!("Image dimensions: width = {}, height = {}", width, height);
    //     // let rgba_image = image.to_rgba8();
    //     // let rgba_buffer = rgba_image.into_raw();
    //     println!("Creating new page...");
    //     let mut page = new_document.pages_mut().create_page_at_end(PdfPagePaperSize::Custom(PdfPoints::new(width as f32), PdfPoints::new(height as f32)))?;
        
    //     println!("Creating image object...");
    //     let image_clone = image.clone();
    //     if let Some(image_object) = page.objects_mut().create_image_object(
    //             PdfPoints::new(0.0), 
    //             PdfPoints::new(0.0), 
    //             &image_clone, 
    //             Some(PdfPoints::new(width as f32)), 
    //             Some(PdfPoints::new(height as f32))).ok() {
    //         println!("Adding image object to page...");
    //         page.objects_mut().add_object(image_object);
    //         println!("Page {} processed", i);
    //     } else {
    //         println!("Failed to create image object for page {}", i);
    //     }
    // }
    
    let output_pdf_path = "output.pdf";
    println!("Saving PDF to file: {}", output_pdf_path);
    new_document.save_to_file(output_pdf_path)?;
    println!("PDF saved successfully.");


    // dynamic_images_to_pdf(new_pages, "output.pdf");



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


fn dynamic_images_to_pdf(images: Vec<DynamicImage>, output_pdf_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing Pdfium...");
    // let pdfium = Pdfium::new(Pdfium::bind_to_library(Path::new("./libpdfium.dylib"))?);
    let pdfium = Pdfium::default();
    println!("Creating new PDF document...");
    let mut document = pdfium.create_new_pdf()?;

    for (i, image) in images.iter().enumerate() {
        println!("Processing image {}...", i);
        let (width, height) = image.dimensions();
        println!("Image dimensions: width = {}, height = {}", width, height);
        let rgba_image = image.to_rgba8();
        let rgba_buffer = rgba_image.into_raw();
        println!("Creating new page...");
        let mut page = document.pages_mut().create_page_at_end(PdfPagePaperSize::Custom(PdfPoints::new(width as f32), PdfPoints::new(height as f32)))?;
        println!("Creating image object...");
        let image_object = page.objects_mut().create_image_object(PdfPoints::new(0 as f32), PdfPoints::new(0 as f32), &image, Some(PdfPoints::new(width as f32)), Some(PdfPoints::new(height as f32)))?;

        println!("Adding image object to page...");
        page.objects_mut().add_object(image_object);
        println!("Page {} processed", i);
    }
    
    println!("Saving PDF to file: {}", output_pdf_path);
    document.save_to_file(output_pdf_path)?;
    println!("PDF saved successfully.");
    Ok(())
}
